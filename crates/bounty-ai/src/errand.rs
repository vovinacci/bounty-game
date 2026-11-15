//! The expedition pilot `errand-v1`: a plain player of a whole expedition, for measuring what a rule costs over a
//! game. It recruits what it can keep, takes the contract that leads to the clue, walks to the bands in the
//! order the expedition lists them, fights with [`approach_v1`], and searches where the clue points. It is not
//! clever and is not meant to be: it is the same before and after a change.
//!
//! It is handed the expedition but uses only what the player's screen shows: where the objective lies is read
//! from the clue (`objective_known`), never from the definition.

use std::collections::VecDeque;

use bounty_core::campaign::{Action, Expedition, ObjectKind, Status};
use bounty_core::content::Cell;

use crate::approach_v1;

pub const ERRAND_V1: &str = "errand-v1";
pub const ERRAND_V2: &str = "errand-v2";

/// The first step of a shortest way from the hero to `to` over ground that can be walked, round every band that
/// still stands except one standing on `to` itself.
fn step_towards(expedition: &Expedition, to: Cell) -> Option<(i32, i32)> {
    let map = &expedition.def().map;
    let from = expedition.hero();
    let barred = |cell: Cell| {
        let band = map
            .object_at(cell)
            .filter(|o| o.kind == ObjectKind::Encounter);
        cell != to && band.is_some_and(|band| !expedition.is_cleared(&band.id))
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
    steps.iter().find(nearer).copied()
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
