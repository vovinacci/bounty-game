//! The hero's spells of the map under the reference rules (`docs/spec/07-RULES-REGISTER.md`, U-13): cast from his
//! book between battles, each for a charge, none taking any time.

use super::{Action, Aim, ArmyStack, Event, Expedition, Refused};
use crate::content::{Cell, MapSpell};

/// Leadership a point of spell power adds until the week ends.
const RAISE_CONTROL: u32 = 100;
/// Parts of a week's walking a point of spell power stops the clock for: one of this many, as ten steps are of
/// the reference's week of five days of forty.
const TIME_STOP_PARTS: u32 = 20;
/// What Instant Army raises for a point of spell power and one more, by the hero's rank.
const INSTANT_ARMY: [u32; 4] = [3, 2, 1, 1];
/// The terrain a bridge is laid over.
const WATER: &str = "water";

impl Expedition {
    /// Ground the hero can walk: the map's, and the water he has bridged.
    #[must_use]
    pub fn passable(&self, cell: Cell) -> bool {
        self.def.map.passable(cell) || self.bridges().contains(&cell)
    }

    /// The cells of water the hero has bridged, in the order he did.
    #[must_use]
    pub fn bridges(&self) -> &[Cell] {
        self.magic.as_ref().map_or(&[], |magic| &magic.bridges)
    }

    /// The steps the hero can still take while time stands.
    #[must_use]
    pub fn free_steps(&self) -> u32 {
        self.magic.as_ref().map_or(0, |magic| magic.free_steps)
    }

    /// The places a gate would carry the hero to now: the town for the one spell, his own castles for the
    /// other, never where he stands.
    #[must_use]
    pub fn gates(&self, spell: MapSpell) -> Vec<&str> {
        let def = &self.def;
        let away = |cell: Cell| cell != self.hero;
        match spell {
            MapSpell::TownGate => {
                let town = def.map.object_at(def.town).filter(|_| away(def.town));
                town.map(|object| object.id.as_str()).into_iter().collect()
            }
            MapSpell::CastleGate => {
                let own = def.castles.iter().filter(|castle| {
                    let cell = def
                        .map
                        .object(&castle.object_id)
                        .map(crate::campaign::MapObject::cell);
                    self.is_taken(&castle.object_id) && cell.is_some_and(away)
                });
                own.map(|castle| castle.object_id.as_str()).collect()
            }
            _ => Vec::new(),
        }
    }

    /// The cells a bridge laid from the hero in this direction would cover: the first cell of water, and the
    /// next if it is water too; none where he does not face water.
    #[must_use]
    pub fn bridge_towards(&self, dx: i32, dy: i32) -> Vec<Cell> {
        let water = |cell: &Cell| {
            let terrain = self.def.map.terrain(*cell);
            terrain.is_some_and(|terrain| terrain.id == WATER) && !self.bridges().contains(cell)
        };
        let at = |steps: i32| {
            let x = self.hero.x.checked_add_signed(dx * steps);
            let y = self.hero.y.checked_add_signed(dy * steps);
            x.zip(y).map(|(x, y)| Cell { x, y })
        };
        if dx.abs() + dy.abs() != 1 {
            return Vec::new();
        }
        let cells = [at(1), at(2)].into_iter().flatten();
        cells.take_while(water).collect()
    }

    /// What Instant Army would raise now: the troop of the hero's rank, and a number that grows with his spell
    /// power and shrinks with his rank.
    #[must_use]
    pub fn instant_army(&self) -> Option<(&str, u32)> {
        let (rank, standing) = (self.rank()?, self.standing.as_ref()?);
        let by_rank = INSTANT_ARMY.get(standing.rank).copied().unwrap_or(1);
        let count = self.spell_power().saturating_add(1).saturating_mul(by_rank);
        Some((rank.instant_army.as_str(), count.max(1)))
    }

    /// A spell of the map. Everything is checked before the charge is spent, so a spell that cannot be cast
    /// costs nothing.
    pub(super) fn cast_on_map(
        &mut self,
        spell_id: &str,
        aim: &Aim,
        events: &mut Vec<Event>,
    ) -> Result<(), Refused> {
        let no_charge = || Refused::NoSpellCharge {
            spell: spell_id.to_owned(),
        };
        let book = &self.def.rules.book;
        let place = book.iter().position(|spell| spell.id == spell_id);
        let place = place.ok_or_else(no_charge)?;
        let Some(magic) = self.magic.as_ref().filter(|magic| magic.knows) else {
            return Err(Refused::CannotCast);
        };
        if magic.charges[place] == 0 {
            return Err(no_charge());
        }
        let spell = book[place].map.ok_or(Refused::NotCastOnTheMap)?;
        let power = self.spell_power().max(1);
        let done = match (spell, aim) {
            (MapSpell::TimeStop, Aim::None) => {
                let clock = &self.def.clock;
                let a_week = clock.week_ticks / clock.movement_ticks.max(1);
                let steps = power.saturating_mul(a_week) / TIME_STOP_PARTS;
                Event::TimeStopped {
                    steps: steps.max(1),
                }
            }
            (MapSpell::RaiseControl, Aim::None) => Event::ControlRaised {
                leadership: power.saturating_mul(RAISE_CONTROL),
            },
            (MapSpell::InstantArmy, Aim::None) => self.raise_army()?,
            (MapSpell::FindVillain, Aim::None) => return Err(Refused::NothingToFind),
            (MapSpell::Bridge, Aim::Towards { dx, dy }) => {
                let cells = self.bridge_towards(*dx, *dy);
                if cells.is_empty() {
                    return Err(Refused::NoWaterToBridge);
                }
                Event::Bridged { cells }
            }
            (MapSpell::TownGate | MapSpell::CastleGate, Aim::Place(id)) => {
                let open = self.gates(spell).contains(&id.as_str());
                let to = self.def.map.object(id).filter(|_| open);
                let to = to.ok_or_else(|| Refused::NoGateTo { place: id.clone() })?;
                Event::Gated {
                    from: self.hero,
                    to: to.cell(),
                }
            }
            _ => return Err(Refused::WrongAim),
        };
        events.push(Event::MapSpellCast {
            spell: spell_id.to_owned(),
        });
        if let Some(magic) = &mut self.magic {
            magic.charges[place] -= 1;
            match &done {
                Event::TimeStopped { steps } => {
                    magic.free_steps = magic.free_steps.saturating_add(*steps);
                }
                Event::ControlRaised { leadership } => {
                    magic.raised = magic.raised.saturating_add(*leadership);
                }
                Event::Bridged { cells } => magic.bridges.extend(cells),
                Event::Gated { to, .. } => self.hero = *to,
                _ => {}
            }
        }
        events.push(done);
        Ok(())
    }

    /// Instant Army: the creatures join a stack of their troop or take a free place of the army, for nothing and
    /// whatever the hero's leadership; with no place for them the spell is not cast.
    fn raise_army(&mut self) -> Result<Event, Refused> {
        let (troop_id, count) = self.instant_army().ok_or(Refused::CannotCast)?;
        let troop_id = troop_id.to_owned();
        let slots = self.def.rules.board.max_stacks_per_side as usize;
        if let Some(stack) = self.army.iter_mut().find(|s| s.troop_id == troop_id) {
            stack.count = stack
                .count
                .checked_add(count)
                .ok_or(Refused::StackTooLarge)?;
        } else if self.army.len() < slots {
            let free = (0..=self.army.len())
                .map(|n| format!("p{n}"))
                .find(|id| self.army.iter().all(|s| s.id != *id));
            self.army.push(ArmyStack {
                id: free.unwrap_or_default(),
                troop_id: troop_id.clone(),
                count,
            });
        } else {
            return Err(Refused::ArmyFull);
        }
        Ok(Event::ArmyRaised { troop_id, count })
    }

    /// The casts open to the hero on the map now, by spell in the book's order: each with every aim it can take.
    #[must_use]
    pub fn casts_on_map(&self) -> Vec<Action> {
        let Some(magic) = self.magic.as_ref().filter(|magic| magic.knows) else {
            return Vec::new();
        };
        let mut casts = Vec::new();
        for (spell, _) in self
            .def
            .rules
            .book
            .iter()
            .zip(&magic.charges)
            .filter(|(_, n)| **n > 0)
        {
            let aims: Vec<Aim> = match spell.map {
                Some(MapSpell::Bridge) => [(0, -1), (1, 0), (0, 1), (-1, 0)]
                    .into_iter()
                    .map(|(dx, dy)| Aim::Towards { dx, dy })
                    .collect(),
                Some(gate @ (MapSpell::TownGate | MapSpell::CastleGate)) => {
                    let places = self.gates(gate).into_iter();
                    places.map(|id| Aim::Place(id.to_owned())).collect()
                }
                Some(_) => vec![Aim::None],
                None => Vec::new(),
            };
            let cast = |aim| Action::CastOnMap {
                spell: spell.id.clone(),
                aim,
            };
            let open = |action: &Action| self.refusal(action).is_none();
            casts.extend(aims.into_iter().map(cast).filter(open));
        }
        casts
    }
}
