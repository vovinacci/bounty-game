//! The artefacts of the reference rules (`docs/spec/07-RULES-REGISTER.md`, U-18): found by stepping onto them,
//! once, each with a power that is the hero's from then on.

use super::{Event, Expedition};
use crate::content::{Power, Powers};

/// What the king pays more every week to a hero with the artefact of that power.
const MORE_COMMISSION: u64 = 2000;

impl Expedition {
    /// Whether the hero has found an artefact of this power.
    #[must_use]
    pub fn has_power(&self, power: Power) -> bool {
        self.gain_of(power).is_some()
    }

    /// The artefacts the hero has found, by their IDs in the rules.
    #[must_use]
    pub fn artefacts_found(&self) -> Vec<&str> {
        self.found.keys().map(String::as_str).collect()
    }

    /// Whether the artefact on this object has been found.
    #[must_use]
    pub fn is_found(&self, object_id: &str) -> bool {
        let laid = self.def.artefacts.iter().find(|a| a.object_id == object_id);
        laid.is_some_and(|laid| {
            self.found
                .contains_key(&self.def.rules.artefacts[laid.artefact].id)
        })
    }

    /// What the found artefact of this power has added, for the powers that double what the hero had: his
    /// leadership, spell power or room for spells as it stood, and what chests opened since have given again.
    pub(super) fn gain_of(&self, power: Power) -> Option<u32> {
        let artefact = self.def.rules.artefacts.iter().find(|a| a.power == power)?;
        self.found.get(&artefact.id).copied()
    }

    /// Adds to what the found artefact of this power has given: a chest opened after it gives twice.
    pub(super) fn gain_more(&mut self, power: Power, more: u32) {
        let artefact = self.def.rules.artefacts.iter().find(|a| a.power == power);
        if let Some(gain) = artefact.and_then(|a| self.found.get_mut(&a.id)) {
            *gain = gain.saturating_add(more);
        }
    }

    /// What the king pays more every week for the artefact that says so.
    pub(super) fn commission_of_artefacts(&self) -> u64 {
        if self.has_power(Power::IncreaseCommission) {
            MORE_COMMISSION
        } else {
            0
        }
    }

    /// What the hero's artefacts do in a battle.
    pub(super) fn powers_for_battle(&self) -> Powers {
        Powers {
            more_damage: self.has_power(Power::IncreasedDamage),
            less_harm: self.has_power(Power::QuarterProtection),
        }
    }

    /// The hero has stepped onto a cell: an artefact that lies there and is not yet found is his. One that
    /// doubles takes what he has now of what it doubles, and gives that much again.
    pub(super) fn claim_artefact(&mut self, events: &mut Vec<Event>) {
        let def = &self.def;
        let Some(laid) = def.artefacts.iter().find(|a| a.cell == self.hero) else {
            return;
        };
        let artefact = &def.rules.artefacts[laid.artefact];
        if self.found.contains_key(&artefact.id) {
            return;
        }
        let before = self.puzzle_before();
        let gain = match artefact.power {
            Power::DoubleLeadership => self.leadership_for_good(),
            Power::DoubleSpellPower => self.spell_power(),
            Power::DoubleMaxSpells => self.spell_limit(),
            _ => 0,
        };
        events.push(Event::ArtefactFound {
            object: laid.object_id.clone(),
            artefact: artefact.id.clone(),
        });
        self.found.insert(artefact.id.clone(), gain);
        self.tell_piece_lifted(before, events);
    }
}
