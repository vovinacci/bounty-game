//! The expedition pilot `errand-v1`: a plain player of a whole expedition, for measuring what a rule costs over a
//! game. It recruits what it can keep, takes the contract that leads to the clue, walks to the bands in the
//! order the expedition lists them, fights with [`approach_v1`], and searches where the clue points. It is not
//! clever and is not meant to be: it is the same before and after a change.
//!
//! It is handed the expedition but uses only what the player's screen shows: where the objective lies is read
//! from the clue (`objective_known`), never from the definition.

use std::collections::VecDeque;

use bounty_core::campaign::{Action, Expedition, Holds, MapObject, ObjectKind, Status};
use bounty_core::content::Cell;

use crate::approach_v1;

pub const ERRAND_V1: &str = "errand-v1";
pub const ERRAND_V2: &str = "errand-v2";
pub const ERRAND_V3: &str = "errand-v3";
pub const ERRAND_V4: &str = "errand-v4";
pub const ERRAND_V5: &str = "errand-v5";

/// The first step of a shortest way from the hero to `to` over ground that can be walked, round every band that
/// still stands except one standing on `to` itself, and how many steps the way has. `None` when there is no way,
/// or the hero is there.
fn way_to(expedition: &Expedition, to: Cell) -> Option<((i32, i32), u32)> {
    let map = &expedition.def().map;
    let from = expedition.hero();
    // A band that stands bars the way, and so does a castle not yet the hero's: no pilot walks through one.
    let barred = |cell: Cell| {
        let bars = |o: &MapObject| match o.kind {
            ObjectKind::Encounter => !expedition.is_cleared(&o.id),
            ObjectKind::Castle => {
                expedition.def().castle(&o.id).is_some() && !expedition.is_taken(&o.id)
            }
            _ => false,
        };
        cell != to && map.object_at(cell).is_some_and(bars)
    };
    let index = |cell: Cell| (cell.y * map.width + cell.x) as usize;
    // Searched from the goal, so that each cell knows its distance to it; the hero then takes the first of his
    // four neighbours, in a fixed order, that is nearest.
    let mut distance = vec![u32::MAX; (map.width * map.height) as usize];
    distance[index(to)] = 0;
    let mut queue = VecDeque::from([to]);
    let steps = [(0, -1), (1, 0), (0, 1), (-1, 0)];
    let neighbour = |cell: Cell, (dx, dy): (i32, i32)| {
        let (x, y) = (
            cell.x.checked_add_signed(dx)?,
            cell.y.checked_add_signed(dy)?,
        );
        Some(Cell { x, y }).filter(|&next| map.passable(next) && !barred(next))
    };
    while let Some(cell) = queue.pop_front() {
        for next in steps.iter().filter_map(|&step| neighbour(cell, step)) {
            if distance[index(next)] == u32::MAX {
                distance[index(next)] = distance[index(cell)] + 1;
                queue.push_back(next);
            }
        }
    }
    let here = distance[index(from)];
    let nearer = |step: &&(i32, i32)| {
        neighbour(from, **step).is_some_and(|next| distance[index(next)] < here)
    };
    steps.iter().find(nearer).map(|step| (*step, here))
}

fn step_towards(expedition: &Expedition, to: Cell) -> Option<(i32, i32)> {
    way_to(expedition, to).map(|(step, _)| step)
}

/// What to buy now, if anything: as many as can be had of the dearest troop the town still sells, the purse can
/// pay and the leadership allows, keeping back the upkeep the enlarged army will cost at the week's end.
fn purchase(expedition: &Expedition) -> Option<Action> {
    let def = expedition.def();
    let mut wares: Vec<(&String, &u64)> = def.prices.iter().collect();
    wares.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for (troop, _) in wares {
        let mut quantity = expedition.stock(troop);
        while quantity > 0 {
            let buy = Action::Recruit {
                troop_id: troop.clone(),
                quantity,
            };
            let mut after = expedition.clone();
            let command = bounty_core::campaign::Command {
                revision: expedition.revision(),
                action: buy.clone(),
            };
            if after.apply(&command).is_ok() && after.gold() >= after.upkeep() {
                return Some(buy);
            }
            quantity /= 2;
        }
    }
    None
}

/// What to buy at the place on `cell`, were the hero there: as `purchase` chooses in the town, among what that
/// place sells him. `None` when the place sells nothing he can pay for, lead and keep.
fn purchase_at(expedition: &Expedition, cell: Cell) -> Option<Action> {
    let mut wares = expedition.on_sale_at(cell);
    wares.sort_by(|a, b| b.price.cmp(&a.price).then(a.troop_id.cmp(&b.troop_id)));
    for ware in wares {
        // Where the place does not run out, no more than the leadership could ever hold.
        let most = expedition.leadership();
        let mut quantity = ware.left.unwrap_or(most).min(most);
        while quantity > 0 {
            let bought = expedition.after_buying_at(cell, &ware.troop_id, quantity);
            if bought.is_ok_and(|after| after.gold() >= after.upkeep()) {
                return Some(Action::Recruit {
                    troop_id: ware.troop_id,
                    quantity,
                });
            }
            quantity /= 2;
        }
    }
    None
}

/// The pilot `errand-v3`: `errand-v2` that recruits wherever troops are sold. Before it goes to a band it calls
/// at the nearest place that has something it can pay for, lead and keep (the town, a dwelling, the king's
/// castle) and that is worth the walk, and buys there as `errand-v1` buys in the town; when no place has, it
/// goes to fight.
///
/// It knows what every place sells without having been there, which a player learns by walking; it is a
/// measuring stick, not a player.
#[must_use]
pub fn errand_v3(expedition: &Expedition) -> Option<Action> {
    muster(expedition, false, false)
}

/// The pilot `errand-v4`: `errand-v3` that opens chests. A closed chest is a place to call at like one that
/// sells, whatever it holds; its gold the pilot shares out for leadership, every time, and a commission it
/// takes.
#[must_use]
pub fn errand_v4(expedition: &Expedition) -> Option<Action> {
    muster(expedition, true, false)
}

/// The pilot `errand-v5`: `errand-v4` that besieges. Once the bands are beaten it takes contracts on the lords
/// at large, buys siege weapons in the town when it can keep its army afterwards, and takes their castles; in a
/// castle just taken it leaves one creature, so that the castle is not lost at the week's end. Castles of
/// monsters it leaves alone: they pay nothing.
#[must_use]
pub fn errand_v5(expedition: &Expedition) -> Option<Action> {
    if let Ok(action) = off_the_map(expedition) {
        return action;
    }
    // One creature of the largest stack stays in a castle that has nobody in it.
    if let Some(castle) = expedition.own_castle_here()
        && expedition.garrison(castle).is_empty()
        && let Some(most) = expedition.army().iter().max_by_key(|stack| stack.count)
    {
        let leave = Action::Garrison {
            troop_id: most.troop_id.clone(),
            quantity: 1,
        };
        if expedition.refusal(&leave).is_none() {
            return Some(leave);
        }
    }
    let def = expedition.def();
    let wanted = expedition.contracts_on_offer().first().copied();
    let lord_next = wanted.is_some_and(|id| def.castle(id).is_some());
    if lord_next && !expedition.has_siege_weapons() {
        // To the town for them, and there until the purse can pay for them and still keep the army.
        if expedition.hero() != def.town {
            let step = step_towards(expedition, def.town);
            return Some(step.map_or(Action::WaitTick, |(dx, dy)| Action::Move { dx, dy }));
        }
        let kept = |cost: u64| expedition.gold() >= cost.saturating_add(expedition.upkeep());
        let can = expedition.refusal(&Action::BuySiegeWeapons).is_none();
        if can && def.siege_weapons_cost.is_some_and(kept) {
            return Some(Action::BuySiegeWeapons);
        }
        return Some(purchase_at(expedition, def.town).unwrap_or(Action::WaitTick));
    }
    muster(expedition, true, true)
}

/// `errand-v3`; with `chests` `errand-v4`; with `lords` as well, what `errand-v5` does once it can besiege:
/// its contracts are then on the lords at large too, after the bands.
fn muster(expedition: &Expedition, chests: bool, lords: bool) -> Option<Action> {
    if let Ok(action) = off_the_map(expedition) {
        return action;
    }
    if let Some(chest) = expedition.chest_here().filter(|_| chests) {
        return Some(match chest.holds {
            Holds::Gold(_) => Action::ShareChest,
            Holds::Commission(_) => Action::KeepChest,
        });
    }
    let def = expedition.def();
    let here = expedition.hero();
    let walk = |to: Cell| step_towards(expedition, to).map(|(dx, dy)| Action::Move { dx, dy });
    if expedition.refusal(&Action::Promote).is_none() {
        return Some(Action::Promote);
    }
    if let Some(buy) = purchase_at(expedition, here) {
        return Some(buy);
    }
    let on_offer = expedition.contracts_on_offer();
    let band = |id: &&str| lords || def.encounter(id).is_some();
    let next = on_offer.into_iter().find(band);
    if let Some(id) = next.filter(|_| here == def.town && expedition.contract().is_none()) {
        let take = Action::TakeContract { id: id.to_owned() };
        if expedition.refusal(&take).is_none() {
            return Some(take);
        }
    }
    let villains = expedition.standing().map_or(0, |s| s.villains);
    let earned = expedition
        .next_rank()
        .is_some_and(|rank| rank.villains_needed <= villains);
    if let Some(king) = def.king.as_ref().filter(|_| earned) {
        return Some(walk(king.castle).unwrap_or(Action::WaitTick));
    }
    if let Some(place) = expedition.objective_known() {
        if here == place {
            return Some(Action::Search);
        }
        return Some(walk(place).unwrap_or(Action::WaitTick));
    }
    let Some(band) = expedition.contract().and_then(|id| def.map.object(id)) else {
        // No contract: to the town for one, while a band still stands.
        let to_town = next.and_then(|_| walk(def.town));
        return Some(to_town.unwrap_or(Action::WaitTick));
    };
    // The places that sell: the nearest with something to buy is called at first.
    let castle = def.king.as_ref().map(|king| king.castle);
    let dwellings = def
        .dwellings
        .iter()
        .filter_map(|d| def.map.object(&d.object_id));
    let places = castle
        .into_iter()
        .chain(dwellings.map(MapObject::cell))
        .chain([def.town]);
    // Worth the walk is a purchase that adds a tenth to the army's health or more; for less the pilot would go
    // from place to place on every week's income and never fight.
    let health = |troop: &str, count: u32| {
        let each = def.rules.troop(troop).map_or(0, |troop| troop.hp);
        u64::from(each) * u64::from(count)
    };
    let army: u64 = expedition
        .army()
        .iter()
        .map(|stack| health(&stack.troop_id, stack.count))
        .sum();
    let worth = places.filter(|&place| match purchase_at(expedition, place) {
        Some(Action::Recruit { troop_id, quantity }) => health(&troop_id, quantity) * 10 >= army,
        _ => false,
    });
    let closed = def
        .chests
        .iter()
        .filter(|chest| chests && !expedition.is_opened(&chest.object_id));
    let closed = closed.filter_map(|chest| def.map.object(&chest.object_id));
    let nearest = worth
        .chain(closed.map(MapObject::cell))
        .filter_map(|place| way_to(expedition, place))
        .min_by_key(|(_, steps)| *steps);
    let step = nearest.map(|(step, _)| step);
    let step = step.or_else(|| step_towards(expedition, band.cell()));
    Some(step.map_or(Action::WaitTick, |(dx, dy)| Action::Move { dx, dy }))
}

/// What both pilots do off the map: fight a battle with [`approach_v1`], and nothing once the expedition is over.
/// `Err` when the hero stands on the map and the pilot has to choose.
fn off_the_map(expedition: &Expedition) -> Result<Option<Action>, ()> {
    match expedition.status() {
        Status::Adventure => Err(()),
        Status::Battle => {
            let order = expedition
                .battle()
                .and_then(|battle| approach_v1(&battle.observe()));
            // A battle the pilot has no command for is given up rather than left standing.
            Ok(Some(order.map_or(Action::Surrender, Action::Battle)))
        }
        Status::Victory | Status::Timeout | Status::Insolvent => Ok(None),
    }
}

/// The pilot `errand-v2`: `errand-v1` in the king's service. It holds a contract on every band it goes to
/// fight, so that each is a villain caught; it goes back to the town for the next contract, and to the king's
/// castle as soon as a rank is earned. The rest is `errand-v1`: it buys what it can keep, fights the bands in
/// the order the expedition lists them, and searches where the clue points.
///
/// It reads what the player's screen shows: the rank above the hero's and the villains it asks for.
#[must_use]
pub fn errand_v2(expedition: &Expedition) -> Option<Action> {
    if let Ok(action) = off_the_map(expedition) {
        return action;
    }
    let def = expedition.def();
    let here = expedition.hero();
    let walk = |to: Cell| step_towards(expedition, to).map(|(dx, dy)| Action::Move { dx, dy });
    if expedition.refusal(&Action::Promote).is_none() {
        return Some(Action::Promote);
    }
    let next = def
        .encounters
        .iter()
        .find(|band| !expedition.is_cleared(&band.object_id));
    if here == def.town {
        if let Some(band) = next.filter(|_| expedition.contract().is_none()) {
            let take = Action::TakeContract {
                id: band.object_id.clone(),
            };
            if expedition.refusal(&take).is_none() {
                return Some(take);
            }
        }
        if let Some(buy) = purchase(expedition) {
            return Some(buy);
        }
    }
    let villains = expedition.standing().map_or(0, |s| s.villains);
    let earned = expedition
        .next_rank()
        .is_some_and(|rank| rank.villains_needed <= villains);
    if let Some(king) = def.king.as_ref().filter(|_| earned) {
        return Some(walk(king.castle).unwrap_or(Action::WaitTick));
    }
    if let Some(place) = expedition.objective_known() {
        if here == place {
            return Some(Action::Search);
        }
        return Some(walk(place).unwrap_or(Action::WaitTick));
    }
    // To the band whose contract is held; with none, to the town for one.
    let band = expedition.contract().and_then(|id| def.map.object(id));
    let to = match (band, next) {
        (Some(band), _) => band.cell(),
        (None, Some(_)) => def.town,
        (None, None) => return Some(Action::WaitTick),
    };
    Some(walk(to).unwrap_or(Action::WaitTick))
}

/// The pilot's next action, or `None` when the expedition is over.
#[must_use]
pub fn errand_v1(expedition: &Expedition) -> Option<Action> {
    if let Ok(action) = off_the_map(expedition) {
        return action;
    }
    let def = expedition.def();
    if expedition.hero() == def.town {
        // The one contract worth holding is on the band that gives the clue.
        let wanted = def
            .encounters
            .iter()
            .find(|band| band.grants_clue && !expedition.is_cleared(&band.object_id));
        if let Some(band) =
            wanted.filter(|band| expedition.contract() != Some(band.object_id.as_str()))
        {
            let take = Action::TakeContract {
                id: band.object_id.clone(),
            };
            if expedition.refusal(&take).is_none() {
                return Some(take);
            }
        }
        if let Some(buy) = purchase(expedition) {
            return Some(buy);
        }
    }
    let walk = |to: Cell| step_towards(expedition, to).map(|(dx, dy)| Action::Move { dx, dy });
    if let Some(place) = expedition.objective_known() {
        if expedition.hero() == place {
            return Some(Action::Search);
        }
        return Some(walk(place).unwrap_or(Action::WaitTick));
    }
    let standing = def
        .encounters
        .iter()
        .find(|band| !expedition.is_cleared(&band.object_id));
    let target = standing.and_then(|band| def.map.object(&band.object_id));
    Some(
        target
            .and_then(|object| walk(object.cell()))
            .unwrap_or(Action::WaitTick),
    )
}
