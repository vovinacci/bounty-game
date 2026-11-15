//! The hero's magic (`docs/spec/07-RULES-REGISTER.md`, U-05): whether he casts, his spell power, the room he has
//! for spells and the charges he holds; the alcove, the town's spell, and the three chests of magic. The meadow
//! serves, with all of them written into its texts.

use super::*;
use crate::canon::Value;

const ALCOVE: Cell = Cell { x: 2, y: 1 };

/// The meadow with an alcove east of the town that teaches for 2500, a town that sells Bridge, and three
/// chests south of them: one of spell power, one of room for a spell, one of two charges of Fireball.
fn texts() -> (String, String) {
    let object = r#"    {
      "id": "meadow-objective","#;
    let objects = r#"    {
      "id": "meadow-alcove",
      "kind": "alcove",
      "x": 2,
      "y": 1
    },
    {
      "id": "meadow-tome",
      "kind": "chest",
      "x": 1,
      "y": 2
    },
    {
      "id": "meadow-satchel",
      "kind": "chest",
      "x": 2,
      "y": 2
    },
    {
      "id": "meadow-scroll",
      "kind": "chest",
      "x": 3,
      "y": 2
    },
"#;
    assert!(MEADOW_MAP.contains(object));
    let map = MEADOW_MAP.replacen(object, &format!("{objects}{object}"), 1);
    let king = r#"  "king": {"#;
    let listed = r#"  "alcove": {
    "object_id": "meadow-alcove",
    "cost": 2500
  },
  "chests": [
    {
      "object_id": "meadow-tome",
      "spell_power": 1
    },
    {
      "object_id": "meadow-satchel",
      "max_spells": 1
    },
    {
      "object_id": "meadow-scroll",
      "spell": {
        "id": "fireball",
        "count": 2
      }
    }
  ],
  "king": {"#;
    let town = r#"  "town": {
    "object_id": "meadow-town","#;
    let selling = r#"  "town": {
    "object_id": "meadow-town",
    "spell_for_sale": "bridge","#;
    assert!(MEADOW.contains(king) && MEADOW.contains(town));
    let expedition = MEADOW.replacen(king, listed, 1);
    (expedition.replacen(town, selling, 1), map)
}

fn started(expedition: &str, map: &str) -> Expedition {
    Expedition::start(Arc::new(
        ExpeditionDef::from_texts(expedition, map, REFERENCE).unwrap(),
    ))
}

fn meadow() -> Expedition {
    let (expedition, map) = texts();
    started(&expedition, &map)
}

/// The meadow with one thing in its file written otherwise.
fn meadow_with(old: &str, new: &str) -> Expedition {
    let (expedition, map) = texts();
    assert!(expedition.contains(old), "{old}");
    started(&expedition.replacen(old, new, 1), &map)
}

fn plain() -> Expedition {
    started(MEADOW, MEADOW_MAP)
}

// A Knight cannot cast, has the spell power and the room of his first rank, and holds nothing. Where nothing
// leads to magic there is no state of it.
#[test]
fn magic_begins_with_the_rank_and_only_where_something_leads_to_it() {
    let e = meadow();
    let magic = e.magic().unwrap();
    assert!(!magic.knows && magic.charges.iter().all(|n| *n == 0));
    assert_eq!(
        magic.charges.len(),
        14,
        "a place for each spell of the book"
    );
    assert_eq!(
        (e.spell_power(), e.spell_limit(), e.spells_held()),
        (1, 2, 0)
    );
    assert_eq!(
        e.spell_on_sale().map(|s| (s.id.as_str(), s.cost)),
        Some(("bridge", 100))
    );

    let plain = plain();
    assert_eq!(plain.magic(), None);
    assert_eq!((plain.spell_on_sale(), plain.spells_held()), (None, 0));
}

#[test]
fn magic_that_cannot_be_is_refused() {
    let (expedition, map) = texts();
    let refused = |old: &str, new: &str| {
        assert!(expedition.contains(old), "{old}");
        ExpeditionDef::from_texts(&expedition.replacen(old, new, 1), &map, REFERENCE).is_err()
    };
    assert!(refused(r#""cost": 2500"#, r#""cost": 0"#), "taught free");
    assert!(
        refused(
            r#""object_id": "meadow-alcove""#,
            r#""object_id": "meadow-town""#
        ),
        "no alcove there"
    );
    assert!(
        refused(
            r#""spell_for_sale": "bridge""#,
            r#""spell_for_sale": "tunnel""#
        ),
        "a spell the rules lack"
    );
    assert!(refused(r#""id": "fireball""#, r#""id": "fire""#));
    let charges = "\"id\": \"fireball\",\n        \"count\": 2";
    let of = |count: u32| charges.replacen('2', &count.to_string(), 1);
    assert!(refused(charges, &of(0)), "no charge");
    assert!(refused(charges, &of(100)), "too many");
    assert!(!refused(charges, &of(99)), "the most a chest gives");
    assert!(refused(r#""spell_power": 1"#, r#""spell_power": 0"#));
    assert!(refused(
        r#""max_spells": 1"#,
        r#""max_spells": 1, "gold": 5"#
    ));
    let king = r#"  "king": {
    "castle_object_id": "meadow-castle",
    "hero_class": "knight"
  },
"#;
    assert!(
        refused(king, ""),
        "nobody gives the room for spells without a king"
    );
}

// The alcove teaches once, for its price, and the last coin pays for it.
#[test]
fn the_alcove_teaches_a_hero_who_can_pay_once() {
    let mut e = meadow();
    refused(&mut e, Action::LearnMagic, &Refused::NoAlcove);
    go(&mut e, 1, 0, 1);
    assert_eq!(
        (e.hero(), e.alcove_here().map(|a| a.cost)),
        (ALCOVE, Some(2500))
    );
    let ticks = e.ticks();
    assert_eq!(
        act(&mut e, Action::LearnMagic).unwrap(),
        [Event::MagicLearned { cost: 2500 }]
    );
    assert_eq!((e.gold(), e.ticks()), (500, ticks), "it takes no time");
    assert!(e.magic().unwrap().knows && e.alcove_here().is_none());
    refused(&mut e, Action::LearnMagic, &Refused::NoAlcove);

    let mut poor = meadow_with(r#""cost": 2500"#, r#""cost": 3001"#);
    go(&mut poor, 1, 0, 1);
    let short = Refused::CannotAfford {
        cost: 3001,
        gold: 3000,
    };
    refused(&mut poor, Action::LearnMagic, &short);
    let mut exact = meadow_with(r#""cost": 2500"#, r#""cost": 3000"#);
    go(&mut exact, 1, 0, 1);
    act(&mut exact, Action::LearnMagic).unwrap();
    assert_eq!(exact.gold(), 0);

    refused(&mut plain(), Action::LearnMagic, &Refused::NoAlcove);
}

// The town sells its spell a charge at a time while there is room, to a hero who cannot cast as to one who
// can; every charge takes one place of the room.
#[test]
fn the_town_sells_its_spell_while_there_is_room() {
    let mut e = meadow();
    let bought = |held| Event::SpellBought {
        spell: "bridge".to_owned(),
        cost: 100,
        held,
    };
    assert_eq!(act(&mut e, Action::BuySpell).unwrap(), [bought(1)]);
    assert_eq!(act(&mut e, Action::BuySpell).unwrap(), [bought(2)]);
    assert_eq!(
        (e.gold(), e.charges("bridge"), e.spells_held()),
        (2800, 2, 2)
    );
    refused(
        &mut e,
        Action::BuySpell,
        &Refused::NoRoomForSpell { limit: 2 },
    );
    go(&mut e, 1, 0, 1);
    refused(&mut e, Action::BuySpell, &Refused::NotInTown);

    refused(&mut plain(), Action::BuySpell, &Refused::NoSpellSold);
}

// The purse must hold more than the price: a hero with exactly the price is turned away.
#[test]
fn a_spell_is_sold_only_for_more_gold_than_its_price() {
    let mut exact = meadow_with(r#""gold": 3000"#, r#""gold": 100"#);
    let short = Refused::CannotAfford {
        cost: 100,
        gold: 100,
    };
    refused(&mut exact, Action::BuySpell, &short);
    let mut more = meadow_with(r#""gold": 3000"#, r#""gold": 101"#);
    act(&mut more, Action::BuySpell).unwrap();
    assert_eq!(more.gold(), 1);
}

// Spell power and room are the hero's for good; a spell's charges are his whatever room he has, and count
// against it afterwards. None of the three can be shared out.
#[test]
fn chests_hold_spell_power_room_for_spells_and_spells() {
    let mut e = meadow();
    act(&mut e, Action::BuySpell).unwrap();
    act(&mut e, Action::BuySpell).unwrap();
    go(&mut e, 0, 1, 1);
    assert_eq!(e.chest_here().unwrap().holds, Holds::Power(1));
    refused(&mut e, Action::ShareChest, &Refused::NothingToShare);
    let power = Event::ChestPower {
        object: "meadow-tome".to_owned(),
        power: 1,
    };
    assert_eq!(act(&mut e, Action::KeepChest).unwrap(), [power]);
    assert_eq!((e.spell_power(), e.spell_limit()), (2, 2));

    go(&mut e, 1, 0, 1);
    refused(&mut e, Action::ShareChest, &Refused::NothingToShare);
    let room = Event::ChestLimit {
        object: "meadow-satchel".to_owned(),
        limit: 1,
    };
    assert_eq!(act(&mut e, Action::KeepChest).unwrap(), [room]);
    assert_eq!((e.spell_power(), e.spell_limit()), (2, 3));

    go(&mut e, 1, 0, 1);
    let fireball = e.def().rules.book.iter().position(|s| s.id == "fireball");
    let holds = Holds::Spell {
        spell: fireball.unwrap(),
        count: 2,
    };
    assert_eq!(e.chest_here().unwrap().holds, holds);
    refused(&mut e, Action::ShareChest, &Refused::NothingToShare);
    let scroll = Event::ChestSpell {
        object: "meadow-scroll".to_owned(),
        spell: "fireball".to_owned(),
        count: 2,
    };
    assert_eq!(act(&mut e, Action::KeepChest).unwrap(), [scroll]);
    assert_eq!(
        (e.charges("fireball"), e.spells_held(), e.spell_limit()),
        (2, 4, 3),
        "a chest gives beyond the room"
    );
    refused(&mut e, Action::KeepChest, &Refused::NoChest);

    go(&mut e, -1, 0, 2);
    go(&mut e, 0, -1, 1);
    refused(
        &mut e,
        Action::BuySpell,
        &Refused::NoRoomForSpell { limit: 3 },
    );
}

// Each rank adds its spell power and its room to what the hero has: a General has the Knight's and his own.
#[test]
fn a_rank_adds_its_spell_power_and_room() {
    let mut e = meadow();
    e.cleared = ["meadow-pack", "meadow-company"]
        .iter()
        .map(|band| (*band).to_owned())
        .collect();
    e.standing.as_mut().unwrap().villains = 2;
    go(&mut e, 1, 0, 3);
    act(&mut e, Action::Promote).unwrap();
    assert_eq!(e.rank().unwrap().id, "general");
    assert_eq!((e.spell_power(), e.spell_limit()), (2, 5));
}

// Whether the hero casts and what he holds is part of the state where there is magic, and only there.
#[test]
fn magic_is_saved_and_magic_that_cannot_be_is_refused() {
    let mut e = meadow();
    act(&mut e, Action::BuySpell).unwrap();
    act(&mut e, Action::BuySpell).unwrap();
    go(&mut e, 1, 0, 1);
    act(&mut e, Action::LearnMagic).unwrap();
    go(&mut e, 0, 1, 1);
    go(&mut e, 1, 0, 1);
    act(&mut e, Action::KeepChest).unwrap();
    let def = Arc::clone(e.def_shared());
    let saved = e.to_canon();
    let loaded = Expedition::from_canon(Arc::clone(&def), &saved, None).unwrap();
    assert_eq!(loaded, e);

    let text = String::from_utf8(saved.to_bytes().unwrap()).unwrap();
    let magic = r#""magic":{"book":{"bridge":2,"fireball":2},"knows":true}"#;
    assert!(text.contains(magic), "{text}");
    let changed = |old: &str, new: &str| {
        assert!(text.contains(old), "{old} in {text}");
        let value = Value::parse(&text.replacen(old, new, 1)).unwrap();
        Expedition::from_canon(Arc::clone(&def), &value, None).map(|_| ())
    };
    assert_eq!(changed(r#""knows":true"#, r#""knows":false"#), Ok(()));
    assert_eq!(changed(r#""bridge":2,"#, ""), Ok(()), "spells are cast");
    assert!(changed(r#""bridge":2"#, r#""bridge":0"#).is_err(), "none");
    assert!(
        changed(r#""bridge":2"#, r#""bridge":3"#).is_err(),
        "more than the room and the chest allow"
    );
    assert!(
        changed(r#""fireball":2"#, r#""fireball":3"#).is_err(),
        "more than the chest held of a spell nobody sells"
    );
    assert!(
        changed(r#""bridge":2"#, r#""clone":2"#).is_err(),
        "a spell from nowhere"
    );
    assert!(changed(r#""bridge":2"#, r#""tunnel":2"#).is_err());

    let taught = meadow_with(
        r#"  "alcove": {
    "object_id": "meadow-alcove",
    "cost": 2500
  },
"#,
        "",
    );
    let def = Arc::clone(taught.def_shared());
    let text = String::from_utf8(taught.to_canon().to_bytes().unwrap()).unwrap();
    let knows = text.replacen(r#""knows":false"#, r#""knows":true"#, 1);
    assert_ne!(knows, text);
    let value = Value::parse(&knows).unwrap();
    assert!(
        Expedition::from_canon(def, &value, None).is_err(),
        "nothing could have taught him"
    );

    let plain = String::from_utf8(plain().to_canon().to_bytes().unwrap()).unwrap();
    assert!(
        !plain.contains("magic"),
        "without magic nothing is written of it"
    );
}

/// The hero beside the pack with the scroll's two charges of Fireball, taught at the alcove or not.
fn before_the_pack(taught: bool) -> Expedition {
    let mut e = meadow();
    go(&mut e, 1, 0, 1);
    if taught {
        act(&mut e, Action::LearnMagic).unwrap();
    }
    go(&mut e, 0, 1, 1);
    go(&mut e, 1, 0, 1);
    act(&mut e, Action::KeepChest).unwrap();
    act(&mut e, Action::Move { dx: 0, dy: 1 }).unwrap();
    assert_eq!(e.status(), Status::Battle);
    e
}

// A hero who casts takes his charges of the spells of battle and his spell power into a battle. A charge spent
// there is spent for good, however the battle ends: Fireball at a power of 1 is 25, eight Wolves of three health.
#[test]
fn a_hero_who_casts_takes_his_spells_into_battle() {
    let mut e = before_the_pack(true);
    let battle = e.battle().unwrap();
    assert_eq!(battle.spell_power(), Some(1));
    assert_eq!(battle.hero_charges().get("fireball"), Some(&2));
    let pack = battle.stacks().iter().find(|s| s.side == Side::Enemy);
    let pack = pack.unwrap().id.clone();
    let cast = Action::Battle(battle::Command {
        revision: battle.revision(),
        actor: battle.active().unwrap().stack.clone(),
        action: battle::Action::Cast {
            spell: "fireball".to_owned(),
            target: pack.clone(),
        },
    });
    act(&mut e, cast).unwrap();
    assert_eq!(e.battle().unwrap().stack(&pack).unwrap().count, 4);
    assert_eq!(e.charges("fireball"), 1, "spent at once");

    let def = Arc::clone(e.def_shared());
    let (saved, fought) = (e.to_canon(), e.battle().unwrap().to_canon());
    let loaded = Expedition::from_canon(Arc::clone(&def), &saved, Some(&fought)).unwrap();
    assert_eq!(loaded, e);
    let text = String::from_utf8(saved.to_bytes().unwrap()).unwrap();
    let other = text.replacen(r#""book":{"fireball":1}"#, r#""book":{"fireball":2}"#, 1);
    assert_ne!(other, text);
    let other = Value::parse(&other).unwrap();
    assert!(
        Expedition::from_canon(def, &other, Some(&fought)).is_err(),
        "the hero's charges are the battle's"
    );

    act(&mut e, Action::Surrender).unwrap();
    assert_eq!(
        e.charges("fireball"),
        1,
        "a battle given up gives nothing back"
    );
}

// A hero who cannot cast holds his charges and fights without them.
#[test]
fn a_hero_who_cannot_cast_fights_without_magic() {
    let e = before_the_pack(false);
    let battle = e.battle().unwrap();
    assert_eq!(battle.spell_power(), None);
    assert!(battle.hero_charges().is_empty());
    assert_eq!(e.charges("fireball"), 2);
}
