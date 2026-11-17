//! Towns beyond an expedition's first (`docs/spec/07-RULES-REGISTER.md`, U-17). Every town sells its own troops and
//! its own spell and has word of its own castle; what any town does (contracts, siege weapons) every town does.

use std::collections::BTreeMap;

use super::content::{Map, ObjectKind};
use super::{Expedition, ExpeditionDef};
use crate::content::{BookSpell, Cell, ContentError, Rules};

/// A town of the map beyond the expedition's first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OtherTown {
    pub object_id: String,
    pub cell: Cell,
    /// What it sells when the expedition starts: stock and price by troop ID.
    pub stock: BTreeMap<String, u32>,
    pub prices: BTreeMap<String, u64>,
    /// The castle it has word of.
    pub intel_castle: Option<String>,
    /// The spell it sells: its place in the rules' book.
    pub spell_for_sale: Option<usize>,
}

/// What a town's file says of it, checked: its object is a town's, its troops are the rules' and priced, its spell
/// is of the book.
pub(super) fn other_town(
    object_id: &str,
    stock: &BTreeMap<String, u32>,
    prices: &BTreeMap<String, u64>,
    (intel_castle, spell): (Option<&String>, Option<&String>),
    map: &Map,
    rules: &Rules,
) -> Result<OtherTown, ContentError> {
    let invalid = |why: String| Err(ContentError::Invalid(format!("town {object_id:?}: {why}")));
    let Some(object) = map.object(object_id).filter(|o| o.kind == ObjectKind::Town) else {
        return invalid("it is not a town of the map".to_owned());
    };
    if let Some(unknown) = stock
        .keys()
        .chain(prices.keys())
        .find(|troop| rules.troop(troop).is_none())
    {
        return invalid(format!("it sells the unknown troop {unknown:?}"));
    }
    if let Some(unpriced) = stock.keys().find(|troop| !prices.contains_key(*troop)) {
        return invalid(format!("it stocks {unpriced:?} without a price"));
    }
    let spell_for_sale = match spell {
        Some(id) => match rules.book.iter().position(|spell| spell.id == *id) {
            Some(place) => Some(place),
            None => {
                return invalid(format!(
                    "it sells the spell {id:?}, which the rules do not have"
                ));
            }
        },
        None => None,
    };
    Ok(OtherTown {
        object_id: object_id.to_owned(),
        cell: object.cell(),
        stock: stock.clone(),
        prices: prices.clone(),
        intel_castle: intel_castle.cloned(),
        spell_for_sale,
    })
}

/// A town as the rules meet it on a cell: the expedition's first, or one of the others.
#[derive(Clone, Copy)]
pub(super) enum TownAt<'a> {
    First,
    Other(&'a OtherTown),
}

impl ExpeditionDef {
    /// The town on this cell.
    pub(super) fn town_at(&self, cell: Cell) -> Option<TownAt<'_>> {
        if cell == self.town {
            return Some(TownAt::First);
        }
        let other = self.more_towns.iter().find(|town| town.cell == cell);
        other.map(TownAt::Other)
    }

    /// The cells of every town, the first town's first.
    #[must_use]
    pub fn town_cells(&self) -> Vec<Cell> {
        let others = self.more_towns.iter().map(|town| town.cell);
        std::iter::once(self.town).chain(others).collect()
    }

    /// Whether a town stands on this cell.
    #[must_use]
    pub fn is_town(&self, cell: Cell) -> bool {
        self.town_at(cell).is_some()
    }

    pub(super) fn prices_of<'a>(&'a self, town: TownAt<'a>) -> &'a BTreeMap<String, u64> {
        match town {
            TownAt::First => &self.prices,
            TownAt::Other(town) => &town.prices,
        }
    }

    pub(super) fn spell_of(&self, town: TownAt<'_>) -> Option<usize> {
        match town {
            TownAt::First => self.spell_for_sale,
            TownAt::Other(town) => town.spell_for_sale,
        }
    }

    pub(super) fn intel_of<'a>(&'a self, town: TownAt<'a>) -> Option<&'a str> {
        match town {
            TownAt::First => self.intel_castle.as_deref(),
            TownAt::Other(town) => town.intel_castle.as_deref(),
        }
    }
}

impl Expedition {
    /// What the town on this cell has left of a troop; nothing where no town stands.
    #[must_use]
    pub fn stock_at(&self, cell: Cell, troop_id: &str) -> u32 {
        let left = match self.def.town_at(cell) {
            Some(TownAt::First) => self.stock.get(troop_id),
            Some(TownAt::Other(town)) => {
                let stock = self.more_stock.get(&town.object_id);
                stock.and_then(|stock| stock.get(troop_id))
            }
            None => None,
        };
        left.copied().unwrap_or(0)
    }

    /// The spell the town on this cell sells.
    #[must_use]
    pub fn spell_on_sale_at(&self, cell: Cell) -> Option<&BookSpell> {
        let town = self.def.town_at(cell)?;
        self.def
            .spell_of(town)
            .map(|place| &self.def.rules.book[place])
    }

    /// Takes creatures out of the stock of the town the hero stands in.
    pub(super) fn sell_from_town(&mut self, troop_id: &str, left: u32) {
        match self.def.town_at(self.hero) {
            Some(TownAt::Other(town)) => {
                let stock = self.more_stock.entry(town.object_id.clone()).or_default();
                stock.insert(troop_id.to_owned(), left);
            }
            _ => {
                self.stock.insert(troop_id.to_owned(), left);
            }
        }
    }
}
