//! Castles that are not the king's (`docs/spec/07-RULES-REGISTER.md`, U-07 and U-14): siege weapons, the siege,
//! a castle taken, a lord caught or freed. The meadow serves, with two castles written into its texts and
//! thirty-three Wolves for an army.

use super::king::close_and_fight;
use super::*;
use crate::campaign::content::{SIEGE_BESIEGERS, SIEGE_GARRISON, SIEGE_WALLS};
use crate::canon::Value;

const KEEP: Cell = Cell { x: 2, y: 3 };
const HALL: Cell = Cell { x: 3, y: 2 };

/// The meadow with a keep of monsters (five Wolves) and a lord's hall (six Militia, 900 gold on his head), siege
/// weapons at 1000, and an army of thirty-three Wolves; with a seed, weeks that have creatures.
fn texts(seed: Option<u32>) -> (String, String) {
    let object = r#"    {
      "id": "meadow-objective","#;
    let castles = r#"    {
      "id": "meadow-keep",
      "kind": "castle",
      "x": 2,
      "y": 3
    },
    {
      "id": "meadow-hall",
      "kind": "castle",
      "x": 3,
      "y": 2
    },
"#;
    assert!(MEADOW_MAP.contains(object));
    let map = MEADOW_MAP.replacen(object, &format!("{castles}{object}"), 1);
    let king = r#"  "king": {"#;
    let listed = r#"  "siege_weapons_cost": 1000,
  "castles": [
    {
      "object_id": "meadow-keep",
      "garrison": [{"troop": "wolves", "count": 5}],
      "battle_rng_state": 31
    },
    {
      "object_id": "meadow-hall",
      "garrison": [{"troop": "militia", "count": 6}, {"troop": "archers", "count": 1}],
      "battle_rng_state": 32,
      "lord": {"reward": 900, "grants_clue": true}
    }
  ],
  "king": {"#;
    assert!(MEADOW.contains(king));
    let mut expedition = MEADOW
        .replacen(king, listed, 1)
        .replacen(r#""troop_id": "sprites""#, r#""troop_id": "wolves""#, 1)
        .replacen(r#""count": 20,"#, r#""count": 33,"#, 1);
    if let Some(seed) = seed {
        let rule = r#""upkeep_rule": "tenth_of_recruit_cost""#;
        let seeded = format!("{rule},\n    \"week_creature_seed\": {seed}");
        expedition = expedition.replacen(rule, &seeded, 1);
    }
    (expedition, map)
}

fn meadow(seed: Option<u32>) -> Expedition {
    let (expedition, map) = texts(seed);
    Expedition::start(Arc::new(
        ExpeditionDef::from_texts(&expedition, &map, REFERENCE).unwrap(),
    ))
}

/// With siege weapons, and with the contract on the hall's lord if asked, one step north of the keep and west of
/// the hall.
fn armed(contract: bool) -> Expedition {
    let mut e = meadow(None);
    act(&mut e, Action::BuySiegeWeapons).unwrap();
    if contract {
        let id = "meadow-hall".to_owned();
        act(&mut e, Action::TakeContract { id }).unwrap();
    }
    go(&mut e, 1, 0, 1);
    go(&mut e, 0, 1, 1);
    e
}

#[test]
fn castles_that_cannot_be_are_refused() {
    let (expedition, map) = texts(None);
    let load = |text: &str| ExpeditionDef::from_texts(text, &map, REFERENCE).map(|_| ());
    assert_eq!(load(&expedition), Ok(()));
    let refused = |old: &str, new: &str| {
        assert!(expedition.contains(old), "{old}");
        load(&expedition.replacen(old, new, 1)).is_err()
    };
    assert!(
        refused(r#"  "siege_weapons_cost": 1000,"#, ""),
        "castles and no price"
    );
    assert!(
        refused(
            r#""object_id": "meadow-keep""#,
            r#""object_id": "meadow-castle""#
        ),
        "the king's"
    );
    assert!(
        refused(
            r#""object_id": "meadow-hall""#,
            r#""object_id": "meadow-keep""#
        ),
        "twice"
    );
    assert!(refused(
        r#"{"troop": "wolves", "count": 5}"#,
        r#"{"troop": "wolves", "count": 0}"#
    ));
    assert!(refused(
        r#"{"troop": "wolves", "count": 5}"#,
        r#"{"troop": "griffins", "count": 5}"#
    ));
    assert!(
        refused(r#"[{"troop": "wolves", "count": 5}]"#, "[]"),
        "no garrison"
    );
    // A price for siege weapons where there is nothing to besiege.
    let lonely = MEADOW.replacen(
        r#"  "king": {"#,
        "  \"siege_weapons_cost\": 1000,\n  \"king\": {",
        1,
    );
    assert!(ExpeditionDef::from_texts(&lonely, MEADOW_MAP, REFERENCE).is_err());
}

#[test]
fn a_castle_is_not_entered_without_siege_weapons_bought_once_in_the_town() {
    let mut e = meadow(None);
    assert!(!e.has_siege_weapons());
    let gold = e.gold();
    go(&mut e, 1, 0, 1);
    refused(&mut e, Action::BuySiegeWeapons, &Refused::NotInTown);
    go(&mut e, 0, 1, 1);
    refused(
        &mut e,
        Action::Move { dx: 0, dy: 1 },
        &Refused::NoSiegeWeapons,
    );
    refused(
        &mut e,
        Action::Move { dx: 1, dy: 0 },
        &Refused::NoSiegeWeapons,
    );
    go(&mut e, 0, -1, 1);
    go(&mut e, -1, 0, 1);
    let events = act(&mut e, Action::BuySiegeWeapons).unwrap();
    assert_eq!(events, [Event::SiegeWeaponsBought { cost: 1000 }]);
    assert_eq!((e.gold(), e.has_siege_weapons()), (gold - 1000, true));
    refused(
        &mut e,
        Action::BuySiegeWeapons,
        &Refused::NoSiegeWeaponsToBuy,
    );
    // Where there is no castle to besiege there are none to buy.
    refused(
        &mut start(),
        Action::BuySiegeWeapons,
        &Refused::NoSiegeWeaponsToBuy,
    );
}

// The reference's layout: the besiegers at the foot of the board, the garrison at its head, walls down both sides
// and in the two lower corners.
#[test]
fn a_siege_is_fought_on_the_castles_ground() {
    let mut e = armed(false);
    let events = act(&mut e, Action::Move { dx: 1, dy: 0 }).unwrap();
    let started = Event::BattleStarted {
        band: "meadow-hall".to_owned(),
    };
    assert!(events.contains(&started), "{events:?}");
    assert_eq!(e.engaged_with(), Some("meadow-hall"));
    let battle = e.battle().unwrap();
    let at = |id: &str| battle.stack(id).unwrap().cell;
    assert_eq!(at("p0"), SIEGE_BESIEGERS[0]);
    assert_eq!((at("e0"), at("e1")), (SIEGE_GARRISON[0], SIEGE_GARRISON[1]));
    let mut walls = SIEGE_WALLS.to_vec();
    walls.sort_by_key(|c| (c.y, c.x));
    assert_eq!(battle.obstacles(), walls);
    assert_eq!(
        battle.stack("e1").unwrap().troop_id,
        "archers",
        "the whole garrison fights"
    );
}

// A castle of monsters that falls is the hero's: he stands in it, and comes and goes as he likes.
#[test]
fn a_castle_of_monsters_that_falls_is_the_heros() {
    let mut e = armed(false);
    assert_eq!(e.garrison("meadow-keep"), [("wolves".to_owned(), 5)]);
    act(&mut e, Action::Move { dx: 0, dy: 1 }).unwrap();
    let events = close_and_fight(&mut e);
    let taken = Event::CastleTaken {
        castle: "meadow-keep".to_owned(),
    };
    assert!(events.contains(&taken), "{events:?}");
    assert!(e.is_taken("meadow-keep") && e.garrison("meadow-keep").is_empty());
    assert_eq!((e.hero(), e.status()), (KEEP, Status::Adventure));
    assert_eq!(
        e.standing().unwrap().villains,
        0,
        "monsters are no villains"
    );
    go(&mut e, 0, -1, 1);
    go(&mut e, 0, 1, 1);
    assert_eq!(e.hero(), KEEP);
}

// A lord's castle taken while his contract is held: the castle, his reward, a villain caught, and his clue.
#[test]
fn a_lord_is_caught_by_taking_his_castle_under_contract() {
    let mut e = armed(true);
    let gold = e.gold();
    act(&mut e, Action::Move { dx: 1, dy: 0 }).unwrap();
    let events = close_and_fight(&mut e);
    let hall = || "meadow-hall".to_owned();
    for event in [
        Event::BattleWon {
            band: hall(),
            reward: 900,
        },
        Event::CastleTaken { castle: hall() },
        Event::VillainCaught {
            band: hall(),
            caught: 1,
        },
    ] {
        assert!(events.contains(&event), "{event:?} in {events:?}");
    }
    assert_eq!((e.gold(), e.hero(), e.contract()), (gold + 900, HALL, None));
    assert!(e.is_taken("meadow-hall") && e.objective_known().is_some());
    assert_eq!(e.standing().unwrap().villains, 1);
    // Caught, he has no contract left to take; and monsters never had one.
    go(&mut e, -1, 0, 1);
    go(&mut e, 0, -1, 1);
    go(&mut e, -1, 0, 1);
    for id in ["meadow-hall", "meadow-keep"] {
        let none = Refused::NoSuchContract { id: id.to_owned() };
        refused(&mut e, Action::TakeContract { id: id.to_owned() }, &none);
    }
}

// Taken without the contract, the castle is not kept: the lord goes free, his garrison is as it first was, and
// the hero stands before the gate. With the contract he has to be fought again.
#[test]
fn a_lord_beaten_without_his_contract_goes_free() {
    let mut e = armed(false);
    let (gold, before) = (e.gold(), e.hero());
    act(&mut e, Action::Move { dx: 1, dy: 0 }).unwrap();
    let events = close_and_fight(&mut e);
    let freed = Event::LordFreed {
        castle: "meadow-hall".to_owned(),
    };
    assert!(events.contains(&freed), "{events:?}");
    assert!(!e.is_taken("meadow-hall"));
    let first = [("militia".to_owned(), 6), ("archers".to_owned(), 1)];
    assert_eq!(e.garrison("meadow-hall"), first);
    assert_eq!(
        (e.gold(), e.hero(), e.status()),
        (gold, before, Status::Adventure)
    );
    assert_eq!(e.standing().unwrap().villains, 0);
    assert_eq!(e.objective_known(), None);
}

// The week's creature grows in the castles that are not the hero's, as in the bands: under seed 22 the keep's
// five Wolves are ten after the first week.
#[test]
fn the_weeks_creature_grows_in_the_castles() {
    let mut e = meadow(Some(22));
    let week = e.def().clock.week_ticks;
    let mut events = Vec::new();
    while e.ticks() < week {
        events.extend(act(&mut e, Action::WaitTick).unwrap());
    }
    let grew = Event::BandGrew {
        band: "meadow-keep".to_owned(),
        troop_id: "wolves".to_owned(),
        count: 10,
    };
    assert!(events.contains(&grew), "{events:?}");
    assert_eq!(e.garrison("meadow-keep"), [("wolves".to_owned(), 10)]);
}

// How the castles stand is part of the state where there are castles, in the middle of a siege too.
#[test]
fn how_the_castles_stand_is_saved() {
    let mut e = armed(true);
    act(&mut e, Action::Move { dx: 1, dy: 0 }).unwrap();
    let def = Arc::clone(e.def_shared());
    let again = |e: &Expedition| {
        let battle = e.battle().map(Battle::to_canon);
        Expedition::from_canon(Arc::clone(&def), &e.to_canon(), battle.as_ref()).unwrap()
    };
    assert_eq!(again(&e), e, "in the middle of the siege");
    close_and_fight(&mut e);
    assert_eq!(again(&e), e, "with the castle taken");

    let text = String::from_utf8(e.to_canon().to_bytes().unwrap()).unwrap();
    let changed = |old: &str, new: &str| {
        assert!(text.contains(old), "{old} in {text}");
        let value = Value::parse(&text.replacen(old, new, 1)).unwrap();
        Expedition::from_canon(Arc::clone(&def), &value, None).map(|_| ())
    };
    assert!(
        changed(r#""siege_weapons":true"#, r#""siege_weapons":false"#).is_err(),
        "taken unarmed"
    );
    let keep = r#""meadow-keep":{"caught":false,"garrison":[5],"own":[],"taken":false}"#;
    let stands = |garrison: &str, taken: bool, caught: bool| {
        let other = format!(
            r#""meadow-keep":{{"caught":{caught},"garrison":{garrison},"own":[],"taken":{taken}}}"#
        );
        changed(keep, &other)
    };
    assert!(stands("[4]", false, false).is_err(), "shrunk");
    assert!(stands("[5]", true, false).is_err(), "taken and held");
    assert!(
        stands("[5]", false, true).is_err(),
        "monsters have no lord to catch"
    );

    let plain = Expedition::start(Arc::new(
        ExpeditionDef::from_texts(MEADOW, MEADOW_MAP, REFERENCE).unwrap(),
    ));
    let plain = String::from_utf8(plain.to_canon().to_bytes().unwrap()).unwrap();
    assert!(!plain.contains("castles") && !plain.contains("siege_weapons"));
}

/// The keep taken, the hero standing in it with what the siege left him.
fn in_the_keep(seed: Option<u32>) -> Expedition {
    let mut e = meadow(seed);
    act(&mut e, Action::BuySiegeWeapons).unwrap();
    go(&mut e, 1, 0, 1);
    go(&mut e, 0, 1, 1);
    act(&mut e, Action::Move { dx: 0, dy: 1 }).unwrap();
    close_and_fight(&mut e);
    assert!(e.is_taken("meadow-keep") && e.hero() == KEEP);
    e
}

fn leave(troop: &str, quantity: u32) -> Action {
    Action::Garrison {
        troop_id: troop.to_owned(),
        quantity,
    }
}

fn take(troop: &str, quantity: u32) -> Action {
    Action::Withdraw {
        troop_id: troop.to_owned(),
        quantity,
    }
}

// In a castle of his own the hero leaves creatures and takes them back; none is lost or made on the way, the
// army keeps at least one, and those left behind cost no upkeep.
#[test]
fn creatures_are_left_in_a_castle_and_taken_back() {
    let mut e = in_the_keep(None);
    let wolves = e.army()[0].count;
    let upkeep = e.upkeep();
    let events = act(&mut e, leave("wolves", 10)).unwrap();
    assert_eq!(
        events,
        [Event::Garrisoned {
            castle: "meadow-keep".to_owned(),
            troop_id: "wolves".to_owned(),
            count: 10
        }]
    );
    assert_eq!(e.army()[0].count, wolves - 10);
    assert_eq!(e.garrison("meadow-keep"), [("wolves".to_owned(), 10)]);
    assert_eq!(e.upkeep(), upkeep - 40, "ten Wolves at four gold a week");

    let rest = wolves - 10;
    refused(&mut e, leave("wolves", rest), &Refused::ArmyEmptied);
    refused(
        &mut e,
        leave("wolves", rest + 1),
        &Refused::NotSoMany { have: rest },
    );
    refused(&mut e, leave("sprites", 1), &Refused::NotSoMany { have: 0 });
    refused(&mut e, leave("wolves", 0), &Refused::NoQuantity);

    act(&mut e, take("wolves", 4)).unwrap();
    assert_eq!(e.army()[0].count, wolves - 6);
    assert_eq!(e.garrison("meadow-keep"), [("wolves".to_owned(), 6)]);
    refused(&mut e, take("wolves", 7), &Refused::NotSoMany { have: 6 });
    refused(&mut e, take("militia", 1), &Refused::NotSoMany { have: 0 });
    act(&mut e, take("wolves", 6)).unwrap();
    assert_eq!(
        (e.army()[0].count, e.garrison("meadow-keep").len()),
        (wolves, 0)
    );

    // A stack that cannot be counted any higher takes no more, and nothing is moved: neither out of the army
    // into a castle that has all it can count, nor back.
    act(&mut e, leave("wolves", 1)).unwrap();
    e.holds.get_mut("meadow-keep").unwrap().own[0].1 = u32::MAX;
    refused(&mut e, leave("wolves", 1), &Refused::StackTooLarge);
    e.holds.get_mut("meadow-keep").unwrap().own[0].1 = 5;
    e.army[0].count = u32::MAX;
    refused(&mut e, take("wolves", 1), &Refused::StackTooLarge);
    e.army[0].count = wolves - 1;
    act(&mut e, take("wolves", 5)).unwrap();

    // Only in a castle of his own.
    go(&mut e, 0, -1, 1);
    refused(&mut e, leave("wolves", 1), &Refused::NotInOwnCastle);
    refused(&mut e, take("wolves", 1), &Refused::NotInOwnCastle);
}

// A castle of the hero's that stands empty when a week ends is held against him again, with the garrison it
// first had; one with a single creature left in it stays his.
#[test]
fn an_empty_castle_is_lost_when_the_week_ends() {
    let week_out = |e: &mut Expedition| {
        let week = e.def().clock.week_ticks;
        let mut events = Vec::new();
        loop {
            events.extend(act(e, Action::WaitTick).unwrap());
            if e.ticks().is_multiple_of(week) {
                return events;
            }
        }
    };
    let mut e = in_the_keep(None);
    let events = week_out(&mut e);
    let lost = Event::CastleLost {
        castle: "meadow-keep".to_owned(),
    };
    assert!(events.contains(&lost), "{events:?}");
    assert!(!e.is_taken("meadow-keep"));
    assert_eq!(e.garrison("meadow-keep"), [("wolves".to_owned(), 5)]);

    let mut e = in_the_keep(None);
    act(&mut e, leave("wolves", 1)).unwrap();
    let events = week_out(&mut e);
    assert!(!events.contains(&lost), "{events:?}");
    assert!(e.is_taken("meadow-keep"));
    assert_eq!(e.garrison("meadow-keep"), [("wolves".to_owned(), 1)]);
}

// A lord caught stays caught when his castle is lost again: it is then a castle of monsters, taken without a
// contract and without a second reward.
#[test]
fn a_lost_castle_of_a_caught_lord_is_a_castle_of_monsters() {
    let mut e = armed(true);
    act(&mut e, Action::Move { dx: 1, dy: 0 }).unwrap();
    close_and_fight(&mut e);
    assert_eq!(e.standing().unwrap().villains, 1);
    go(&mut e, -1, 0, 1);
    let week = e.def().clock.week_ticks;
    while !e.ticks().is_multiple_of(week) {
        act(&mut e, Action::WaitTick).unwrap();
    }
    assert!(!e.is_taken("meadow-hall"));
    assert_eq!(
        e.standing().unwrap().villains,
        1,
        "he is not at large again"
    );
    let gold = e.gold();
    act(&mut e, Action::Move { dx: 1, dy: 0 }).unwrap();
    let events = close_and_fight(&mut e);
    assert!(e.is_taken("meadow-hall"), "{events:?}");
    assert_eq!((e.gold(), e.standing().unwrap().villains), (gold, 1));
}

// What the hero has left in his castles is part of the state.
#[test]
fn what_is_left_in_a_castle_is_saved() {
    let mut e = in_the_keep(None);
    act(&mut e, leave("wolves", 7)).unwrap();
    let def = Arc::clone(e.def_shared());
    let saved = e.to_canon();
    assert_eq!(
        Expedition::from_canon(Arc::clone(&def), &saved, None).unwrap(),
        e
    );
    let text = String::from_utf8(saved.to_bytes().unwrap()).unwrap();
    let changed = |old: &str, new: &str| {
        assert!(text.contains(old), "{old} in {text}");
        let value = Value::parse(&text.replacen(old, new, 1)).unwrap();
        Expedition::from_canon(Arc::clone(&def), &value, None).map(|_| ())
    };
    let own = r#""own":[{"count":7,"troop_id":"wolves"}]"#;
    assert!(
        changed(own, r#""own":[{"count":0,"troop_id":"wolves"}]"#).is_err(),
        "an empty stack"
    );
    assert!(
        changed(own, r#""own":[{"count":7,"troop_id":"griffins"}]"#).is_err(),
        "no troop"
    );
    let twice = r#""own":[{"count":7,"troop_id":"wolves"},{"count":1,"troop_id":"wolves"}]"#;
    assert!(changed(own, twice).is_err(), "a troop twice");
}
