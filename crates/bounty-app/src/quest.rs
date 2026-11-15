//! The puzzle map: the window of land round the buried sceptre, its pieces lifted or not. It shows what the
//! rules say shows (`Expedition::shows`); nothing is decided here.

use bounty_core::campaign::{Piece, WINDOW};
use bounty_core::content::Cell;
use gamekit::t;
use gamekit::text::{text_at, text_centered_fit};
use macroquad::prelude::*;

use crate::adventure::Adventure;
use crate::map::{MAP, terrain};
use crate::view::{CURSOR, ENEMY, LINE, PANEL};
use crate::words::{named, place_name};

/// Side of a cell of the puzzle.
const CELL: f32 = 100.0;

/// The puzzle in the map's place: a cell whose piece is lifted shows its ground, bare of what stands on it, and
/// the sceptre's own cell its mark; a covered cell shows whose piece covers it.
pub fn draw(adventure: &Adventure) {
    let expedition = &adventure.expedition;
    let Some(puzzle) = expedition.puzzle() else {
        return;
    };
    for dy in 0..WINDOW {
        for dx in 0..WINDOW {
            let cell = Cell {
                x: puzzle.origin.x + dx,
                y: puzzle.origin.y + dy,
            };
            let at = MAP + vec2(dx as f32, dy as f32) * CELL;
            let centre = at + vec2(CELL / 2.0, CELL / 2.0);
            if expedition.shows(cell) {
                terrain(adventure, cell, at, CELL);
                if cell == puzzle.sceptre {
                    draw_poly_lines(centre.x, centre.y, 4, 30.0, 0.0, 6.0, CURSOR);
                }
                continue;
            }
            draw_rectangle(at.x, at.y, CELL, CELL, PANEL);
            draw_rectangle_lines(at.x, at.y, CELL, CELL, 1.0, LINE);
            let Some(piece) = puzzle.piece_over(cell).map(|place| &puzzle.pieces[place]) else {
                continue;
            };
            // Whose piece it is: a lord's in the enemy's colour, an artefact's in the colour of what is found.
            let (name, colour) = match piece {
                Piece::Lord(castle) => (place_name(castle), ENEMY),
                Piece::Artefact(id) => (named("artefact", id), CURSOR),
            };
            draw_rectangle_lines(centre.x - 14.0, centre.y - 26.0, 28.0, 28.0, 4.0, colour);
            text_centered_fit(
                &name,
                centre.x,
                at.y + CELL - 16.0,
                CELL - 10.0,
                14.0,
                LIGHTGRAY,
            );
        }
    }
    let lifted = t!(
        "puzzle-lifted",
        n = expedition.pieces_lifted(),
        of = puzzle.pieces.len()
    );
    let under = MAP.y + WINDOW as f32 * CELL + 28.0;
    text_at(&lifted, MAP.x, under, 20.0, WHITE);
}
