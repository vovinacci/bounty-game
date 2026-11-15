//! Chests (`docs/spec/07-RULES-REGISTER.md`, U-05): opened once, gold kept or shared out for leadership, or a
//! commission. The meadow serves, with two chests written into its texts.

use super::*;
use crate::canon::Value;

const PURSE: Cell = Cell { x: 3, y: 1 };
const WRIT: Cell = Cell { x: 2, y: 2 };

/// The meadow with a chest of 400 gold two steps east of the town and one with a commission of 30 south of
/// that road.
fn texts() -> (String, String) {
    let object = r#"    {
      "id": "meadow-objective","#;
    let chests = r#"    {
      "id": "meadow-purse",
      "kind": "chest",
      "x": 3,
      "y": 1
    },
    {
      "id": "meadow-writ",
      "kind": "chest",
      "x": 2,
      "y": 2
    },
"#;
    assert!(MEADOW_MAP.contains(object));
    let map = MEADOW_MAP.replacen(object, &format!("{chests}{object}"), 1);
    let king = r#"  "king": {"#;
    let listed = r#"  "chests": [
    {
      "object_id": "meadow-purse",
      "gold": 400
    },
    {
      "object_id": "meadow-writ",
      "commission": 30
    }
  ],
  "king": {"#;
    assert!(MEADOW.contains(king));
    (MEADOW.replacen(king, listed, 1), map)
}

fn meadow() -> Expedition {
    let (expedition, map) = texts();
    Expedition::start(Arc::new(
        ExpeditionDef::from_texts(&expedition, &map, REFERENCE).unwrap(),
    ))
}

fn at_the_purse() -> Expedition {
    let mut e = meadow();
    go(&mut e, 1, 0, 2);
    assert_eq!(e.hero(), PURSE);
    e
}

fn at_the_writ() -> Expedition {
    let mut e = meadow();
    go(&mut e, 1, 0, 1);
    go(&mut e, 0, 1, 1);
    assert_eq!(e.hero(), WRIT);
    e
}

#[test]
fn a_chest_holds_one_thing_and_one_that_cannot_be_is_refused() {
    let e = at_the_purse();
    assert_eq!(e.chest_here().unwrap().holds, Holds::Gold(400));
    assert_eq!(
        at_the_writ().chest_here().unwrap().holds,
        Holds::Commission(30)
    );
    assert_eq!(meadow().chest_here(), None, "the town is no chest");

    let (expedition, map) = texts();
    let refused = |old: &str, new: &str| {
        assert!(expedition.contains(old), "{old}");
        ExpeditionDef::from_texts(&expedition.replacen(old, new, 1), &map, REFERENCE).is_err()
    };
    assert!(
        refused(r#""gold": 400"#, r#""gold": 400, "commission": 5"#),
        "both"
    );
    assert!(refused(r#""gold": 400"#, r#""gold": 0"#), "nothing");
    assert!(
        refused(r#""commission": 30"#, r#""gems": 30"#),
        "something else"
    );
    assert!(refused(
        r#""object_id": "meadow-purse""#,
        r#""object_id": "meadow-town""#
    ));
    assert!(
        refused(
            r#""object_id": "meadow-writ""#,
            r#""object_id": "meadow-purse""#
        ),
        "twice"
    );
}

// Kept, the gold is in the purse and nothing else changes; the chest is open and stays so.
#[test]
fn a_chests_gold_can_be_kept() {
    let mut e = at_the_purse();
    let (gold, ticks) = (e.gold(), e.ticks());
    let events = act(&mut e, Action::KeepChest).unwrap();
    assert_eq!(
        events,
        [Event::ChestKept {
            object: "meadow-purse".to_owned(),
            gold: 400
        }]
    );
    assert_eq!(
        (e.gold(), e.ticks(), e.leadership()),
        (gold + 400, ticks, 100)
    );
    assert!(e.is_opened("meadow-purse") && e.chest_here().is_none());
    refused(&mut e, Action::KeepChest, &Refused::NoChest);
    refused(&mut e, Action::ShareChest, &Refused::NoChest);
}

// Shared out, every fifty gold is a point of leadership: 400 gold, eight points, and the purse as it was. The
// leadership is the hero's from then on: 36 Wolves of three health are 108, and one more is too many.
#[test]
fn a_chests_gold_can_be_shared_out_for_leadership() {
    let mut e = at_the_purse();
    let gold = e.gold();
    let events = act(&mut e, Action::ShareChest).unwrap();
    assert_eq!(
        events,
        [Event::ChestShared {
            object: "meadow-purse".to_owned(),
            leadership: 8
        }]
    );
    assert_eq!((e.gold(), e.leadership()), (gold, 108));
    go(&mut e, -1, 0, 2);
    let wolves = |quantity| Action::Recruit {
        troop_id: "wolves".to_owned(),
        quantity,
    };
    let over = Refused::OverLeadership {
        health: 111,
        leadership: 108,
    };
    refused(&mut e, wolves(37), &over);
    act(&mut e, wolves(36)).unwrap();

    // What the king gives is added to it: a General's 200, and the chest's eight.
    let mut e = at_the_purse();
    act(&mut e, Action::ShareChest).unwrap();
    e.cleared = ["meadow-pack", "meadow-company"].map(str::to_owned).into();
    e.standing.as_mut().unwrap().villains = 2;
    go(&mut e, 1, 0, 1);
    let events = act(&mut e, Action::Promote).unwrap();
    assert_eq!(
        events,
        [Event::Promoted {
            rank: "general".to_owned(),
            ranks: 1,
            leadership: 208,
            commission: 2000
        }]
    );
}

// A commission is so much more every week, and cannot be shared out.
#[test]
fn a_chest_can_hold_a_commission() {
    let mut e = at_the_writ();
    refused(&mut e, Action::ShareChest, &Refused::NothingToShare);
    let gold = e.gold();
    let events = act(&mut e, Action::KeepChest).unwrap();
    assert_eq!(
        events,
        [Event::ChestCommission {
            object: "meadow-writ".to_owned(),
            commission: 30
        }]
    );
    assert_eq!((e.gold(), e.weekly_income()), (gold, 1030));
    let week = e.def().clock.week_ticks;
    let mut settled = Vec::new();
    while e.ticks() < week {
        settled.extend(act(&mut e, Action::WaitTick).unwrap());
    }
    let income = settled.iter().find_map(|event| match event {
        Event::WeekSettled { income, .. } => Some(*income),
        _ => None,
    });
    assert_eq!(income, Some(1030));
}

// Which chests are open, and what was done with each, is part of the state where there are chests.
#[test]
fn the_open_chests_are_saved() {
    let mut e = at_the_purse();
    act(&mut e, Action::ShareChest).unwrap();
    let def = Arc::clone(e.def_shared());
    let saved = e.to_canon();
    let loaded = Expedition::from_canon(Arc::clone(&def), &saved, None).unwrap();
    assert_eq!(loaded, e);
    assert_eq!(loaded.leadership(), 108);

    let text = String::from_utf8(saved.to_bytes().unwrap()).unwrap();
    let changed = |old: &str, new: &str| {
        assert!(text.contains(old), "{old} in {text}");
        let value = Value::parse(&text.replacen(old, new, 1)).unwrap();
        Expedition::from_canon(Arc::clone(&def), &value, None).map(|_| ())
    };
    let open = r#""meadow-purse":"shared""#;
    assert_eq!(
        changed(open, r#""meadow-purse":"kept""#),
        Ok(()),
        "another choice is a state too"
    );
    assert!(changed(open, r#""meadow-purse":"lost""#).is_err());
    assert!(
        changed(open, r#""meadow-writ":"shared""#).is_err(),
        "a commission shared out"
    );
    assert!(
        changed(open, r#""meadow-sack":"kept""#).is_err(),
        "another's chest"
    );

    let plain = Expedition::start(Arc::new(
        ExpeditionDef::from_texts(MEADOW, MEADOW_MAP, REFERENCE).unwrap(),
    ));
    let plain = String::from_utf8(plain.to_canon().to_bytes().unwrap()).unwrap();
    assert!(
        !plain.contains("chests"),
        "without chests nothing is written of them"
    );
}
