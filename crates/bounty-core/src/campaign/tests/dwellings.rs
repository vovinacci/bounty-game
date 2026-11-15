//! Dwellings, the week's creature and the king's castle as a recruiter (`docs/spec/07-RULES-REGISTER.md`, U-05
//! and U-07). The meadow serves, with a den of Wolves written into its texts.

use super::*;
use crate::canon::Value;

const DEN: Cell = Cell { x: 2, y: 2 };
const CASTLE: Cell = Cell { x: 4, y: 1 };

/// The meadow with a den of Wolves south-east of the town and, with a seed, weeks that have a creature. Under
/// seed 22 the first four weeks are those of the Wolves, the Dwarves, the Trolls and the Peasants.
fn texts(seed: Option<u32>) -> (String, String) {
    let object = r#"    {
      "id": "meadow-objective","#;
    let den = r#"    {
      "id": "meadow-den",
      "kind": "dwelling",
      "x": 2,
      "y": 2
    },
"#;
    assert!(MEADOW_MAP.contains(object));
    let map = MEADOW_MAP.replacen(object, &format!("{den}{object}"), 1);
    let king = r#"  "king": {"#;
    let dwellings = r#"  "dwellings": [
    {
      "object_id": "meadow-den",
      "troop": "wolves"
    }
  ],
  "king": {"#;
    assert!(MEADOW.contains(king));
    let mut expedition = MEADOW.replacen(king, dwellings, 1);
    if let Some(seed) = seed {
        let rule = r#""upkeep_rule": "tenth_of_recruit_cost""#;
        assert!(expedition.contains(rule));
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

fn at_the_den(seed: Option<u32>) -> Expedition {
    let mut e = meadow(seed);
    go(&mut e, 1, 0, 1);
    go(&mut e, 0, 1, 1);
    assert_eq!(e.hero(), DEN);
    e
}

fn buy(troop: &str, quantity: u32) -> Action {
    Action::Recruit {
        troop_id: troop.to_owned(),
        quantity,
    }
}

fn wait_out_the_week(e: &mut Expedition) -> Vec<Event> {
    let week = e.def().clock.week_ticks;
    let mut events = Vec::new();
    loop {
        events.extend(act(e, Action::WaitTick).unwrap());
        if e.ticks().is_multiple_of(week) || e.status() != Status::Adventure {
            return events;
        }
    }
}

#[test]
fn a_dwelling_starts_full_and_one_that_cannot_be_is_refused() {
    let e = meadow(None);
    assert_eq!(
        e.population("meadow-den"),
        150,
        "all the Wolves a den holds"
    );
    assert_eq!(e.def().dwelling_at(DEN).unwrap().troop, "wolves");
    assert_eq!(e.week_creature(), None, "no seed, no creature");

    let (expedition, map) = texts(None);
    let refused = |old: &str, new: &str| {
        assert!(expedition.contains(old), "{old}");
        ExpeditionDef::from_texts(&expedition.replacen(old, new, 1), &map, REFERENCE).is_err()
    };
    // Archers live in the castle, not in a dwelling; Griffins nowhere.
    assert!(refused(r#""troop": "wolves""#, r#""troop": "archers""#));
    assert!(refused(r#""troop": "wolves""#, r#""troop": "griffins""#));
    assert!(refused(
        r#""object_id": "meadow-den""#,
        r#""object_id": "meadow-town""#
    ));
    let twice = r#"    {
      "object_id": "meadow-den",
      "troop": "wolves"
    }"#;
    assert!(refused(twice, &format!("{twice},\n{twice}")));
}

// A dwelling sells its own troop at the rules' price until none are left; the leadership holds as in the town.
#[test]
fn a_dwelling_sells_its_troop_and_no_other() {
    let mut e = at_the_den(None);
    let gold = e.gold();
    let events = act(&mut e, buy("wolves", 20)).unwrap();
    assert_eq!(
        events,
        [Event::Recruited {
            troop_id: "wolves".to_owned(),
            quantity: 20,
            cost: 800
        }]
    );
    assert_eq!((e.gold(), e.population("meadow-den")), (gold - 800, 130));
    assert_eq!(
        e.stock("wolves"),
        40,
        "the town's Wolves are another matter"
    );

    let sprites = Refused::NotSold {
        troop_id: "sprites".to_owned(),
    };
    refused(&mut e, buy("sprites", 1), &sprites);
    refused(
        &mut e,
        buy("wolves", 131),
        &Refused::OutOfStock { stock: 130 },
    );
    // Fourteen more would be 34 Wolves, 102 health against a Knight's 100.
    let over = Refused::OverLeadership {
        health: 102,
        leadership: 100,
    };
    refused(&mut e, buy("wolves", 14), &over);
    // On the open road nothing is sold.
    go(&mut e, 0, 1, 1);
    refused(&mut e, buy("wolves", 1), &Refused::NotInTown);
}

// Every fourth week is the first troop's; any other week's creature comes from the seed and the week's number
// by the reference's sum, worked out here by hand for seed 22.
#[test]
fn a_week_has_the_creature_the_seed_gives_it() {
    let mut e = meadow(Some(22));
    assert_eq!(e.week_creature(), Some("peasants"), "week 0");
    let mut creatures = Vec::new();
    for _ in 0..3 {
        let events = wait_out_the_week(&mut e);
        let named = events.iter().find_map(|event| match event {
            Event::WeekCreature { week, troop_id } => Some((*week, troop_id.clone())),
            _ => None,
        });
        creatures.push(named.unwrap());
        assert_eq!(
            e.week_creature(),
            Some(creatures.last().unwrap().1.as_str())
        );
    }
    // Week 1: (22 xor 1) x 1664525 + 1013904223 = 1052188298, over 24 leaves 2: the third troop after the first.
    let expected = [(1, "wolves"), (2, "dwarves"), (3, "trolls")];
    let got: Vec<(u32, &str)> = creatures.iter().map(|(w, t)| (*w, t.as_str())).collect();
    assert_eq!(got, expected);
}

// Only the dwellings of the week's creature are full again; under seed 7 the first week is the Knights', and the
// den stays as it was left.
#[test]
fn the_weeks_creature_fills_its_dwellings_and_no_others() {
    let mut e = at_the_den(Some(22));
    act(&mut e, buy("wolves", 20)).unwrap();
    let events = wait_out_the_week(&mut e);
    let refilled = Event::DwellingRefilled {
        object: "meadow-den".to_owned(),
        population: 150,
    };
    assert!(events.contains(&refilled), "{events:?}");
    assert_eq!(e.population("meadow-den"), 150);

    let mut e = at_the_den(Some(7));
    act(&mut e, buy("wolves", 20)).unwrap();
    let events = wait_out_the_week(&mut e);
    let knights = Event::WeekCreature {
        week: 1,
        troop_id: "knights".to_owned(),
    };
    assert!(events.contains(&knights), "{events:?}");
    assert_eq!(e.population("meadow-den"), 130);
}

// In the week of the Peasants, the first troop of the rules, the army's Ghosts are Peasants: as many of them,
// in the same place of the army. The fourth week is one, and on the meadow also the last.
#[test]
fn ghosts_turn_into_peasants_in_the_week_of_the_peasants() {
    let (expedition, map) = texts(Some(22));
    let haunted = expedition
        .replacen(r#""troop_id": "sprites""#, r#""troop_id": "ghosts""#, 1)
        .replacen(r#""count": 20,"#, r#""count": 5,"#, 1);
    assert!(haunted.contains(r#""troop_id": "ghosts""#));
    let def = ExpeditionDef::from_texts(&haunted, &map, REFERENCE).unwrap();
    let mut e = Expedition::start(Arc::new(def));
    let mut events = Vec::new();
    for _ in 0..4 {
        assert_eq!(e.army()[0].troop_id, "ghosts");
        events = wait_out_the_week(&mut e);
    }
    let turned = Event::Turned {
        from: "ghosts".to_owned(),
        to: "peasants".to_owned(),
        count: 5,
    };
    assert!(events.contains(&turned), "{events:?}");
    let stack = &e.army()[0];
    assert_eq!(
        (stack.id.as_str(), stack.troop_id.as_str(), stack.count),
        ("p0", "peasants", 5)
    );
}

// The king's castle sells the troops that live in a castle, as many as are asked for, each only to a hero whose
// leadership holds six of its creatures.
#[test]
fn the_kings_castle_sells_its_troops_to_a_hero_who_can_lead_six() {
    let mut e = meadow(None);
    go(&mut e, 1, 0, 3);
    assert_eq!(e.hero(), CASTLE);
    let gold = e.gold();
    // Militia: two health, so twelve leadership is asked and a Knight has a hundred.
    act(&mut e, buy("militia", 10)).unwrap();
    assert_eq!(e.gold(), gold - 500);
    act(&mut e, buy("militia", 30)).unwrap();
    assert_eq!(e.gold(), gold - 2000, "no stock to run out of");
    // Knights: thirty-five health, six of them 210.
    let knights = Refused::NotYetOffered { leadership: 210 };
    refused(&mut e, buy("knights", 1), &knights);
    // Wolves live in a den, not here.
    let wolves = Refused::NotSold {
        troop_id: "wolves".to_owned(),
    };
    refused(&mut e, buy("wolves", 1), &wolves);
}

// What the dwellings have left is part of the state, where there are dwellings.
#[test]
fn what_a_dwelling_has_left_is_saved() {
    let mut e = at_the_den(Some(22));
    act(&mut e, buy("wolves", 20)).unwrap();
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
    assert!(
        changed(r#""meadow-den":130"#, r#""meadow-den":151"#).is_err(),
        "more than it holds"
    );
    assert!(
        changed(r#""meadow-den":130"#, r#""meadow-hut":130"#).is_err(),
        "another's dwelling"
    );

    let plain = Expedition::start(Arc::new(
        ExpeditionDef::from_texts(MEADOW, MEADOW_MAP, REFERENCE).unwrap(),
    ));
    let plain = String::from_utf8(plain.to_canon().to_bytes().unwrap()).unwrap();
    assert!(
        !plain.contains("dwellings"),
        "without dwellings nothing is written of them"
    );
}

// The week's creature grows in the bands that still stand, by what the troop grows in a week. Under seed 22 the
// first week is the Wolves', who grow by five: the pack of twelve is seventeen, and seventeen take the field.
#[test]
fn the_weeks_creature_grows_in_the_bands() {
    let mut e = meadow(Some(22));
    assert_eq!(e.band("meadow-pack"), [("wolves".to_owned(), 12)]);
    let events = wait_out_the_week(&mut e);
    let grew = Event::BandGrew {
        band: "meadow-pack".to_owned(),
        troop_id: "wolves".to_owned(),
        count: 17,
    };
    assert!(events.contains(&grew), "{events:?}");
    assert_eq!(e.band("meadow-pack"), [("wolves".to_owned(), 17)]);
    // The company has Archers and Sprites, neither the week's.
    let company = [("archers".to_owned(), 6), ("sprites".to_owned(), 30)];
    assert_eq!(e.band("meadow-company"), company);

    go(&mut e, 1, 0, 2);
    go(&mut e, 0, 1, 2);
    let pack = e.battle().unwrap().stack("e0").unwrap();
    assert_eq!((pack.troop_id.as_str(), pack.count), ("wolves", 17));

    // Without a seed the weeks have no creature, a band is what its file says, and nothing is written of it.
    let mut plain = meadow(None);
    wait_out_the_week(&mut plain);
    assert_eq!(plain.band("meadow-pack"), [("wolves".to_owned(), 12)]);
    let text = String::from_utf8(plain.to_canon().to_bytes().unwrap()).unwrap();
    assert!(!text.contains("bands"));
}

// How many a band has is part of the state where bands grow, and a band never shrinks.
#[test]
fn what_the_bands_have_grown_to_is_saved() {
    let mut e = meadow(Some(22));
    wait_out_the_week(&mut e);
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
    assert!(
        changed(r#""meadow-pack":[17]"#, r#""meadow-pack":[11]"#).is_err(),
        "shrunk"
    );
    assert!(
        changed(r#""meadow-pack":[17]"#, r#""meadow-pack":[17,3]"#).is_err(),
        "a stack more"
    );
    assert!(
        changed(r#""meadow-pack":[17]"#, r#""meadow-herd":[17]"#).is_err(),
        "another's band"
    );
}
