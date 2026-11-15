//! The hero's spells of battle under the reference rules (`docs/spec/07-RULES-REGISTER.md`, U-13): cast from his
//! book, one a round, during the turn of one of his stacks, with his spell power.

use super::{Action, Battle, Blow, Event, RuleError, Stack};
use crate::content::{Ability, BattleSpell, Cell, Powers, Side};

/// Damage for a point of spell power.
const FIREBALL: u32 = 25;
const LIGHTNING: u32 = 10;
const TURN_UNDEAD: u32 = 50;
/// Health a point of spell power adds to a stack that is cloned.
const CLONE: u32 = 10;

impl Battle {
    /// What the artefacts of the player's hero do in this battle.
    #[must_use]
    pub const fn powers(&self) -> Powers {
        self.powers
    }

    /// The spell power of the player's hero, where he casts from his book.
    #[must_use]
    pub const fn spell_power(&self) -> Option<u32> {
        self.spell_power
    }

    /// Whether the spell can be cast on the stack: its own side for the spells that add to a stack, the enemy for
    /// those that harm, and never a troop that magic does nothing to.
    fn takes(&self, spell: BattleSpell, stack: &Stack) -> bool {
        let troop = &self.rules.troops[stack.troop];
        let (own, immune) = (stack.side == Side::Player, troop.has(Ability::Immune));
        stack.alive()
            && match spell {
                BattleSpell::Teleport => true,
                BattleSpell::Clone => own,
                BattleSpell::Resurrect => {
                    own && stack.began_with.is_some_and(|began| stack.count < began)
                }
                BattleSpell::Fireball | BattleSpell::Lightning | BattleSpell::Freeze => {
                    !own && !immune
                }
                BattleSpell::TurnUndead => !own && !immune && troop.has(Ability::Undead),
            }
    }

    /// The casts open to the hero now: by spell, then by target, and for the spell that carries a stack away by
    /// the cells it can be set down on, row by row.
    pub(super) fn castable(&self, living: &[&Stack]) -> Vec<Action> {
        let board = &self.rules.board;
        let cells = || (0..board.height).flat_map(|y| (0..board.width).map(move |x| Cell { x, y }));
        let mut actions = Vec::new();
        for (id, _) in self.hero_charges.iter().filter(|(_, left)| **left > 0) {
            let Some(spell) = self.rules.battle_spell(id) else {
                continue;
            };
            for stack in living.iter().filter(|stack| self.takes(spell, stack)) {
                let (spell_id, target) = (id.clone(), stack.id.clone());
                if spell == BattleSpell::Teleport {
                    let free = cells().filter(|&to| self.free(to));
                    actions.extend(free.map(|to| Action::CastTo {
                        spell: spell_id.clone(),
                        target: target.clone(),
                        to,
                    }));
                } else {
                    actions.push(Action::Cast {
                        spell: spell_id,
                        target,
                    });
                }
            }
        }
        actions
    }

    /// A spell of the hero's book. Everything is checked before the charge is spent, so a spell that cannot be
    /// cast costs nothing; `to` is where the spell that carries a stack away sets it down, and no other has one.
    pub(super) fn cast_from_book(
        &mut self,
        actor: usize,
        spell_id: &str,
        target: &str,
        to: Option<Cell>,
        events: &mut Vec<Event>,
    ) -> Result<(), RuleError> {
        if self.stacks[actor].side != Side::Player {
            return Err(RuleError::NoHero);
        }
        if self.cast_this_round {
            return Err(RuleError::AlreadyCast);
        }
        let no_charge = || RuleError::NoCharge {
            spell: spell_id.to_owned(),
        };
        let spell = self.rules.battle_spell(spell_id).ok_or_else(no_charge)?;
        let power = self.spell_power.ok_or_else(no_charge)?.max(1);
        if self.hero_charges.get(spell_id).copied().unwrap_or(0) == 0 {
            return Err(no_charge());
        }
        let wrong = || RuleError::WrongSpellTarget {
            target: target.to_owned(),
        };
        let at = self.index(target).ok_or_else(wrong)?;
        if !self.takes(spell, &self.stacks[at]) || (spell == BattleSpell::Teleport) != to.is_some()
        {
            return Err(wrong());
        }
        if let Some(to) = to.filter(|&to| !self.free(to)) {
            return Err(RuleError::CannotEnter { to });
        }
        if let Some(charges) = self.hero_charges.get_mut(spell_id) {
            *charges -= 1;
        }
        self.cast_this_round = true;
        events.push(Event::SpellCast {
            spell: spell_id.to_owned(),
            target: target.to_owned(),
        });
        let harm = |by: u32| by.saturating_mul(power);
        match spell {
            BattleSpell::Fireball => self.burn(at, harm(FIREBALL), events),
            BattleSpell::Lightning => self.burn(at, harm(LIGHTNING), events),
            BattleSpell::TurnUndead => self.burn(at, harm(TURN_UNDEAD), events),
            BattleSpell::Clone => self.clone_stack(at, power, events),
            BattleSpell::Resurrect => self.revive(at, power, events),
            BattleSpell::Freeze => {
                self.stacks[at].frozen = true;
                events.push(Event::Frozen {
                    target: target.to_owned(),
                });
            }
            BattleSpell::Teleport => {
                if let Some(to) = to {
                    let from = std::mem::replace(&mut self.stacks[at].cell, to);
                    events.push(Event::Teleported {
                        stack: target.to_owned(),
                        from,
                        to,
                    });
                }
            }
        }
        Ok(())
    }

    /// A spell's damage: nothing is rolled, nothing scales it, and nobody strikes back.
    fn burn(&mut self, at: usize, damage: u32, events: &mut Vec<Event>) {
        let blow = Blow {
            roll: damage,
            rolls: 1,
            attackers: 1,
            damage: u64::from(damage),
            retaliation: false,
            morale: None,
            scythed: 0,
        };
        self.hurt_by(None, at, blow, events);
    }

    /// Ten health for a point of spell power, and the stack's injury with it, make whole creatures of the
    /// stack's troop; what is left over is the stack's injury afterwards. The number the stack began with rises
    /// by the clones.
    fn clone_stack(&mut self, at: usize, power: u32, events: &mut Vec<Event>) {
        let hp = self.rules.troops[self.stacks[at].troop].hp;
        let stack = &mut self.stacks[at];
        let health = CLONE.saturating_mul(power).saturating_add(stack.injury);
        // No more than the stack can count.
        let clones = (health / hp).min(u32::MAX - stack.count);
        stack.count += clones;
        stack.injury = health % hp;
        stack.began_with = stack.began_with.map(|began| began.saturating_add(clones));
        events.push(Event::Cloned {
            target: stack.id.clone(),
            clones,
            count: stack.count,
            injury: stack.injury,
        });
    }

    /// A creature for a point of spell power comes back, up to the number the stack began with, and the stack is
    /// unhurt.
    fn revive(&mut self, at: usize, power: u32, events: &mut Vec<Event>) {
        let stack = &mut self.stacks[at];
        let fallen = stack
            .began_with
            .map_or(0, |began| began.saturating_sub(stack.count));
        let revived = power.min(fallen);
        stack.count += revived;
        stack.injury = 0;
        events.push(Event::Revived {
            target: stack.id.clone(),
            revived,
            count: stack.count,
        });
    }
}
