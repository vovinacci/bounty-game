//! The hero's spells of the map (`docs/spec/07-RULES-REGISTER.md`, U-13). The meadow with magic serves; the hero is
//! given what a test needs of charges, as a Knight of spell power 1 who has been taught.

use super::*;
use crate::canon::Value;

fn caster_on(map: &str, charges: &[(&str, u32)]) -> Expedition {
    let (expedition, _) = magic::texts();
    let def = ExpeditionDef::from_texts(&expedition, map, REFERENCE).unwrap();
    let mut e = Expedition::start(Arc::new(def));
    give(&mut e, charges);
    e
}

/// The hero can cast and holds these charges, however he came by them.
fn give(e: &mut Expedition, charges: &[(&str, u32)]) {
    let book = e.def().rules.book.clone();
    let magic = e.magic.get_or_insert_with(|| Magic {
        knows: true,
        charges: vec![0; book.len()],
        free_steps: 0,
        raised: 0,
        bridges: Vec::new(),
    });
    magic.knows = true;
    for (spell, n) in charges {
        let place = book.iter().position(|s| s.id == *spell).unwrap();
        magic.charges[place] = *n;
    }
}

fn caster(charges: &[(&str, u32)]) -> Expedition {
    caster_on(&magic::texts().1, charges)
}

fn cast(spell: &str, aim: Aim) -> Action {
    Action::CastOnMap {
        spell: spell.to_owned(),
        aim,
    }
}

fn towards(dx: i32, dy: i32) -> Aim {
    Aim::Towards { dx, dy }
}

fn spell_cast(spell: &str) -> Event {
    Event::MapSpellCast {
        spell: spell.to_owned(),
    }
}

// Only a hero who casts, with a charge of a spell of the map, aimed as the spell is aimed; a refusal spends
// nothing. The spell that finds a villain has none to find yet.
#[test]
fn a_cast_needs_a_caster_a_charge_and_the_spells_aim() {
    let mut e = caster(&[("time_stop", 1), ("fireball", 1), ("find_villain", 1)]);
    let none = Refused::NoSpellCharge {
        spell: "bridge".to_owned(),
    };
    refused(&mut e, cast("bridge", towards(1, 0)), &none);
    let unknown = Refused::NoSpellCharge {
        spell: "tunnel".to_owned(),
    };
    refused(&mut e, cast("tunnel", Aim::None), &unknown);
    refused(
        &mut e,
        cast("fireball", Aim::None),
        &Refused::NotCastOnTheMap,
    );
    refused(&mut e, cast("time_stop", towards(1, 0)), &Refused::WrongAim);
    refused(
        &mut e,
        cast("find_villain", Aim::None),
        &Refused::NothingToFind,
    );
    assert_eq!(e.casts_on_map(), [cast("time_stop", Aim::None)]);

    e.magic.as_mut().unwrap().knows = false;
    refused(&mut e, cast("time_stop", Aim::None), &Refused::CannotCast);
    assert_eq!(e.casts_on_map(), []);
}

// Time Stop: a twentieth of a week's walking for a point of spell power, here one step of twenty. The step
// costs no time; time that passes in any other way ends what is left.
#[test]
fn time_stop_gives_steps_that_cost_no_time() {
    let mut e = caster(&[("time_stop", 3)]);
    let events = act(&mut e, cast("time_stop", Aim::None)).unwrap();
    assert_eq!(
        events,
        [spell_cast("time_stop"), Event::TimeStopped { steps: 1 }]
    );
    act(&mut e, cast("time_stop", Aim::None)).unwrap();
    assert_eq!(
        (e.free_steps(), e.charges("time_stop"), e.ticks()),
        (2, 1, 0)
    );

    let events = act(&mut e, Action::Move { dx: 1, dy: 0 }).unwrap();
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::TimePassed { .. }))
    );
    assert_eq!(
        (e.free_steps(), e.ticks(), e.hero()),
        (1, 0, Cell { x: 2, y: 1 })
    );
    act(&mut e, Action::WaitTick).unwrap();
    assert_eq!((e.free_steps(), e.ticks()), (0, 1));
    act(&mut e, Action::Move { dx: 1, dy: 0 }).unwrap();
    assert_eq!(e.ticks(), 2, "a step takes its time again");
}

// Raise Control: a hundred of leadership for a point of spell power, until the week ends.
#[test]
fn raise_control_lifts_the_leadership_for_the_week() {
    let mut e = caster(&[("raise_control", 1)]);
    let events = act(&mut e, cast("raise_control", Aim::None)).unwrap();
    assert_eq!(events[1], Event::ControlRaised { leadership: 100 });
    assert_eq!(e.leadership(), 200);
    wait(&mut e, 19);
    assert_eq!(e.leadership(), 200);
    wait(&mut e, 1);
    assert_eq!(e.leadership(), 100, "the week is over");
}

// Instant Army: the troop of the hero's rank, Peasants for a Knight, a point of spell power and one more times
// three: six, for nothing. They join their own or take a free place; with none, the spell is not cast.
#[test]
fn instant_army_raises_the_ranks_troop() {
    let mut e = caster(&[("instant_army", 3)]);
    assert_eq!(e.instant_army(), Some(("peasants", 6)));
    let gold = e.gold();
    let events = act(&mut e, cast("instant_army", Aim::None)).unwrap();
    let raised = Event::ArmyRaised {
        troop_id: "peasants".to_owned(),
        count: 6,
    };
    assert_eq!(events, [spell_cast("instant_army"), raised]);
    act(&mut e, cast("instant_army", Aim::None)).unwrap();
    let army: Vec<(&str, &str, u32)> = e
        .army()
        .iter()
        .map(|s| (s.id.as_str(), s.troop_id.as_str(), s.count))
        .collect();
    assert_eq!(army, [("p0", "sprites", 20), ("p1", "peasants", 12)]);
    assert_eq!(e.gold(), gold);

    let mut full = caster(&[("instant_army", 1)]);
    for (n, troop) in ["wolves", "archers", "militia", "skeletons"]
        .iter()
        .enumerate()
    {
        full.army.push(ArmyStack {
            id: format!("p{}", n + 1),
            troop_id: (*troop).to_owned(),
            count: 1,
        });
    }
    refused(
        &mut full,
        cast("instant_army", Aim::None),
        &Refused::ArmyFull,
    );
}

// Bridge: the first cell of water the hero faces, and the next if it is water too, can be walked from then on.
#[test]
fn a_bridge_is_laid_over_one_or_two_cells_of_water() {
    let map = magic::texts().1;
    let (third, fourth) = (r#""~........^...~""#, r#""~.....^..^...~""#);
    assert!(map.contains(third) && map.contains(fourth));
    let map =
        map.replacen(third, r#""~~.......^...~""#, 1)
            .replacen(fourth, r#""~~....^..^...~""#, 1);
    let mut e = caster_on(&map, &[("bridge", 3)]);
    go(&mut e, 0, 1, 1);
    refused(&mut e, Action::Move { dx: 0, dy: 1 }, &Refused::CannotEnter);
    refused(
        &mut e,
        cast("bridge", towards(1, 0)),
        &Refused::NoWaterToBridge,
    );
    refused(
        &mut e,
        cast("bridge", towards(1, 1)),
        &Refused::NoWaterToBridge,
    );
    refused(&mut e, cast("bridge", Aim::None), &Refused::WrongAim);
    let (near, far) = (Cell { x: 1, y: 3 }, Cell { x: 1, y: 4 });
    let events = act(&mut e, cast("bridge", towards(0, 1))).unwrap();
    let bridged = Event::Bridged {
        cells: vec![near, far],
    };
    assert_eq!(events, [spell_cast("bridge"), bridged]);
    assert_eq!(e.bridges(), [near, far]);
    refused(
        &mut e,
        cast("bridge", towards(0, 1)),
        &Refused::NoWaterToBridge,
    );
    go(&mut e, 0, 1, 3);
    assert_eq!(e.hero(), Cell { x: 1, y: 5 });

    // The water at the map's edge is one cell deep.
    let events = act(&mut e, cast("bridge", towards(-1, 0))).unwrap();
    let edge = Event::Bridged {
        cells: vec![Cell { x: 0, y: 5 }],
    };
    assert_eq!(events[1], edge);
    assert_eq!(e.charges("bridge"), 1);
}

// The gates carry the hero to the town, or to a castle of his own, in no time; never to where he stands, to the
// king's castle or to a castle still held against him.
#[test]
fn the_gates_lead_to_the_town_and_to_the_heros_castles() {
    let mut e = caster(&[("town_gate", 2), ("castle_gate", 1)]);
    let town = || Aim::Place("meadow-town".to_owned());
    let no_gate = |place: &str| Refused::NoGateTo {
        place: place.to_owned(),
    };
    refused(&mut e, cast("town_gate", town()), &no_gate("meadow-town"));
    go(&mut e, 1, 0, 3);
    let ticks = e.ticks();
    let castle = Aim::Place("meadow-castle".to_owned());
    refused(
        &mut e,
        cast("town_gate", castle.clone()),
        &no_gate("meadow-castle"),
    );
    refused(
        &mut e,
        cast("castle_gate", castle),
        &no_gate("meadow-castle"),
    );
    refused(&mut e, cast("castle_gate", town()), &no_gate("meadow-town"));
    let events = act(&mut e, cast("town_gate", town())).unwrap();
    let gated = Event::Gated {
        from: Cell { x: 4, y: 1 },
        to: Cell { x: 1, y: 1 },
    };
    assert_eq!(events, [spell_cast("town_gate"), gated]);
    assert_eq!(
        (e.hero(), e.ticks(), e.charges("town_gate")),
        (Cell { x: 1, y: 1 }, ticks, 1)
    );

    let mut e = castles::in_the_keep(None);
    give(&mut e, &[("castle_gate", 1)]);
    let keep = || Aim::Place("meadow-keep".to_owned());
    let hall = Aim::Place("meadow-hall".to_owned());
    refused(&mut e, cast("castle_gate", keep()), &no_gate("meadow-keep"));
    go(&mut e, 0, -1, 1);
    refused(&mut e, cast("castle_gate", hall), &no_gate("meadow-hall"));
    assert_eq!(e.casts_on_map(), [cast("castle_gate", keep())]);
    act(&mut e, cast("castle_gate", keep())).unwrap();
    assert_eq!(e.own_castle_here(), Some("meadow-keep"));
}

// What the spells leave behind is part of the state where there is magic.
#[test]
fn what_the_spells_of_the_map_leave_is_saved() {
    let mut e = caster(&[("time_stop", 1), ("raise_control", 1), ("bridge", 1)]);
    act(&mut e, cast("bridge", towards(-1, 0))).unwrap();
    act(&mut e, cast("raise_control", Aim::None)).unwrap();
    act(&mut e, cast("time_stop", Aim::None)).unwrap();
    let def = Arc::clone(e.def_shared());
    let saved = e.to_canon();
    let text = String::from_utf8(saved.to_bytes().unwrap()).unwrap();
    let magic =
        r#""magic":{"book":{},"bridges":[{"x":0,"y":1}],"free_steps":1,"knows":true,"raised":100}"#;
    assert!(text.contains(magic), "{text}");
    let changed = |old: &str, new: &str| {
        assert!(text.contains(old), "{old} in {text}");
        let value = Value::parse(&text.replacen(old, new, 1)).unwrap();
        Expedition::from_canon(Arc::clone(&def), &value, None).map(|_| ())
    };
    assert_eq!(changed(r#""raised":100"#, r#""raised":0"#), Ok(()));
    assert!(
        changed(r#""knows":true"#, r#""knows":false"#).is_err(),
        "no caster"
    );
    let bridge = r#""bridges":[{"x":0,"y":1}]"#;
    assert!(
        changed(bridge, r#""bridges":[{"x":2,"y":1}]"#).is_err(),
        "no water"
    );
    let twice = r#""bridges":[{"x":0,"y":1},{"x":0,"y":1}]"#;
    assert!(changed(bridge, twice).is_err(), "twice");
    assert_eq!(Expedition::from_canon(def, &saved, None).unwrap(), e);
}
