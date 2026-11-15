//! The hero's spells of battle under the reference rules (`docs/spec/07-RULES-REGISTER.md`, U-13). The numbers are
//! worked out by hand: a spell's damage is its number times the hero's spell power, and nothing else.

use std::collections::BTreeMap;

use super::reference::stack;
use super::*;
use crate::content::StackStart;

fn rules() -> Arc<Rules> {
    Arc::new(Rules::from_json(include_str!("../../../../../content/reference/rules.json")).unwrap())
}

fn scenario(power: Option<u32>, charges: &[(&str, u32)], stacks: Vec<StackStart>) -> Scenario {
    let charges = charges.iter().map(|(spell, n)| ((*spell).to_owned(), *n));
    Scenario {
        id: "test".to_owned(),
        rng_state: 1,
        obstacles: vec![],
        hero_charges: charges.collect::<BTreeMap<_, _>>(),
        leadership: None,
        spell_power: power,
        powers: Powers::default(),
        stacks,
    }
}

/// A battle whose hero has this spell power and these charges.
fn magic(power: u32, charges: &[(&str, u32)], stacks: Vec<StackStart>) -> Battle {
    Battle::start(rules(), &scenario(Some(power), charges, stacks), None).unwrap()
}

fn cast_to(spell: &str, target: &str, x: u32, y: u32) -> Action {
    Action::CastTo {
        spell: spell.to_owned(),
        target: target.to_owned(),
        to: cell(x, y),
    }
}

fn wrong(target: &str) -> RuleError {
    RuleError::WrongSpellTarget {
        target: target.to_owned(),
    }
}

fn two_sides() -> Vec<StackStart> {
    vec![
        stack("p0", "militia", 10, 1, 2),
        stack("e0", "militia", 30, 4, 2),
    ]
}

// Fireball is 25 a point of spell power, Lightning 10: with a power of 2, 50 and 20 on Militia of two health.
// A cast is the hero's, once a round: it does not end the stack's turn, and nobody strikes back.
#[test]
fn fireball_and_lightning_do_their_number_times_the_spell_power() {
    let mut b = magic(2, &[("fireball", 1), ("lightning", 2)], two_sides());
    assert_eq!(b.spell_power(), Some(2));
    let events = act(&mut b, cast("fireball", "e0")).unwrap();
    assert!(matches!(
        &events[..],
        [
            Event::SpellCast { spell, target },
            Event::Damage { attacker, damage: 50, losses: 25, retaliation: false, .. },
        ] if spell == "fireball" && target == "e0" && attacker == "hero"
    ));
    assert_eq!(count_and_injury(&b, "e0"), (5, 0));
    assert_eq!(b.active().unwrap().stack, "p0", "the turn goes on");
    assert_eq!(b.hero_charges()["fireball"], 0);
    refused(&mut b, cast("lightning", "e0"), &RuleError::AlreadyCast);

    let mut b = magic(2, &[("lightning", 2)], two_sides());
    act(&mut b, cast("lightning", "e0")).unwrap();
    assert_eq!(count_and_injury(&b, "e0"), (20, 0));
    assert_eq!(b.hero_charges()["lightning"], 1);
    let spent = RuleError::NoCharge {
        spell: "fireball".to_owned(),
    };
    refused(
        &mut magic(2, &[("lightning", 1)], two_sides()),
        cast("fireball", "e0"),
        &spent,
    );
    refused(
        &mut magic(2, &[("lightning", 1)], two_sides()),
        cast("lightning", "p0"),
        &wrong("p0"),
    );
}

// Magic does nothing to Dragons: no spell that harms is cast on them, and none is offered.
#[test]
fn a_troop_that_magic_does_nothing_to_takes_no_harmful_spell() {
    let charges = [
        ("fireball", 1),
        ("lightning", 1),
        ("freeze", 1),
        ("turn_undead", 1),
    ];
    let mut b = magic(
        3,
        &charges,
        vec![
            stack("p0", "militia", 10, 1, 2),
            stack("e0", "dragons", 2, 4, 2),
        ],
    );
    for (spell, _) in charges {
        refused(&mut b, cast(spell, "e0"), &wrong("e0"));
    }
    let casts = |a: &Action| matches!(a, Action::Cast { .. } | Action::CastTo { .. });
    assert!(!b.legal_actions().iter().any(casts));
    assert_eq!(
        b.hero_charges().values().sum::<u32>(),
        4,
        "nothing is spent"
    );
}

// Turn Undead is 50 a point of spell power, on the undead only: 50 on Skeletons of three health kills 16 and
// hurts one by 2.
#[test]
fn turn_undead_harms_only_the_undead() {
    let mut b = magic(
        1,
        &[("turn_undead", 2)],
        vec![
            stack("p0", "militia", 10, 1, 2),
            stack("e0", "skeletons", 30, 4, 2),
            stack("e1", "militia", 30, 4, 3),
        ],
    );
    refused(&mut b, cast("turn_undead", "e1"), &wrong("e1"));
    act(&mut b, cast("turn_undead", "e0")).unwrap();
    assert_eq!(count_and_injury(&b, "e0"), (14, 2));
}

// A frozen stack stands out its turn of the round it was frozen in, and acts again in the next.
#[test]
fn freeze_costs_a_stack_its_turn_of_the_round() {
    let mut b = magic(1, &[("freeze", 1)], two_sides());
    let events = act(&mut b, cast("freeze", "e0")).unwrap();
    let frozen = Event::Frozen {
        target: "e0".to_owned(),
    };
    assert_eq!(events.last(), Some(&frozen));
    assert!(b.stack("e0").unwrap().frozen);

    let saved = b.to_canon();
    assert_eq!(Battle::from_canon(rules(), &saved).unwrap(), b);
    let text = String::from_utf8(saved.to_bytes().unwrap()).unwrap();
    assert!(text.contains(r#""frozen":true"#) && text.contains(r#""spell_power":1"#));

    let events = act(&mut b, Action::Pass).unwrap();
    let stood = Event::StoodFrozen {
        stack: "e0".to_owned(),
    };
    assert!(events.contains(&stood));
    assert_eq!((b.round(), b.active().unwrap().stack.as_str()), (2, "p0"));
    assert!(!b.stack("e0").unwrap().frozen, "a new round thaws it");
    act(&mut b, Action::Pass).unwrap();
    assert_eq!(b.active().unwrap().stack, "e0");
}

// Clone: ten health a point of spell power, with the stack's injury, makes whole creatures; the rest is the
// stack's injury afterwards. Wolves of three health hurt by 2, power 1: 12, four Wolves and no injury; unhurt,
// 10: three Wolves, and the stack is hurt by 1.
#[test]
fn clone_adds_the_creatures_its_health_makes() {
    let mut hurt = stack("p0", "wolves", 10, 1, 2);
    hurt.injury = 2;
    let mut b = magic(
        1,
        &[("clone", 2)],
        vec![hurt, stack("e0", "militia", 30, 4, 2)],
    );
    refused(&mut b, cast("clone", "e0"), &wrong("e0"));
    let events = act(&mut b, cast("clone", "p0")).unwrap();
    let cloned = Event::Cloned {
        target: "p0".to_owned(),
        clones: 4,
        count: 14,
        injury: 0,
    };
    assert_eq!(events.last(), Some(&cloned));
    assert_eq!(b.stack("p0").unwrap().began_with, Some(14));

    let wolves = vec![
        stack("p0", "wolves", 10, 1, 2),
        stack("e0", "militia", 30, 4, 2),
    ];
    let mut b = magic(1, &[("clone", 1)], wolves);
    act(&mut b, cast("clone", "p0")).unwrap();
    assert_eq!(count_and_injury(&b, "p0"), (13, 1));
}

// Resurrect: a creature a point of spell power comes back, never more than the stack began with, and the stack
// is unhurt. Ten Militia lose seven to the answer of thirty.
#[test]
fn resurrect_gives_back_the_fallen_up_to_the_number_the_stack_began_with() {
    let stacks = || {
        vec![
            stack("p0", "militia", 10, 1, 2),
            stack("p1", "militia", 5, 0, 0),
            stack("e0", "militia", 30, 2, 2),
        ]
    };
    let mut b = magic(2, &[("resurrect", 2)], stacks());
    refused(&mut b, cast("resurrect", "p0"), &wrong("p0"));
    apply_forced(&mut b, melee("e0"), &[1, 1]).unwrap();
    assert_eq!(b.stack("p0").unwrap().count, 3);
    assert_eq!(b.active().unwrap().stack, "p1");
    refused(&mut b, cast("resurrect", "e0"), &wrong("e0"));
    let events = act(&mut b, cast("resurrect", "p0")).unwrap();
    let revived = Event::Revived {
        target: "p0".to_owned(),
        revived: 2,
        count: 5,
    };
    assert_eq!(events.last(), Some(&revived));
    assert_eq!(count_and_injury(&b, "p0"), (5, 0));

    let mut b = magic(9, &[("resurrect", 2)], stacks());
    apply_forced(&mut b, melee("e0"), &[1, 1]).unwrap();
    act(&mut b, cast("resurrect", "p0")).unwrap();
    assert_eq!(
        count_and_injury(&b, "p0"),
        (10, 0),
        "seven fell, seven are back"
    );
}

// Teleport sets any stack down on any free cell, and takes a cell where the other spells take none.
#[test]
fn teleport_carries_a_stack_of_either_side_to_a_free_cell() {
    let mut b = magic(1, &[("teleport", 2), ("fireball", 1)], two_sides());
    refused(&mut b, cast("teleport", "p0"), &wrong("p0"));
    refused(&mut b, cast_to("fireball", "e0", 0, 0), &wrong("e0"));
    let taken = RuleError::CannotEnter { to: cell(1, 2) };
    refused(&mut b, cast_to("teleport", "e0", 1, 2), &taken);
    let events = act(&mut b, cast_to("teleport", "e0", 2, 2)).unwrap();
    let carried = Event::Teleported {
        stack: "e0".to_owned(),
        from: cell(4, 2),
        to: cell(2, 2),
    };
    assert_eq!(events.last(), Some(&carried));
    assert_eq!(b.stack("e0").unwrap().cell, cell(2, 2));
    assert!(b.legal_actions().contains(&melee("e0")), "now beside p0");
}

// Casts are offered by spell, then by target, and only while the hero has not cast this round.
#[test]
fn the_casts_on_offer_follow_the_charges() {
    let mut b = magic(
        1,
        &[("fireball", 1), ("clone", 1), ("lightning", 0)],
        two_sides(),
    );
    let offered = |b: &Battle| -> Vec<Action> {
        let casts = |a: &Action| matches!(a, Action::Cast { .. });
        b.legal_actions().into_iter().filter(casts).collect()
    };
    assert_eq!(offered(&b), [cast("clone", "p0"), cast("fireball", "e0")]);
    act(&mut b, cast("clone", "p0")).unwrap();
    assert_eq!(offered(&b), []);
}

// A hero casts from his book with a spell power, under the reference rules, and only spells of battle.
#[test]
fn a_scenario_with_magic_that_cannot_be_is_refused() {
    let start = |power, charges: &[(&str, u32)]| {
        Battle::start(rules(), &scenario(power, charges, two_sides()), None).map(|_| ())
    };
    assert_eq!(start(Some(1), &[("freeze", 1)]), Ok(()));
    assert_eq!(start(None, &[]), Ok(()));
    assert!(start(None, &[("freeze", 1)]).is_err(), "no spell power");
    assert!(start(Some(0), &[("freeze", 1)]).is_err(), "a power of none");
    assert!(start(Some(1), &[]).is_err(), "a power and nothing to cast");
    assert!(
        start(Some(1), &[("bridge", 1)]).is_err(),
        "a spell of the map"
    );

    // A battle without magic is written as it always was.
    let plain = Battle::start(rules(), &scenario(None, &[], two_sides()), None).unwrap();
    let text = String::from_utf8(plain.to_canon().to_bytes().unwrap()).unwrap();
    for key in ["spell_power", "frozen", "began_with"] {
        assert!(!text.contains(key), "{key}");
    }
}
