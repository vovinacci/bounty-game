//! The buried sceptre and its puzzle (`docs/spec/07-RULES-REGISTER.md`, U-16). The meadow with seven artefacts
//! serves, its objective taken out and a puzzle put in; and the meadow with castles, for a lord's piece.

use super::king::close_and_fight;
use super::*;

const OBJECTIVE: &str = r#"  "objective": {
    "object_id": "meadow-objective",
    "hidden_until_clue": true,
    "search_requires_clue": false
  },
"#;

/// An expedition's file with a puzzle of this seed in place of its objective, and nobody giving a clue.
fn puzzled(expedition: &str, seed: u32) -> String {
    assert!(expedition.contains(OBJECTIVE));
    let puzzle = format!("  \"puzzle\": {{\n    \"seed\": {seed}\n  }},\n");
    expedition
        .replacen(OBJECTIVE, &puzzle, 1)
        .replace(r#""grants_clue": true"#, r#""grants_clue": false"#)
}

fn def(seed: u32) -> ExpeditionDef {
    let (expedition, map) = artefacts::texts();
    ExpeditionDef::from_texts(&puzzled(&expedition, seed), &map, REFERENCE).unwrap()
}

fn meadow(seed: u32) -> Expedition {
    Expedition::start(Arc::new(def(seed)))
}

// The sceptre lies on free ground inside the window, the window on the map; every artefact is a piece, and the
// twenty-five cells are dealt so that each piece covers three or four. The seed decides all of it.
#[test]
fn the_seed_buries_the_sceptre_and_deals_the_pieces() {
    let def = def(7);
    let puzzle = def.puzzle.as_ref().unwrap();
    let sceptre = puzzle.sceptre;
    assert!(def.map.passable(sceptre) && def.map.object_at(sceptre).is_none());
    assert_eq!(def.objective(), sceptre);
    let (dx, dy) = (sceptre.x - puzzle.origin.x, sceptre.y - puzzle.origin.y);
    assert!(dx < WINDOW && dy < WINDOW, "the sceptre is in the window");
    assert!(
        puzzle.origin.x + WINDOW <= def.map.width && puzzle.origin.y + WINDOW <= def.map.height
    );
    let outside = Cell {
        x: puzzle.origin.x + WINDOW,
        y: puzzle.origin.y,
    };
    assert_eq!(puzzle.piece_over(outside), None);

    let wanted: Vec<Piece> = artefacts::LAID
        .iter()
        .map(|(id, _, _)| Piece::Artefact((*id).to_owned()))
        .collect();
    assert_eq!(puzzle.pieces, wanted);
    assert_eq!(puzzle.cover.len(), 25);
    for piece in 0..puzzle.pieces.len() {
        let covers = puzzle.cover.iter().filter(|p| **p == piece).count();
        assert!((3..=4).contains(&covers), "piece {piece} covers {covers}");
    }

    assert_eq!(
        self::def(7).puzzle,
        def.puzzle,
        "the same seed, the same puzzle"
    );
    let others = (1..=5).map(|seed| self::def(seed).puzzle);
    assert!(others.into_iter().any(|other| other != def.puzzle));
}

#[test]
fn a_prize_that_cannot_be_is_refused() {
    let (expedition, map) = artefacts::texts();
    let loads = |text: &str, map: &str| ExpeditionDef::from_texts(text, map, REFERENCE).is_ok();
    let with_puzzle = puzzled(&expedition, 7);
    assert!(loads(&with_puzzle, &map));
    let both = with_puzzle.replacen("  \"puzzle\"", &format!("{OBJECTIVE}  \"puzzle\""), 1);
    assert!(!loads(&both, &map), "an objective and a puzzle");
    assert!(
        !loads(&expedition.replacen(OBJECTIVE, "", 1), &map),
        "neither"
    );
    let clue = with_puzzle.replacen(r#""grants_clue": false"#, r#""grants_clue": true"#, 1);
    assert!(!loads(&clue, &map), "a clue beside the puzzle");
    assert!(
        !loads(&puzzled(MEADOW, 7), MEADOW_MAP),
        "no lord and no artefact for a piece"
    );
}

// Every artefact found lifts its piece. The sceptre's place is known once the piece over it is lifted, and not
// before; a search there wins, and nowhere else.
#[test]
fn finding_artefacts_lifts_pieces_until_the_sceptre_shows() {
    let mut e = meadow(7);
    let puzzle = e.puzzle().unwrap().clone();
    let sceptre = puzzle.sceptre;
    let Piece::Artefact(over) = &puzzle.pieces[puzzle.piece_over(sceptre).unwrap()] else {
        panic!("every piece is an artefact's");
    };
    assert_eq!((e.pieces_lifted(), e.objective_known()), (0, None));
    assert!(!e.shows(sceptre));

    // East along the first row, then south: to the artefact whose piece covers the sceptre.
    let (_, x, y) = artefacts::LAID
        .iter()
        .find(|(id, _, _)| id == over)
        .unwrap();
    let mut events = Vec::new();
    for (dx, dy, steps) in [(1, 0, x - 1), (0, 1, y - 1)] {
        for _ in 0..steps {
            assert_eq!(e.objective_known(), None, "not before its piece is lifted");
            events.extend(act(&mut e, Action::Move { dx, dy }).unwrap());
        }
    }
    assert!(e.artefacts_found().contains(&over.as_str()));
    assert_eq!(e.objective_known(), Some(sceptre));
    let lifted: Vec<(u32, u32, bool)> = events
        .iter()
        .filter_map(|event| match event {
            Event::PieceLifted {
                lifted,
                of,
                sceptre,
            } => Some((*lifted, *of, *sceptre)),
            _ => None,
        })
        .collect();
    assert_eq!(lifted.len(), e.pieces_lifted());
    assert_eq!(lifted.last(), Some(&(lifted.len() as u32, 7, true)));
    assert_eq!(lifted.iter().filter(|(_, _, shown)| *shown).count(), 1);

    let def = Arc::clone(e.def_shared());
    assert_eq!(Expedition::from_canon(def, &e.to_canon(), None).unwrap(), e);

    let searched = act(&mut e, Action::Search).unwrap();
    assert!(searched.contains(&Event::Searched { found: false }));
    e.hero = sceptre;
    let searched = act(&mut e, Action::Search).unwrap();
    assert!(searched.contains(&Event::Searched { found: true }));
    assert_eq!(e.status(), Status::Victory);
}

// A lord is a piece too: with one lord and no artefact his piece covers the whole window, and catching him
// shows the sceptre.
#[test]
fn a_lord_caught_lifts_his_piece() {
    let (expedition, map) = castles::texts(None);
    let def = ExpeditionDef::from_texts(&puzzled(&expedition, 3), &map, REFERENCE).unwrap();
    let mut e = Expedition::start(Arc::new(def));
    let hall = Piece::Lord("meadow-hall".to_owned());
    assert_eq!(e.puzzle().unwrap().pieces, std::slice::from_ref(&hall));
    assert!(!e.is_lifted(&hall));
    act(&mut e, Action::BuySiegeWeapons).unwrap();
    let id = "meadow-hall".to_owned();
    act(&mut e, Action::TakeContract { id }).unwrap();
    go(&mut e, 1, 0, 1);
    go(&mut e, 0, 1, 1);
    act(&mut e, Action::Move { dx: 1, dy: 0 }).unwrap();
    let events = close_and_fight(&mut e);
    let shown = Event::PieceLifted {
        lifted: 1,
        of: 1,
        sceptre: true,
    };
    assert!(events.contains(&shown), "{events:?}");
    assert!(e.is_lifted(&hall));
    assert_eq!(e.objective_known(), Some(e.puzzle().unwrap().sceptre));
}
