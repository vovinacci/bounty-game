//! The score (`docs/spec/07-RULES-REGISTER.md`, U-18): what the villains caught and the castles held earn, less
//! the hero's creatures that fell. The meadow with castles serves, scored.

use super::king::close_and_fight;
use super::*;
use crate::canon::Value;

fn texts() -> (String, String) {
    let (expedition, map) = castles::texts(None);
    let king = r#"  "king": {"#;
    assert!(expedition.contains(king));
    let scored = expedition.replacen(king, "  \"scored\": true,\n  \"king\": {", 1);
    (scored, map)
}

fn scored() -> Expedition {
    let (expedition, map) = texts();
    Expedition::start(Arc::new(
        ExpeditionDef::from_texts(&expedition, &map, REFERENCE).unwrap(),
    ))
}

/// The creatures of the hero's side that the blows of these events killed.
fn killed(events: &[Event]) -> u64 {
    let losses = events.iter().filter_map(|event| match event {
        Event::Battle(battle::Event::Damage { target, losses, .. }) if target.starts_with('p') => {
            Some(u64::from(*losses))
        }
        _ => None,
    });
    losses.sum()
}

// The rules say what a score is made of; an expedition says whether it has one.
#[test]
fn only_a_scored_expedition_has_a_score() {
    let e = scored();
    let scoring = e.def().rules.scoring.unwrap();
    assert_eq!(
        (
            scoring.per_villain,
            scoring.per_artefact,
            scoring.per_castle,
            scoring.per_fallen
        ),
        (500, 250, 100, 1)
    );
    assert_eq!((e.score(), e.fallen()), (Some(0), 0));

    let (plain, map) = castles::texts(None);
    let plain = Expedition::start(Arc::new(
        ExpeditionDef::from_texts(&plain, &map, REFERENCE).unwrap(),
    ));
    assert_eq!(plain.score(), None);
    let text = String::from_utf8(plain.to_canon().to_bytes().unwrap()).unwrap();
    assert!(
        !text.contains("fallen"),
        "nothing is counted where nothing is scored"
    );

    let (expedition, map) = texts();
    let king = r#"  "king": {
    "castle_object_id": "meadow-castle",
    "hero_class": "knight"
  },
"#;
    assert!(expedition.contains(king));
    let kingless = expedition.replacen(king, "", 1);
    assert!(ExpeditionDef::from_texts(&kingless, &map, REFERENCE).is_err());
}

// A castle held is a hundred, a villain caught five hundred, and every creature of the hero's that a blow kills
// is one off: the keep of monsters first, then the pack under its contract.
#[test]
fn castles_and_villains_earn_and_the_fallen_cost() {
    let mut e = scored();
    act(&mut e, Action::BuySiegeWeapons).unwrap();
    let id = "meadow-pack".to_owned();
    act(&mut e, Action::TakeContract { id }).unwrap();
    go(&mut e, 1, 0, 1);
    go(&mut e, 0, 1, 1);
    act(&mut e, Action::Move { dx: 0, dy: 1 }).unwrap();
    let siege = close_and_fight(&mut e);
    assert!(e.is_taken("meadow-keep"));
    let fell = killed(&siege);
    assert!(fell > 0 && fell < 100, "{fell}");
    assert_eq!((e.fallen(), e.score()), (fell, Some(100 - fell)));

    act(&mut e, Action::Move { dx: 1, dy: 0 }).unwrap();
    assert_eq!(e.status(), Status::Battle);
    let fight = close_and_fight(&mut e);
    let fell = fell + killed(&fight);
    assert_eq!(e.fallen(), fell);
    let villains = u64::from(e.standing().unwrap().villains);
    assert_eq!(villains, u64::from(e.is_cleared("meadow-pack")));
    let earned = 100 + 500 * villains;
    assert_eq!(e.score(), Some(earned.saturating_sub(fell)));

    let def = Arc::clone(e.def_shared());
    let saved = e.to_canon();
    assert_eq!(
        Expedition::from_canon(Arc::clone(&def), &saved, None).unwrap(),
        e
    );
    let text = String::from_utf8(saved.to_bytes().unwrap()).unwrap();
    let counted = format!(r#""fallen":{fell}"#);
    assert!(text.contains(&counted), "{text}");
    let more = Value::parse(&text.replacen(&counted, r#""fallen":100000"#, 1)).unwrap();
    let more = Expedition::from_canon(def, &more, None).unwrap();
    assert_eq!(more.score(), Some(0), "a score is never less than nothing");
}
