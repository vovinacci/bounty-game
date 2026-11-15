//! What the hero's artefacts do in a battle (`docs/spec/07-RULES-REGISTER.md`, U-18): his side's blows do half as
//! much again, and blows on his side take a quarter less. Militia on Militia, no hero's morale: a blow is its roll
//! times the creatures that strike, halved.

use std::collections::BTreeMap;

use super::reference::{damage_events, stack};
use super::*;
use crate::content::Powers;

fn rules() -> Arc<Rules> {
    Arc::new(Rules::from_json(include_str!("../../../../../content/reference/rules.json")).unwrap())
}

fn fight(powers: Powers) -> Battle {
    let scenario = Scenario {
        id: "test".to_owned(),
        rng_state: 1,
        obstacles: vec![],
        hero_charges: BTreeMap::new(),
        leadership: None,
        spell_power: None,
        powers,
        stacks: vec![
            stack("p0", "militia", 10, 1, 2),
            stack("e0", "militia", 40, 2, 2),
        ],
    };
    Battle::start(rules(), &scenario, None).unwrap()
}

/// The damage of the hero's blow and of the answer, each beside what the blow is before any artefact.
fn exchange(powers: Powers) -> [(u64, u64); 2] {
    let mut b = fight(powers);
    let events = apply_forced(&mut b, melee("e0"), &[2, 1]).unwrap();
    let blows = damage_events(&events);
    let plain = |(roll, attackers, ..): (u32, u32, u64, u32, u32, u32, bool)| {
        u64::from(roll) * u64::from(attackers) / 2
    };
    [(blows[0].2, plain(blows[0])), (blows[1].2, plain(blows[1]))]
}

#[test]
fn the_heros_blows_do_half_as_much_again_and_blows_on_his_side_a_quarter_less() {
    let [(dealt, blow), (taken, answer)] = exchange(Powers::default());
    assert_eq!((dealt, taken), (blow, answer));
    assert_eq!((blow, answer), (10, 20), "2 x 10 and 1 x 40, halved");

    let more = Powers {
        more_damage: true,
        less_harm: false,
    };
    let [(dealt, blow), (taken, answer)] = exchange(more);
    assert_eq!(
        (dealt, taken),
        (blow + blow / 2, answer),
        "the enemy's blow is as it was"
    );

    let less = Powers {
        more_damage: false,
        less_harm: true,
    };
    let [(dealt, blow), (taken, answer)] = exchange(less);
    assert_eq!(
        (dealt, taken),
        (blow, answer / 4 * 3),
        "the hero's blow is as it was"
    );
}

#[test]
fn the_powers_are_saved_only_where_there_are_any() {
    let both = Powers {
        more_damage: true,
        less_harm: true,
    };
    let b = fight(both);
    assert_eq!(b.powers(), both);
    let saved = b.to_canon();
    assert_eq!(Battle::from_canon(rules(), &saved).unwrap(), b);
    let text = String::from_utf8(saved.to_bytes().unwrap()).unwrap();
    let written = r#""powers":{"less_harm":true,"more_damage":true}"#;
    assert!(text.contains(written), "{text}");
    let none = text.replacen(
        written,
        r#""powers":{"less_harm":false,"more_damage":false}"#,
        1,
    );
    let none = Value::parse(&none).unwrap();
    assert!(
        Battle::from_canon(rules(), &none).is_err(),
        "written and none"
    );

    let plain = fight(Powers::default());
    let text = String::from_utf8(plain.to_canon().to_bytes().unwrap()).unwrap();
    assert!(!text.contains("powers"));
}
