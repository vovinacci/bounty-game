//! The second expedition, `content/meadow/`, on the reference's troops: the rules of the campaign that the demo
//! does not reach (upkeep by a troop's cost, a week that cannot be paid, the fallen who return, a band of several
//! stacks), the expedition pilot, and saving, under the reference rules and the variant.

use std::path::Path;
use std::sync::Arc;

use bounty_ai::errand_v1;
use bounty_core::campaign::{Action, Command, Event, Expedition, ExpeditionDef, Status};
use bounty_core::content::{Profile, Rules, Unpaid};
use bounty_core::save;
use bounty_tools::expedition::load_under;

fn meadow(rules: &str) -> ExpeditionDef {
    let content = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let rules = content.join(rules).join("rules.json");
    let def = load_under(&content.join("meadow/expedition.json"), Some(&rules)).unwrap();
    (*def).clone()
}

/// The expedition with the campaign switches of its rules set by hand.
fn switched(mut def: ExpeditionDef, leave: bool, returning: u32) -> ExpeditionDef {
    let mut rules: Rules = (*def.rules).clone();
    let Profile::Reference(reference) = &mut rules.profile else {
        panic!("the meadow is played under the reference rules or their variant");
    };
    reference.switches.unpaid = if leave { Unpaid::Leave } else { Unpaid::Stay };
    reference.switches.fallen_return_percent = returning;
    def.rules = Arc::new(rules);
    def
}

fn act(expedition: &mut Expedition, action: Action) -> Vec<Event> {
    let said = format!("{action:?}");
    let command = Command {
        revision: expedition.revision(),
        action,
    };
    expedition
        .apply(&command)
        .unwrap_or_else(|refused| panic!("{said} refused: {refused}"))
}

/// Plays on with the pilot until the expedition is over; every event, in order. No command of the pilot is ever
/// refused, and every state on the way comes back from its save as it was.
fn play_out(expedition: &mut Expedition) -> Vec<Event> {
    let mut events = Vec::new();
    for _ in 0..100_000 {
        let Some(action) = errand_v1(expedition) else {
            return events;
        };
        events.extend(act(expedition, action));
        let saved = save::encode(expedition);
        let loaded = save::decode(&saved, Arc::clone(expedition.def_shared())).unwrap();
        assert_eq!(&loaded, expedition);
    }
    panic!("the pilot does not come to an end");
}

fn count(expedition: &Expedition, troop: &str) -> u32 {
    let stacks = expedition.army().iter().filter(|s| s.troop_id == troop);
    stacks.map(|s| s.count).sum()
}

#[test]
fn the_variant_file_sets_the_campaign_switches_and_the_reference_has_none() {
    let switches = |def: &ExpeditionDef| match &def.rules.profile {
        Profile::Reference(reference) => reference.switches,
        Profile::Prototype => panic!("not the prototype"),
    };
    let (reference, modern) = (switches(&meadow("reference")), switches(&meadow("modern")));
    assert_eq!(
        (reference.unpaid, reference.fallen_return_percent),
        (Unpaid::Stay, 0)
    );
    assert_eq!(
        (modern.unpaid, modern.fallen_return_percent),
        (Unpaid::Stay, 25)
    );
}

// The reference's rule: a tenth of the recruiting cost a creature, rounded down, each week.
#[test]
fn upkeep_is_a_tenth_of_each_creatures_cost() {
    let mut e = Expedition::start(Arc::new(meadow("reference")));
    assert_eq!(e.upkeep(), 20, "twenty Sprites at 15: one each");
    act(
        &mut e,
        Action::Recruit {
            troop_id: "archers".to_owned(),
            quantity: 2,
        },
    );
    act(
        &mut e,
        Action::Recruit {
            troop_id: "wolves".to_owned(),
            quantity: 3,
        },
    );
    assert_eq!(e.upkeep(), 20 + 2 * 25 + 3 * 4);
    let gold = e.gold();
    let mut settled = Vec::new();
    for _ in 0..20 {
        settled.extend(act(&mut e, Action::WaitTick));
    }
    let week = Event::WeekSettled {
        week: 1,
        income: 1000,
        upkeep: 82,
        gold: gold + 1000 - 82,
    };
    assert!(settled.contains(&week), "{settled:?}");
}

/// The meadow with an army it cannot pay: twenty gold in the purse and this much income, against an upkeep of
/// 20 for the Sprites, 250 for the Archers and 48 for the Wolves.
fn broke(def: ExpeditionDef, income: u64) -> Expedition {
    let mut def = def;
    def.clock.weekly_income = income;
    let mut e = Expedition::start(Arc::new(def));
    act(
        &mut e,
        Action::Recruit {
            troop_id: "archers".to_owned(),
            quantity: 10,
        },
    );
    act(
        &mut e,
        Action::Recruit {
            troop_id: "wolves".to_owned(),
            quantity: 12,
        },
    );
    assert_eq!((e.gold(), e.upkeep()), (20, 20 + 250 + 48));
    e
}

fn a_week(expedition: &mut Expedition) -> Vec<Event> {
    (0..20)
        .flat_map(|_| act(expedition, Action::WaitTick))
        .collect()
}

#[test]
fn a_week_that_cannot_be_paid_empties_the_purse_and_everyone_stays() {
    let mut e = broke(switched(meadow("reference"), false, 0), 0);
    let events = a_week(&mut e);
    assert!(events.contains(&Event::WeekSettled {
        week: 1,
        income: 0,
        upkeep: 20,
        gold: 0
    }));
    assert_eq!((e.gold(), e.status()), (0, Status::Adventure));
    assert_eq!(
        (
            count(&e, "sprites"),
            count(&e, "archers"),
            count(&e, "wolves")
        ),
        (20, 10, 12)
    );
}

// The switch: stacks are paid in slot order, each in full or not at all; one that is not paid leaves, and those
// after it are still paid if they can be.
#[test]
fn with_the_switch_the_stacks_that_cannot_be_paid_leave() {
    let mut e = broke(switched(meadow("reference"), true, 0), 0);
    let events = a_week(&mut e);
    // Sprites cost 20 and are paid; nothing is left for the Archers (250) or the Wolves (48).
    assert!(events.contains(&Event::StackLeft {
        troop_id: "archers".to_owned(),
        count: 10
    }));
    assert!(events.contains(&Event::StackLeft {
        troop_id: "wolves".to_owned(),
        count: 12
    }));
    assert!(events.contains(&Event::WeekSettled {
        week: 1,
        income: 0,
        upkeep: 20,
        gold: 0
    }));
    assert_eq!(
        (
            count(&e, "sprites"),
            count(&e, "archers"),
            count(&e, "wolves")
        ),
        (20, 0, 0)
    );
    // With a little more the Wolves, who come after the Archers, are paid and stay.
    let mut e = broke(switched(meadow("reference"), true, 0), 50);
    a_week(&mut e);
    assert_eq!(
        (
            count(&e, "sprites"),
            count(&e, "archers"),
            count(&e, "wolves")
        ),
        (20, 0, 12)
    );
    assert_eq!(e.gold(), 20 + 50 - 20 - 48);
}

// After a won battle a share of each stack's fallen returns, rounded down; who won is untouched.
#[test]
fn a_share_of_the_fallen_of_a_won_battle_returns() {
    let play = |returning: u32| {
        let mut e = Expedition::start(Arc::new(switched(meadow("modern"), false, returning)));
        let mut events = Vec::new();
        while !events
            .iter()
            .any(|event| matches!(event, Event::BattleWon { .. }))
        {
            let action = errand_v1(&e).expect("the first battle is won before the expedition ends");
            events.extend(act(&mut e, action));
        }
        (e, events)
    };
    let (none, plain) = play(0);
    let (half, with_half) = play(50);
    assert!(
        !plain
            .iter()
            .any(|event| matches!(event, Event::FallenReturned { .. }))
    );
    let mut back = 0;
    for event in &with_half {
        if let Event::FallenReturned { count, .. } = event {
            back += count;
        }
    }
    assert!(back > 0, "the first battle costs creatures: {with_half:?}");
    // The same battle was fought: the same army went in, and half of what fell, stack by stack, came back.
    let creatures = |e: &Expedition| e.army().iter().map(|s| s.count).sum::<u32>();
    assert_eq!(creatures(&half), creatures(&none) + back);
    assert_eq!(none.gold(), half.gold());
    assert_eq!(none.ticks(), half.ticks());
}

#[test]
fn a_band_of_two_stacks_stands_down_the_enemys_column() {
    let def = Arc::new(meadow("reference"));
    let mut e = Expedition::start(Arc::clone(&def));
    while e.engaged_with() != Some("meadow-company") {
        let action = errand_v1(&e).expect("the pilot reaches the second band");
        act(&mut e, action);
    }
    let battle = e.battle().unwrap();
    let enemy = |id: &str| {
        let stack = battle.stack(id).unwrap();
        (
            stack.troop_id.as_str(),
            stack.count,
            stack.cell.x,
            stack.cell.y,
        )
    };
    assert_eq!(enemy("e0"), ("archers", 6, 5, 1));
    assert_eq!(enemy("e1"), ("sprites", 30, 5, 2));
    assert_eq!(
        battle.leadership(),
        Some(100),
        "the hero's leadership goes into the battle"
    );
}

// Under both sets of rules the pilot plays the meadow to an end, the same way every time, with what the report
// needs along the way.
#[test]
fn the_pilot_plays_the_meadow_to_an_end_under_both_rules() {
    for rules in ["reference", "modern"] {
        let def = Arc::new(meadow(rules));
        let mut first = Expedition::start(Arc::clone(&def));
        let events = play_out(&mut first);
        let mut second = Expedition::start(Arc::clone(&def));
        play_out(&mut second);
        assert_eq!(first.digest(), second.digest(), "{rules}");
        assert_eq!(first.status(), Status::Victory, "{rules}");
        let said = |wanted: &dyn Fn(&Event) -> bool| events.iter().any(wanted);
        assert!(said(
            &|e| matches!(e, Event::ContractTaken { id } if id == "meadow-company")
        ));
        assert!(said(&|e| matches!(e, Event::ClueGained { .. })));
        assert!(said(&|e| matches!(e, Event::Searched { found: true })));
        let returned = said(&|e| matches!(e, Event::FallenReturned { .. }));
        assert_eq!(returned, rules == "modern", "{rules}");
    }
}

// The pilot sees what a player sees: with the prize buried elsewhere it does exactly the same until the clue
// tells it where.
#[test]
fn the_pilot_does_not_know_where_the_prize_lies_before_the_clue() {
    let def = meadow("reference");
    let mut moved = def.clone();
    let prize = moved
        .map
        .objects
        .iter_mut()
        .find(|o| o.id == "meadow-objective")
        .unwrap();
    (prize.x, prize.y) = (12, 7);
    let (mut here, mut there) = (
        Expedition::start(Arc::new(def)),
        Expedition::start(Arc::new(moved)),
    );
    let mut compared = 0;
    while here.objective_known().is_none() {
        let action = errand_v1(&here).unwrap();
        assert_eq!(errand_v1(&there).as_ref(), Some(&action));
        act(&mut here, action.clone());
        act(&mut there, action);
        compared += 1;
    }
    assert!(compared > 20);
    play_out(&mut there);
    assert_eq!((there.status(), there.hero().x), (Status::Victory, 12));
}

// The pilot in the king's service: a contract on each band it fights, back to the town for the next, to the castle
// when a rank is earned, and only then to the prize. No command is refused, every state on the way comes back
// from its save, and the hero ends as a General. The plain pilot on the same meadow catches one villain, the one
// whose contract gives the clue, and stays a Knight.
#[test]
fn the_pilot_in_the_kings_service_is_promoted_and_the_plain_one_is_not() {
    use bounty_ai::errand_v2;
    for rules in ["reference", "modern"] {
        let mut e = Expedition::start(Arc::new(meadow(rules)));
        let mut events = Vec::new();
        for _ in 0..100_000 {
            let Some(action) = errand_v2(&e) else { break };
            events.extend(act(&mut e, action));
            let loaded = save::decode(&save::encode(&e), Arc::clone(e.def_shared())).unwrap();
            assert_eq!(loaded, e);
        }
        assert_eq!(e.status(), Status::Victory, "{rules}");
        let standing = e.standing().unwrap();
        assert_eq!((standing.villains, standing.rank), (2, 1), "{rules}");
        assert_eq!(e.rank().unwrap().id, "general");
        assert_eq!((e.leadership(), e.weekly_income()), (200, 2000));
        let promoted = events
            .iter()
            .filter(|event| matches!(event, Event::Promoted { ranks: 1, .. }));
        assert_eq!(promoted.count(), 1, "{rules}: one audience, one rank");

        let mut plain = Expedition::start(Arc::new(meadow(rules)));
        play_out(&mut plain);
        assert_eq!(plain.status(), Status::Victory);
        let standing = plain.standing().unwrap();
        assert_eq!((standing.villains, standing.rank), (1, 0), "{rules}");
    }
}

fn vale(rules: &str) -> ExpeditionDef {
    let content = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content");
    let rules = content.join(rules).join("rules.json");
    let def = load_under(&content.join("vale/expedition.json"), Some(&rules)).unwrap();
    (*def).clone()
}

// The vale, this project's larger expedition: a town that sells nothing, four dwellings, the king's castle, four
// bands and two castles. The pilot that recruits wherever troops are sold beats all four bands; no command is
// refused, and every state on the way comes back from its save.
#[test]
fn the_pilot_that_recruits_everywhere_beats_the_vales_bands() {
    use bounty_ai::errand_v3;
    for rules in ["reference", "modern"] {
        let mut e = Expedition::start(Arc::new(vale(rules)));
        let mut events = Vec::new();
        for _ in 0..100_000 {
            let Some(action) = errand_v3(&e) else { break };
            events.extend(act(&mut e, action));
            let loaded = save::decode(&save::encode(&e), Arc::clone(e.def_shared())).unwrap();
            assert_eq!(loaded, e);
        }
        // It beats the four bands; the prize it cannot find, for the clue is with the lord of the hold.
        assert_eq!(e.status(), Status::Timeout, "{rules}");
        // The vale's town gives its contracts in turn, on lords only: the bands are beaten for their gold, and a
        // pilot that cannot besiege catches nobody and stays a Knight.
        assert_eq!(e.rank().unwrap().id, "knight", "{rules}");
        assert_eq!(e.standing().unwrap().villains, 0, "{rules}");
        let bought: std::collections::BTreeSet<&str> = events
            .iter()
            .filter_map(|event| match event {
                Event::Recruited { troop_id, .. } => Some(troop_id.as_str()),
                _ => None,
            })
            .collect();
        // From the castle and from dwellings, with a Knight's leadership to the end.
        for troop in ["militia", "pikemen", "archers", "wolves"] {
            assert!(bought.contains(troop), "{rules}: {troop} of {bought:?}");
        }
        assert_eq!(e.leadership(), 100, "{rules}");
        let weeks = events
            .iter()
            .filter(|event| matches!(event, Event::WeekCreature { .. }));
        assert!(
            weeks.count() >= 5,
            "{rules}: the weeks have their creatures"
        );
    }
}

// In the vale the town sells no troops, so the pilot that buys only there never wins a battle: it is beaten,
// recovers and tries again until time runs out.
#[test]
fn without_the_dwellings_and_the_castle_the_vale_is_lost() {
    use bounty_ai::errand_v2;
    let mut e = Expedition::start(Arc::new(vale("reference")));
    for _ in 0..100_000 {
        let Some(action) = errand_v2(&e) else { break };
        act(&mut e, action);
    }
    assert_eq!(e.status(), Status::Timeout);
    assert_eq!(e.standing().unwrap().villains, 0);
}

// The pilot that opens chests finds all three of the vale's on its way, shares the gold of two out and takes the
// third's commission: a Knight's 100 leadership and 900 gold's worth more, 1000 a week and 40 more.
#[test]
fn the_pilot_that_opens_chests_ends_with_what_they_held() {
    use bounty_ai::errand_v4;
    for rules in ["reference", "modern"] {
        let mut e = Expedition::start(Arc::new(vale(rules)));
        for _ in 0..100_000 {
            let Some(action) = errand_v4(&e) else { break };
            act(&mut e, action);
            let loaded = save::decode(&save::encode(&e), Arc::clone(e.def_shared())).unwrap();
            assert_eq!(loaded, e);
        }
        assert_eq!(e.status(), Status::Timeout, "{rules}");
        for chest in ["vale-purse", "vale-hoard", "vale-writ"] {
            assert!(e.is_opened(chest), "{rules}: {chest}");
        }
        assert_eq!((e.leadership(), e.weekly_income()), (118, 1040), "{rules}");
    }
}

// The pilot that besieges: with the four bands beaten it takes the contracts the town gives in turn, on the lord of
// the keep and then on the lord of the hold, buys siege weapons, takes both castles and leaves a creature in each;
// the second lord's clue leads it to the prize, by way of the king, who makes it a General. The pilots that cannot
// besiege beat the bands and then have nowhere to go.
#[test]
fn the_pilot_that_besieges_catches_the_lord_and_finds_the_prize() {
    use bounty_ai::{errand_v4, errand_v5};
    for rules in ["reference", "modern"] {
        let mut e = Expedition::start(Arc::new(vale(rules)));
        for _ in 0..100_000 {
            let Some(action) = errand_v5(&e) else { break };
            act(&mut e, action);
            let loaded = save::decode(&save::encode(&e), Arc::clone(e.def_shared())).unwrap();
            assert_eq!(loaded, e);
        }
        assert_eq!(e.status(), Status::Victory, "{rules}");
        assert!(e.has_siege_weapons(), "{rules}");
        for castle in ["vale-keep", "vale-hold"] {
            assert!(e.is_taken(castle), "{rules}: {castle}");
            let left: u32 = e.garrison(castle).iter().map(|(_, count)| count).sum();
            assert_eq!(left, 1, "{rules}: one creature holds {castle}");
        }
        assert_eq!(e.standing().unwrap().villains, 2, "{rules}: two lords");
        assert_eq!(e.rank().unwrap().id, "general", "{rules}");

        let mut e = Expedition::start(Arc::new(vale(rules)));
        for _ in 0..100_000 {
            let Some(action) = errand_v4(&e) else { break };
            act(&mut e, action);
        }
        assert_eq!(e.status(), Status::Timeout, "{rules}");
        assert_eq!(e.standing().unwrap().villains, 0, "{rules}");
    }
}

// The pilot that takes in whoever would join: the vale's Gnomes join it for nothing and take the fifth place of
// its army. With that place gone to twenty Gnomes it does not take the lord's hold, which the pilot that leaves
// the Gnomes alone does; a band that joins is not always a gift.
#[test]
fn the_pilot_that_takes_in_a_band_pays_for_the_place_it_takes() {
    use bounty_ai::errand_v6;
    for rules in ["reference", "modern"] {
        let mut e = Expedition::start(Arc::new(vale(rules)));
        let mut events = Vec::new();
        for _ in 0..100_000 {
            let Some(action) = errand_v6(&e) else { break };
            events.extend(act(&mut e, action));
            let loaded = save::decode(&save::encode(&e), Arc::clone(e.def_shared())).unwrap();
            assert_eq!(loaded, e);
        }
        let joined = Event::BandJoined {
            band: "vale-gnomes".to_owned(),
            troop_id: "gnomes".to_owned(),
            count: 20,
        };
        assert!(events.contains(&joined), "{rules}");
        assert!(e.is_cleared("vale-gnomes"), "{rules}");
        assert_eq!(e.status(), Status::Timeout, "{rules}");
        assert!(!e.is_taken("vale-hold"), "{rules}");
    }
}
