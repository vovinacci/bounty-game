//! The buried sceptre and the puzzle map that shows where it lies (`docs/spec/07-RULES-REGISTER.md`, U-16). The
//! sceptre is buried on free ground, drawn from the expedition's seed. The puzzle is a window of the map round it,
//! covered by pieces: one for every lord and one for every artefact laid out. A piece is lifted when its lord is
//! caught or its artefact found, and the ground under it shows.

use super::content::{EnemyCastle, LaidArtefact, Map};
use super::{Event, Expedition};
use crate::content::{Cell, ContentError, Rules};
use crate::rng::Lcg64;

/// The side of the puzzle's window, in cells.
pub const WINDOW: u32 = 5;

/// What a piece of the puzzle stands for: who or what has to be caught or found for it to be lifted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Piece {
    /// The lord of this castle, by its object's ID.
    Lord(String),
    /// This artefact, by its ID in the rules.
    Artefact(String),
}

/// The puzzle of an expedition: where the sceptre lies, the window of the map the puzzle shows, and which piece
/// covers each cell of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Puzzle {
    /// Where the sceptre is buried. Not shown until the piece over it is lifted.
    pub sceptre: Cell,
    /// The top-left cell of the window, which lies wholly on the map and holds the sceptre.
    pub origin: Cell,
    pub pieces: Vec<Piece>,
    /// For each cell of the window, row by row, the place of its piece in `pieces`.
    pub cover: Vec<usize>,
}

impl Puzzle {
    /// Buries the sceptre and deals the window's cells to the pieces, both from `seed`. The sceptre lies on ground
    /// that can be walked, with nothing standing on it; the cells go to the pieces in turn, in an order the seed
    /// shuffles, so every piece covers as many cells as any other, or one more.
    ///
    /// # Errors
    /// A map smaller than the window, no free ground, or no piece or more pieces than cells.
    pub(super) fn make(
        seed: u32,
        map: &Map,
        castles: &[EnemyCastle],
        artefacts: &[LaidArtefact],
        rules: &Rules,
    ) -> Result<Self, ContentError> {
        let invalid = |why: &str| Err(ContentError::Invalid(format!("the puzzle: {why}")));
        if map.width < WINDOW || map.height < WINDOW {
            return invalid("the map is smaller than the puzzle's window");
        }
        let lords = castles.iter().filter(|castle| castle.lord.is_some());
        let lords = lords.map(|castle| Piece::Lord(castle.object_id.clone()));
        let found = artefacts.iter();
        let found = found.map(|laid| Piece::Artefact(rules.artefacts[laid.artefact].id.clone()));
        let pieces: Vec<Piece> = lords.chain(found).collect();
        let cells = (WINDOW * WINDOW) as usize;
        if pieces.is_empty() || pieces.len() > cells {
            return invalid(
                "it needs a lord or an artefact for a piece, and no more of them than it has cells",
            );
        }
        let all = (0..map.height).flat_map(|y| (0..map.width).map(move |x| Cell { x, y }));
        let free: Vec<Cell> = all
            .filter(|&cell| map.passable(cell) && map.object_at(cell).is_none())
            .collect();
        let Some(last) = free.len().checked_sub(1) else {
            return invalid("there is no free ground to bury the sceptre in");
        };
        let mut rng = Lcg64::new(u64::from(seed));
        let at = rng.range(0, u32::try_from(last).unwrap_or(u32::MAX));
        let sceptre = free[at as usize];
        // The sceptre in the middle where the map allows it, the window pushed back onto the map where not.
        let origin = Cell {
            x: sceptre.x.saturating_sub(WINDOW / 2).min(map.width - WINDOW),
            y: sceptre
                .y
                .saturating_sub(WINDOW / 2)
                .min(map.height - WINDOW),
        };
        let mut order: Vec<usize> = (0..cells).collect();
        for i in (1..cells).rev() {
            let j = rng.range(0, u32::try_from(i).unwrap_or(u32::MAX)) as usize;
            order.swap(i, j);
        }
        let mut cover = vec![0; cells];
        for (turn, cell) in order.into_iter().enumerate() {
            cover[cell] = turn % pieces.len();
        }
        Ok(Self {
            sceptre,
            origin,
            pieces,
            cover,
        })
    }

    /// The place among the pieces of the one that covers this cell of the map; `None` outside the window.
    #[must_use]
    pub fn piece_over(&self, cell: Cell) -> Option<usize> {
        let (dx, dy) = (
            cell.x.checked_sub(self.origin.x)?,
            cell.y.checked_sub(self.origin.y)?,
        );
        (dx < WINDOW && dy < WINDOW).then(|| self.cover[(dy * WINDOW + dx) as usize])
    }
}

impl Expedition {
    /// The puzzle, where the expedition has one.
    #[must_use]
    pub fn puzzle(&self) -> Option<&Puzzle> {
        self.def.puzzle.as_ref()
    }

    /// Whether this piece of the puzzle is lifted: its lord caught, or its artefact found.
    #[must_use]
    pub fn is_lifted(&self, piece: &Piece) -> bool {
        match piece {
            Piece::Lord(castle) => self.holds.get(castle).is_some_and(|hold| hold.caught),
            Piece::Artefact(id) => self.found.contains_key(id),
        }
    }

    /// How many pieces of the puzzle are lifted.
    #[must_use]
    pub fn pieces_lifted(&self) -> usize {
        let pieces = self.puzzle().map_or(&[][..], |puzzle| &puzzle.pieces);
        pieces.iter().filter(|piece| self.is_lifted(piece)).count()
    }

    /// Whether the ground under this cell of the puzzle's window shows: its piece is lifted.
    #[must_use]
    pub fn shows(&self, cell: Cell) -> bool {
        let piece = self
            .puzzle()
            .and_then(|puzzle| Some(&puzzle.pieces[puzzle.piece_over(cell)?]));
        piece.is_some_and(|piece| self.is_lifted(piece))
    }

    /// Where the sceptre lies, once the piece over it is lifted.
    pub(super) fn sceptre_shown(&self) -> Option<Cell> {
        let sceptre = self.puzzle()?.sceptre;
        self.shows(sceptre).then_some(sceptre)
    }

    /// Tells that a piece was lifted, after a lord was caught or an artefact found: `before` is how many were
    /// lifted until then, and whether the sceptre's place showed.
    pub(super) fn tell_piece_lifted(&self, before: (usize, bool), events: &mut Vec<Event>) {
        let Some(puzzle) = self.puzzle() else {
            return;
        };
        let lifted = self.pieces_lifted();
        if lifted > before.0 {
            events.push(Event::PieceLifted {
                lifted: u32::try_from(lifted).unwrap_or(u32::MAX),
                of: u32::try_from(puzzle.pieces.len()).unwrap_or(u32::MAX),
                sceptre: self.sceptre_shown().is_some() && !before.1,
            });
        }
    }

    /// How the puzzle stands now, for telling afterwards what a command lifted.
    pub(super) fn puzzle_before(&self) -> (usize, bool) {
        (self.pieces_lifted(), self.sceptre_shown().is_some())
    }
}
