//! Bands that do not fight (`docs/spec/07-RULES-REGISTER.md`, U-07): a friendly band offers to join the hero.
//! The meadow serves, with thirty Gnomes written into its texts south-east of the town.

use super::*;

const CAMP: Cell = Cell { x: 2, y: 2 };

fn texts() -> (String, String) {
    let object = r#"    {
      "id": "meadow-objective","#;
    let gnomes = r#"    {
      "id": "meadow-gnomes",
      "kind": "encounter",
      "x": 2,
      "y": 2
    },
"#;
    assert!(MEADOW_MAP.contains(object));
    let map = MEADOW_MAP.replacen(object, &format!("{gnomes}{object}"), 1);
    let bands = r#"  "encounters": ["#;
    let friendly = r#"  "encounters": [
    {
      "object_id": "meadow-gnomes",
      "enemy_troop": "gnomes",
      "enemy_count": 30,
      "battle_rng_state": 0,
      "gold_reward": 0,
      "grants_clue": false,
      "friendly": true
    },"#;
    assert!(MEADOW.contains(bands));
    (MEADOW.replacen(bands, friendly, 1), map)
}

fn at_the_camp() -> Expedition {
    let (expedition, map) = texts();
    let def = ExpeditionDef::from_texts(&expedition, &map, REFERENCE).unwrap();
    let mut e = Expedition::start(Arc::new(def));
    go(&mut e, 1, 0, 1);
    let events = act(&mut e, Action::Move { dx: 0, dy: 1 }).unwrap();
    let fought = events
        .iter()
        .any(|event| matches!(event, Event::BattleStarted { .. }));
    assert!(!fought && e.hero() == CAMP && e.status() == Status::Adventure);
    e
}

// A friendly band is stepped onto like any place. Taken in, it joins the army for nothing, whatever the hero's
// leadership: thirty Gnomes of five health are 150 against a Knight's hundred. Then it is gone from the map.
#[test]
fn a_friendly_band_joins_the_army_for_nothing() {
    let mut e = at_the_camp();
    assert_eq!(e.band_here().unwrap().object_id, "meadow-gnomes");
    let gold = e.gold();
    let events = act(&mut e, Action::AcceptBand).unwrap();
    assert_eq!(
        events,
        [Event::BandJoined {
            band: "meadow-gnomes".to_owned(),
            troop_id: "gnomes".to_owned(),
            count: 30
        }]
    );
    let stack = &e.army()[1];
    assert_eq!(
        (stack.id.as_str(), stack.troop_id.as_str(), stack.count),
        ("p1", "gnomes", 30)
    );
    assert_eq!((e.gold(), e.leadership()), (gold, 100));
    assert!(e.is_cleared("meadow-gnomes") && e.band_here().is_none());
    refused(&mut e, Action::AcceptBand, &Refused::NoBandToJoin);
    refused(&mut e, Action::DismissBand, &Refused::NoBandToJoin);
    // Where the troop is in the army already, the band joins that stack.
    let mut e = at_the_camp();
    e.army[0].troop_id = "gnomes".to_owned();
    act(&mut e, Action::AcceptBand).unwrap();
    assert_eq!((e.army().len(), e.army()[0].count), (1, 50));
}

// Sent off, the band goes its way and the army is as it was; and so it does when every slot holds another troop.
#[test]
fn a_friendly_band_sent_off_or_without_room_goes_its_way() {
    let left = [Event::BandLeft {
        band: "meadow-gnomes".to_owned(),
    }];
    let mut e = at_the_camp();
    let army = e.army().to_vec();
    assert_eq!(act(&mut e, Action::DismissBand).unwrap(), left);
    assert_eq!(e.army(), army);
    assert!(e.is_cleared("meadow-gnomes"));

    let mut e = at_the_camp();
    for (slot, troop) in ["wolves", "archers", "militia", "orcs"].iter().enumerate() {
        e.army.push(ArmyStack {
            id: format!("p{}", slot + 1),
            troop_id: (*troop).to_owned(),
            count: 1,
        });
    }
    let army = e.army().to_vec();
    assert_eq!(act(&mut e, Action::AcceptBand).unwrap(), left);
    assert_eq!(e.army(), army);
    assert!(e.is_cleared("meadow-gnomes"));
}

// There is no contract on those who would join, and nobody to answer where no such band stands.
#[test]
fn a_friendly_band_is_no_villain() {
    let (expedition, map) = texts();
    let def = ExpeditionDef::from_texts(&expedition, &map, REFERENCE).unwrap();
    let mut e = Expedition::start(Arc::new(def));
    assert!(!e.contracts_on_offer().contains(&"meadow-gnomes"));
    let none = Refused::NoSuchContract {
        id: "meadow-gnomes".to_owned(),
    };
    let take = Action::TakeContract {
        id: "meadow-gnomes".to_owned(),
    };
    refused(&mut e, take, &none);
    refused(&mut e, Action::AcceptBand, &Refused::NoBandToJoin);

    let broken = |old: &str, new: &str| {
        assert!(expedition.contains(old), "{old}");
        ExpeditionDef::from_texts(&expedition.replacen(old, new, 1), &map, REFERENCE).is_err()
    };
    let friendly = r#""grants_clue": false,
      "friendly": true"#;
    assert!(
        broken(r#""gold_reward": 0,"#, r#""gold_reward": 5,"#),
        "a reward"
    );
    assert!(
        broken(friendly, "\"grants_clue\": true,\n      \"friendly\": true"),
        "a clue"
    );
    let more = "\"grants_clue\": false,\n      \"also\": [{\"troop\": \"wolves\", \"count\": 1}],\n      \"friendly\": true";
    assert!(broken(friendly, more), "a second stack");
}
