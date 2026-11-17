//! Towns beyond the first (`docs/spec/07-RULES-REGISTER.md`, U-17). The meadow with castles serves, with a port
//! east of the king's castle that sells Militia and Lightning and has word of the keep.

use super::*;
use crate::canon::Value;

const PORT: Cell = Cell { x: 5, y: 2 };

fn texts() -> (String, String) {
    let (expedition, map) = castles::texts(None);
    let object = r#"    {
      "id": "meadow-objective","#;
    let port = r#"    {
      "id": "meadow-port",
      "kind": "town",
      "x": 5,
      "y": 2
    },
"#;
    let king = r#"  "king": {"#;
    let listed = r#"  "more_towns": [
    {
      "object_id": "meadow-port",
      "stock": {
        "militia": 10
      },
      "price_per_creature": {
        "militia": 60
      },
      "intel_castle": "meadow-keep",
      "spell_for_sale": "lightning"
    }
  ],
  "king": {"#;
    assert!(map.contains(object) && expedition.contains(king));
    (
        expedition.replacen(king, listed, 1),
        map.replacen(object, &format!("{port}{object}"), 1),
    )
}

fn meadow() -> Expedition {
    let (expedition, map) = texts();
    Expedition::start(Arc::new(
        ExpeditionDef::from_texts(&expedition, &map, REFERENCE).unwrap(),
    ))
}

fn in_port() -> Expedition {
    let mut e = meadow();
    go(&mut e, 1, 0, 4);
    go(&mut e, 0, 1, 1);
    assert_eq!(e.hero(), PORT);
    e
}

fn militia(quantity: u32) -> Action {
    Action::Recruit {
        troop_id: "militia".to_owned(),
        quantity,
    }
}

#[test]
fn a_town_that_cannot_be_is_refused() {
    let e = meadow();
    assert_eq!(e.def().town_cells(), [Cell { x: 1, y: 1 }, PORT]);
    assert!(
        e.def().has_magic(),
        "a spell for sale in any town is a way to magic"
    );

    let (expedition, map) = texts();
    let refused = |old: &str, new: &str| {
        assert!(expedition.contains(old), "{old}");
        ExpeditionDef::from_texts(&expedition.replacen(old, new, 1), &map, REFERENCE).is_err()
    };
    let port = r#""object_id": "meadow-port""#;
    assert!(
        refused(port, r#""object_id": "meadow-town""#),
        "the first town again"
    );
    assert!(refused(port, r#""object_id": "meadow-castle""#), "no town");
    assert!(
        refused(r#""militia": 60"#, r#""wolves": 60"#),
        "stock without a price"
    );
    assert!(refused(r#""militia": 10"#, r#""gryphons": 10"#));
    assert!(refused(
        r#""spell_for_sale": "lightning""#,
        r#""spell_for_sale": "thunder""#
    ));
    assert!(refused(
        r#""intel_castle": "meadow-keep""#,
        r#""intel_castle": "meadow-castle""#
    ));
}

// Each town sells its own troops at its own prices from its own stock.
#[test]
fn each_town_sells_its_own() {
    let mut e = in_port();
    let wares: Vec<(String, u64, Option<u32>)> = e
        .on_sale_at(PORT)
        .into_iter()
        .map(|offer| (offer.troop_id, offer.price, offer.left))
        .collect();
    assert_eq!(wares, [("militia".to_owned(), 60, Some(10))]);
    let gold = e.gold();
    act(&mut e, militia(4)).unwrap();
    assert_eq!((e.gold(), e.stock_at(PORT, "militia")), (gold - 240, 6));
    refused(&mut e, militia(7), &Refused::OutOfStock { stock: 6 });
    let wolves = Action::Recruit {
        troop_id: "wolves".to_owned(),
        quantity: 1,
    };
    let not_sold = Refused::NotSold {
        troop_id: "wolves".to_owned(),
    };
    refused(&mut e, wolves, &not_sold);
    let first = Cell { x: 1, y: 1 };
    assert_eq!(e.stock_at(first, "wolves"), e.stock("wolves"));
    assert_eq!(
        e.stock_at(first, "militia"),
        0,
        "the first town never had any"
    );
}

// What any town does every town does: siege weapons and contracts; and each has its own spell and its own word.
#[test]
fn every_town_gives_contracts_sells_siege_weapons_its_spell_and_its_word() {
    let mut e = in_port();
    assert_eq!(
        e.spell_on_sale_at(PORT).map(|s| s.id.as_str()),
        Some("lightning")
    );
    assert_eq!(e.spell_on_sale(), None, "the first town sells none");
    act(&mut e, Action::BuySpell).unwrap();
    assert_eq!(e.charges("lightning"), 1);
    act(&mut e, Action::BuySiegeWeapons).unwrap();
    assert!(e.has_siege_weapons());
    let id = "meadow-hall".to_owned();
    act(&mut e, Action::TakeContract { id }).unwrap();
    assert_eq!(e.contract(), Some("meadow-hall"));
    assert_eq!(
        e.word_of_castle().map(|(castle, _)| castle),
        Some("meadow-keep")
    );

    go(&mut e, 0, -1, 1);
    refused(&mut e, Action::BuySpell, &Refused::NotInTown);
    refused(&mut e, Action::BuySiegeWeapons, &Refused::NotInTown);
    let mut first = meadow();
    refused(&mut first, Action::BuySpell, &Refused::NoSpellSold);
    assert_eq!(
        first.word_of_castle(),
        None,
        "the first town has word of none"
    );
}

// What each town has left is part of the state where there are towns beyond the first.
#[test]
fn the_towns_stock_is_saved() {
    let mut e = in_port();
    act(&mut e, militia(4)).unwrap();
    let def = Arc::clone(e.def_shared());
    let saved = e.to_canon();
    assert_eq!(
        Expedition::from_canon(Arc::clone(&def), &saved, None).unwrap(),
        e
    );
    let text = String::from_utf8(saved.to_bytes().unwrap()).unwrap();
    let port = r#""towns":{"meadow-port":{"militia":6}}"#;
    assert!(text.contains(port), "{text}");
    let changed = |new: &str| {
        let value = Value::parse(&text.replacen(port, new, 1)).unwrap();
        Expedition::from_canon(Arc::clone(&def), &value, None).map(|_| ())
    };
    assert!(
        changed(r#""towns":{"meadow-port":{"militia":11}}"#).is_err(),
        "more than it had"
    );
    assert!(changed(r#""towns":{"meadow-port":{"wolves":6}}"#).is_err());
    assert!(changed(r#""towns":{}"#).is_err(), "a town left out");

    let (plain, map) = castles::texts(None);
    let plain = Expedition::start(Arc::new(
        ExpeditionDef::from_texts(&plain, &map, REFERENCE).unwrap(),
    ));
    let plain = String::from_utf8(plain.to_canon().to_bytes().unwrap()).unwrap();
    assert!(!plain.contains("\"towns\""));
}
