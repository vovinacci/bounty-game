//! Artefacts (`docs/spec/07-RULES-REGISTER.md`, U-18): found by stepping onto them, once, each with its power.
//! The meadow with magic serves, scored, with seven artefacts laid out round the town.

use super::*;
use crate::canon::Value;
use crate::content::{Power, Powers};

pub(super) const LAID: [(&str, u32, u32); 7] = [
    ("crown", 3, 1),
    ("articles", 5, 1),
    ("amulet", 4, 2),
    ("ring", 5, 2),
    ("book", 1, 3),
    ("sword", 4, 3),
    ("shield", 5, 3),
];

pub(super) fn texts() -> (String, String) {
    let (expedition, map) = magic::texts();
    let object = r#"    {
      "id": "meadow-objective","#;
    let king = r#"  "king": {"#;
    assert!(map.contains(object) && expedition.contains(king));
    let object_of = |(id, x, y): (&str, u32, u32)| {
        format!(
            "    {{\n      \"id\": \"meadow-{id}\",\n      \"kind\": \"artefact\",\n      \"x\": {x},\n      \"y\": {y}\n    }},\n"
        )
    };
    let entry_of = |(id, _, _): (&str, u32, u32)| {
        format!(
            "    {{\n      \"object_id\": \"meadow-{id}\",\n      \"artefact\": \"{id}\"\n    }}"
        )
    };
    let objects: String = LAID.into_iter().map(object_of).collect();
    let listed: Vec<String> = LAID.into_iter().map(entry_of).collect();
    let listed = format!(
        "  \"scored\": true,\n  \"artefacts\": [\n{}\n  ],\n{king}",
        listed.join(",\n")
    );
    (
        expedition.replacen(king, &listed, 1),
        map.replacen(object, &format!("{objects}{object}"), 1),
    )
}

fn meadow() -> Expedition {
    let (expedition, map) = texts();
    Expedition::start(Arc::new(
        ExpeditionDef::from_texts(&expedition, &map, REFERENCE).unwrap(),
    ))
}

fn found(id: &str) -> Event {
    Event::ArtefactFound {
        object: format!("meadow-{id}"),
        artefact: id.to_owned(),
    }
}

// The rules have the reference's eight artefacts; an expedition lays out those it wants, each once.
#[test]
fn artefacts_are_the_rules_and_laid_out_once() {
    let e = meadow();
    let rules = &e.def().rules;
    let ids: Vec<&str> = rules.artefacts.iter().map(|a| a.id.as_str()).collect();
    let all = [
        "sword", "shield", "crown", "articles", "amulet", "ring", "book", "anchor",
    ];
    assert_eq!(ids, all);
    assert_eq!(rules.artefacts[2].power, Power::DoubleLeadership);
    assert_eq!(e.def().artefacts.len(), 7);
    assert_eq!((e.artefacts_found().len(), e.score()), (0, Some(0)));

    let (expedition, map) = texts();
    let refused = |old: &str, new: &str| {
        assert!(expedition.contains(old), "{old}");
        ExpeditionDef::from_texts(&expedition.replacen(old, new, 1), &map, REFERENCE).is_err()
    };
    assert!(refused(
        r#""artefact": "crown""#,
        r#""artefact": "sceptre""#
    ));
    assert!(
        refused(r#""artefact": "crown""#, r#""artefact": "ring""#),
        "twice"
    );
    assert!(refused(
        r#""object_id": "meadow-crown""#,
        r#""object_id": "meadow-town""#
    ));
    assert!(refused(
        r#""object_id": "meadow-crown""#,
        r#""object_id": "meadow-ring""#
    ));
}

// The Crown doubles the leadership the hero has when he steps onto it; the Articles add 2000 a week. Each is
// found once and is 250 of the score.
#[test]
fn the_crown_doubles_leadership_and_the_articles_raise_the_commission() {
    let mut e = meadow();
    go(&mut e, 1, 0, 1);
    let events = act(&mut e, Action::Move { dx: 1, dy: 0 }).unwrap();
    assert_eq!(events.last(), Some(&found("crown")));
    assert!(e.is_found("meadow-crown") && e.has_power(Power::DoubleLeadership));
    assert_eq!((e.leadership(), e.score()), (200, Some(250)));
    go(&mut e, -1, 0, 1);
    let again = act(&mut e, Action::Move { dx: 1, dy: 0 }).unwrap();
    assert!(!again.contains(&found("crown")), "once");
    assert_eq!(e.leadership(), 200);

    assert_eq!(e.weekly_income(), 1000);
    go(&mut e, 1, 0, 2);
    assert_eq!(e.artefacts_found(), ["articles", "crown"]);
    assert_eq!((e.weekly_income(), e.score()), (3000, Some(500)));
}

// The Amulet doubles the spell power and the Ring the room for spells, as they stand. A chest of room opened
// with the Ring gives twice; a chest of spell power gives its one point all the same, as the reference has it.
#[test]
fn the_amulet_and_the_ring_double_the_heros_magic() {
    let mut e = meadow();
    assert_eq!((e.spell_power(), e.spell_limit()), (1, 2));
    go(&mut e, 1, 0, 3);
    go(&mut e, 0, 1, 1);
    assert_eq!(e.artefacts_found(), ["amulet", "crown"]);
    assert_eq!((e.spell_power(), e.spell_limit()), (2, 2));
    go(&mut e, 1, 0, 1);
    assert_eq!((e.spell_power(), e.spell_limit()), (2, 4));

    go(&mut e, -1, 0, 3);
    let room = Event::ChestLimit {
        object: "meadow-satchel".to_owned(),
        limit: 2,
    };
    assert_eq!(act(&mut e, Action::KeepChest).unwrap(), [room]);
    assert_eq!(e.spell_limit(), 6);
    go(&mut e, -1, 0, 1);
    act(&mut e, Action::KeepChest).unwrap();
    assert_eq!(e.spell_power(), 3);

    // The other way round comes to the same: the chest first, then the Ring doubles what it gave too.
    let mut e = meadow();
    go(&mut e, 1, 0, 1);
    go(&mut e, 0, 1, 1);
    act(&mut e, Action::KeepChest).unwrap();
    assert_eq!(e.spell_limit(), 3);
    go(&mut e, 1, 0, 3);
    assert!(e.has_power(Power::DoubleMaxSpells));
    assert_eq!(e.spell_limit(), 6);
}

// The Sword and the Shield go into the hero's battles; the Book does nothing but count.
#[test]
fn the_sword_and_the_shield_are_taken_into_battle() {
    let mut e = meadow();
    go(&mut e, 0, 1, 2);
    assert_eq!(e.artefacts_found(), ["book"]);
    assert_eq!(
        (e.leadership(), e.weekly_income(), e.score()),
        (100, 1000, Some(250))
    );
    go(&mut e, 0, -1, 1);
    go(&mut e, 1, 0, 4);
    go(&mut e, 0, 1, 1);
    assert_eq!(e.artefacts_found(), ["amulet", "book", "ring", "shield"]);
    go(&mut e, -1, 0, 1);
    assert!(e.has_power(Power::IncreasedDamage));
    act(&mut e, Action::Move { dx: -1, dy: 0 }).unwrap();
    assert_eq!(e.status(), Status::Battle);
    let both = Powers {
        more_damage: true,
        less_harm: true,
    };
    assert_eq!(e.battle().unwrap().powers(), both);

    let def = Arc::clone(e.def_shared());
    let (saved, fought) = (e.to_canon(), e.battle().unwrap().to_canon());
    let loaded = Expedition::from_canon(Arc::clone(&def), &saved, Some(&fought)).unwrap();
    assert_eq!(loaded, e);
    let text = String::from_utf8(saved.to_bytes().unwrap()).unwrap();
    let other = text.replacen(r#""shield":0,"#, "", 1);
    assert_ne!(other, text);
    let other = Value::parse(&other).unwrap();
    assert!(
        Expedition::from_canon(def, &other, Some(&fought)).is_err(),
        "the battle's artefacts are the hero's"
    );
}

// Which artefacts are found, and what the doubling ones added, is part of the state where artefacts lie.
#[test]
fn the_artefacts_found_are_saved() {
    let mut e = meadow();
    go(&mut e, 1, 0, 2);
    let def = Arc::clone(e.def_shared());
    let saved = e.to_canon();
    assert_eq!(
        Expedition::from_canon(Arc::clone(&def), &saved, None).unwrap(),
        e
    );
    let text = String::from_utf8(saved.to_bytes().unwrap()).unwrap();
    let crown = r#""artefacts":{"crown":100}"#;
    assert!(text.contains(crown), "{text}");
    let changed = |new: &str| {
        let value = Value::parse(&text.replacen(crown, new, 1)).unwrap();
        Expedition::from_canon(Arc::clone(&def), &value, None).map(|_| ())
    };
    assert_eq!(changed(r#""artefacts":{}"#), Ok(()));
    assert!(
        changed(r#""artefacts":{"crown":101}"#).is_err(),
        "more than he had"
    );
    assert!(
        changed(r#""artefacts":{"book":1}"#).is_err(),
        "the Book adds nothing"
    );
    assert!(
        changed(r#""artefacts":{"anchor":0}"#).is_err(),
        "not laid out here"
    );

    let plain = Expedition::start(Arc::new(
        ExpeditionDef::from_texts(MEADOW, MEADOW_MAP, REFERENCE).unwrap(),
    ));
    let plain = String::from_utf8(plain.to_canon().to_bytes().unwrap()).unwrap();
    assert!(!plain.contains("artefacts"));
}
