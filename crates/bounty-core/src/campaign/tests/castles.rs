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
    let keep = r#""meadow-keep":{"garrison":[5],"taken":false}"#;
    assert!(
        changed(keep, r#""meadow-keep":{"garrison":[4],"taken":false}"#).is_err(),
        "shrunk"
    );
    assert!(
        changed(keep, r#""meadow-keep":{"garrison":[5],"taken":true}"#).is_err(),
        "taken and held"
    );

    let plain = Expedition::start(Arc::new(
        ExpeditionDef::from_texts(MEADOW, MEADOW_MAP, REFERENCE).unwrap(),
    ));
    let plain = String::from_utf8(plain.to_canon().to_bytes().unwrap()).unwrap();
    assert!(!plain.contains("castles") && !plain.contains("siege_weapons"));
}
