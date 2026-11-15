//! An expedition: the hero on the map, the clock, the purse, the army, and the commands that change them.
//!
//! The same discipline as a battle. The only way to change an [`Expedition`] is [`Expedition::apply`]; a command
//! that is refused changes nothing; what happened is told as events, in order, with IDs and numbers and no text for
//! the screen. One clock owns time: every command that takes time goes through `pass`, which settles each week
//! once and stops at the deadline.

mod content;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

pub use puzzle::{Piece, Puzzle, WINDOW};

pub use content::{
    Alcove, ArmyStart, BandStack, BattleLayout, Chest, Clock, Dwelling, Encounter, EnemyCastle,
    ExpeditionDef, Holds, King, Lord, Map, MapObject, ObjectKind, Recovery, Rotation,
    SIEGE_BESIEGERS, SIEGE_GARRISON, SIEGE_WALLS, SIZES, Terrain, Upkeep,
};

use crate::battle::{self, Battle, Outcome};
use crate::canon::Value;
use crate::content::{
    Ability, BookSpell, Cell, Home, Power, Profile, Rank, Scenario, Side, StackStart, Switches,
    Unpaid,
};
use crate::save::{Reader, SaveError, invalid};

/// How an expedition stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// The hero is on the map and may act.
    Adventure,
    /// A battle is being fought; only its commands and surrender are taken.
    Battle,
    /// The objective was found.
    Victory,
    /// The deadline came first.
    Timeout,
    /// A week's upkeep could not be paid.
    Insolvent,
}

impl Status {
    /// The name used in reports, saves and digests.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Adventure => "adventure",
            Self::Battle => "battle",
            Self::Victory => "victory",
            Self::Timeout => "timeout",
            Self::Insolvent => "insolvent",
        }
    }

    /// The status of that name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        let all = [
            Self::Adventure,
            Self::Battle,
            Self::Victory,
            Self::Timeout,
            Self::Insolvent,
        ];
        all.into_iter().find(|status| status.name() == name)
    }
}

/// A stack of the hero's army between battles. Injury is healed outside battle, so there is none to keep.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArmyStack {
    /// Stable for the whole expedition.
    pub id: String,
    pub troop_id: String,
    pub count: u32,
}

/// What the hero is asked to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// One step north, south, east or west.
    Move { dx: i32, dy: i32 },
    /// Let one tick pass.
    WaitTick,
    /// Buy creatures in the town.
    Recruit { troop_id: String, quantity: u32 },
    /// Take the contract on an encounter, in the town.
    TakeContract { id: String },
    /// Search the ground where the hero stands for the objective.
    Search,
    /// A command of the battle being fought.
    Battle(battle::Command),
    /// Give the battle up.
    Surrender,
    /// Ask the king, in his castle, for the rank the villains caught have earned.
    Promote,
    /// Buy siege weapons in the town: without them no castle is entered.
    BuySiegeWeapons,
    /// In a castle of the hero's own: leave so many of a troop of the army in it.
    Garrison { troop_id: String, quantity: u32 },
    /// In a castle of the hero's own: take so many of a troop left there back into the army.
    Withdraw { troop_id: String, quantity: u32 },
    /// Take the friendly band the hero stands with into the army.
    AcceptBand,
    /// Send the friendly band the hero stands with away.
    DismissBand,
    /// Open the chest the hero stands on and keep what it holds.
    KeepChest,
    /// Open the chest the hero stands on and share its gold out among the men, for leadership.
    ShareChest,
    /// Be taught to cast, at the alcove the hero stands on.
    LearnMagic,
    /// Buy one charge of the spell the town sells.
    BuySpell,
    /// Cast a spell of the map from the hero's book.
    CastOnMap { spell: String, aim: Aim },
}

/// What a spell of the map is cast at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Aim {
    /// Nothing: the spell works on the hero and his army.
    None,
    /// One step north, south, east or west of the hero: where a bridge is laid.
    Towards { dx: i32, dy: i32 },
    /// A place of the map, by its object's ID: where a gate leads.
    Place(String),
}

/// A request to change the expedition. It names what should happen, never the price or the result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Command {
    /// The revision the sender saw; a command made for another state is refused.
    pub revision: u64,
    pub action: Action,
}

/// Why a command was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refused {
    /// The expedition has ended.
    Over,
    /// The command was made for another revision.
    Stale { command: u64, expedition: u64 },
    /// Not one step north, south, east or west.
    NotAStep,
    /// The step leads off the map or onto ground that cannot be walked.
    CannotEnter,
    /// A battle is being fought; the map waits.
    InBattle,
    /// No battle is being fought.
    NotInBattle,
    /// The battle refused its command.
    Battle(battle::RuleError),
    /// Only possible in the town.
    NotInTown,
    /// The town does not sell this troop.
    NotSold { troop_id: String },
    /// Nothing asked for.
    NoQuantity,
    /// More than the town has left.
    OutOfStock { stock: u32 },
    /// More than the purse holds.
    CannotAfford { cost: u64, gold: u64 },
    /// The stack's whole health would exceed the hero's leadership.
    OverLeadership { health: u64, leadership: u32 },
    /// Every slot of the army holds another troop.
    ArmyFull,
    /// No such band to take a contract on, or it is already beaten.
    NoSuchContract { id: String },
    /// The king's castle sells this troop only to a hero of this much leadership: six of its creatures' health.
    NotYetOffered { leadership: u32 },
    /// Only possible in the king's castle; an expedition without a king has none.
    NotAtCastle,
    /// A castle is not entered without siege weapons.
    NoSiegeWeapons,
    /// Only possible in a castle of the hero's own.
    NotInOwnCastle,
    /// There are not so many of that troop to move: `have` is how many there are.
    NotSoMany { have: u32 },
    /// The army may not be left without a creature.
    ArmyEmptied,
    /// Every place in the castle holds another troop.
    CastleFull,
    /// The stack the creatures would join cannot be counted any higher.
    StackTooLarge,
    /// The hero has siege weapons already, or the expedition has no castle to use them on.
    NoSiegeWeaponsToBuy,
    /// The hero stands with no band that would join him.
    NoBandToJoin,
    /// The hero stands on no chest that is still closed.
    NoChest,
    /// The chest holds no gold to share out.
    NothingToShare,
    /// The hero stands on no alcove, or can cast already.
    NoAlcove,
    /// The town sells no spell.
    NoSpellSold,
    /// The hero holds as many spells as he has room for.
    NoRoomForSpell { limit: u32 },
    /// The hero cannot cast: nobody has taught him.
    CannotCast,
    /// No such spell in the book, or no charge of it held.
    NoSpellCharge { spell: String },
    /// A spell of battle, which is cast in a battle only.
    NotCastOnTheMap,
    /// The spell is not cast at that: a direction for a bridge, a place for a gate, nothing for the others.
    WrongAim,
    /// The spell that finds a villain has none to find.
    NothingToFind,
    /// The hero does not face water that a bridge could be laid over.
    NoWaterToBridge,
    /// No gate leads there: not the town or a castle of the hero's own, or he stands there.
    NoGateTo { place: String },
    /// The king has no rank to give: `needed` villains earn the next, or there is none above the hero's.
    NoPromotion { caught: u32, needed: Option<u32> },
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Over => f.write_str("the expedition is over"),
            Self::Stale {
                command,
                expedition,
            } => {
                write!(
                    f,
                    "command is for revision {command}, the expedition is at {expedition}"
                )
            }
            Self::NotAStep => f.write_str("not one step north, south, east or west"),
            Self::CannotEnter => {
                f.write_str("the step leads off the map or onto ground that cannot be walked")
            }
            Self::InBattle => f.write_str("a battle is being fought"),
            Self::NotInBattle => f.write_str("no battle is being fought"),
            Self::Battle(refused) => write!(f, "the battle refused: {refused}"),
            Self::NotInTown => f.write_str("only possible in the town"),
            Self::NotSold { troop_id } => write!(f, "the town does not sell {troop_id:?}"),
            Self::NoQuantity => f.write_str("nothing asked for"),
            Self::OutOfStock { stock } => write!(f, "the town has only {stock} left"),
            Self::CannotAfford { cost, gold } => {
                write!(f, "that costs {cost}, the purse holds {gold}")
            }
            Self::OverLeadership { health, leadership } => {
                write!(
                    f,
                    "a stack of {health} health is more than the leadership of {leadership}"
                )
            }
            Self::ArmyFull => f.write_str("every slot of the army holds another troop"),
            Self::NoSuchContract { id } => write!(f, "no contract to take on {id:?}"),
            Self::NotYetOffered { leadership } => {
                write!(f, "the castle offers that to a leadership of {leadership}")
            }
            Self::NotAtCastle => f.write_str("only possible in the king's castle"),
            Self::NoSiegeWeapons => f.write_str("a castle is not entered without siege weapons"),
            Self::NotInOwnCastle => f.write_str("only possible in a castle of the hero's own"),
            Self::NotSoMany { have } => write!(f, "there are only {have} of that troop to move"),
            Self::ArmyEmptied => f.write_str("the army may not be left without a creature"),
            Self::CastleFull => f.write_str("every place in the castle holds another troop"),
            Self::StackTooLarge => f.write_str("that stack cannot be counted any higher"),
            Self::NoSiegeWeaponsToBuy => f.write_str("there are no siege weapons to buy"),
            Self::NoBandToJoin => f.write_str("there is no band here that would join"),
            Self::NoChest => f.write_str("there is no closed chest here"),
            Self::NothingToShare => f.write_str("the chest holds no gold to share out"),
            Self::NoAlcove => f.write_str("there is no alcove here with magic left to teach"),
            Self::NoSpellSold => f.write_str("the town sells no spell"),
            Self::NoRoomForSpell { limit } => {
                write!(f, "the hero holds all {limit} spells he has room for")
            }
            Self::CannotCast => f.write_str("the hero cannot cast"),
            Self::NoSpellCharge { spell } => write!(f, "no charge of {spell:?}"),
            Self::NotCastOnTheMap => f.write_str("that spell is cast in a battle"),
            Self::WrongAim => f.write_str("the spell is not cast at that"),
            Self::NothingToFind => f.write_str("there is no villain to find"),
            Self::NoWaterToBridge => f.write_str("there is no water there to bridge"),
            Self::NoGateTo { place } => write!(f, "no gate leads to {place:?}"),
            Self::NoPromotion {
                caught,
                needed: Some(needed),
            } => write!(
                f,
                "the next rank asks for {needed} villains, {caught} are caught"
            ),
            Self::NoPromotion { needed: None, .. } => f.write_str("there is no higher rank"),
        }
    }
}

impl std::error::Error for Refused {}

/// Something that happened, in the order it happened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Moved {
        from: Cell,
        to: Cell,
    },
    /// The clock went on, from one tick to another.
    TimePassed {
        from: u32,
        to: u32,
    },
    /// A week ended: income came in, then upkeep went out. `gold` is the purse afterwards.
    WeekSettled {
        week: u32,
        income: u64,
        upkeep: u64,
        gold: u64,
    },
    Recruited {
        troop_id: String,
        quantity: u32,
        cost: u64,
    },
    ContractTaken {
        id: String,
    },
    Searched {
        found: bool,
    },
    /// The hero stepped onto a band and its battle began.
    BattleStarted {
        band: String,
    },
    /// Something happened in the battle.
    Battle(battle::Event),
    /// A stack that could not be paid at the week's end left the army.
    StackLeft {
        troop_id: String,
        count: u32,
    },
    /// After a won battle some of a stack's fallen came back to it.
    FallenReturned {
        troop_id: String,
        count: u32,
    },
    /// The band is beaten; the reward is in the purse.
    BattleWon {
        band: String,
        reward: u64,
    },
    /// A piece of the puzzle was lifted: `lifted` of all its pieces are now, and with `sceptre` the ground it
    /// uncovered is where the sceptre lies.
    PieceLifted {
        lifted: u32,
        of: u32,
        sceptre: bool,
    },
    /// A clue named the place of an object.
    ClueGained {
        object: String,
    },
    /// A week began under its creature.
    WeekCreature {
        week: u32,
        troop_id: String,
    },
    /// The week's creature filled a dwelling of its kind again; `population` is what it holds now.
    DwellingRefilled {
        object: String,
        population: u32,
    },
    /// The week's creature grew in a band: `count` is how many the band has of it now.
    BandGrew {
        band: String,
        troop_id: String,
        count: u32,
    },
    /// In the week of the first troop of the rules, a stack of the army that grows by what it kills became that
    /// troop, creature for creature.
    Turned {
        from: String,
        to: String,
        count: u32,
    },
    SiegeWeaponsBought {
        cost: u64,
    },
    /// Creatures of the army were left in a castle of the hero's.
    Garrisoned {
        castle: String,
        troop_id: String,
        count: u32,
    },
    /// Creatures left in a castle of the hero's were taken back into the army.
    Withdrawn {
        castle: String,
        troop_id: String,
        count: u32,
    },
    /// A castle of the hero's stood empty when the week ended: it is held against him again, with the garrison it
    /// first had.
    CastleLost {
        castle: String,
    },
    /// A castle fell and is the hero's.
    CastleTaken {
        castle: String,
    },
    /// A castle fell while the contract on its lord was not held: he goes free, his castle is his again with its
    /// garrison as it first was, and he has to be fought again with the contract.
    LordFreed {
        castle: String,
    },
    /// A friendly band joined the army.
    BandJoined {
        band: String,
        troop_id: String,
        count: u32,
    },
    /// A friendly band went its way: sent off, or with no place for it in the army.
    BandLeft {
        band: String,
    },
    /// A chest was opened and its gold kept.
    ChestKept {
        object: String,
        gold: u64,
    },
    /// A chest was opened and its gold shared out: the hero's leadership rose by `leadership`.
    ChestShared {
        object: String,
        leadership: u32,
    },
    /// A chest was opened and held a commission: so much more every week.
    ChestCommission {
        object: String,
        commission: u64,
    },
    /// A chest was opened and held spell power: so much more of it.
    ChestPower {
        object: String,
        power: u32,
    },
    /// A chest was opened and held room for so many more spells.
    ChestLimit {
        object: String,
        limit: u32,
    },
    /// A chest was opened and held charges of a spell.
    ChestSpell {
        object: String,
        spell: String,
        count: u32,
    },
    /// The hero was taught to cast at the alcove.
    MagicLearned {
        cost: u64,
    },
    /// A charge of a spell was bought in the town; `held` is how many spells the hero holds now.
    SpellBought {
        spell: String,
        cost: u64,
        held: u32,
    },
    /// The hero stepped onto an artefact, and it is his.
    ArtefactFound {
        object: String,
        artefact: String,
    },
    /// A spell of the map was cast; what it did follows.
    MapSpellCast {
        spell: String,
    },
    /// Time stands for so many steps more.
    TimeStopped {
        steps: u32,
    },
    /// The hero's leadership is so much higher until the week ends.
    ControlRaised {
        leadership: u32,
    },
    /// Creatures raised by a spell joined the army.
    ArmyRaised {
        troop_id: String,
        count: u32,
    },
    /// Water was bridged and can be walked.
    Bridged {
        cells: Vec<Cell>,
    },
    /// A gate carried the hero.
    Gated {
        from: Cell,
        to: Cell,
    },
    /// A band was beaten while its contract was held: a villain caught. `caught` is how many there are now.
    VillainCaught {
        band: String,
        caught: u32,
    },
    /// The king gave the hero a rank, or several at once: `rank` is the one he holds now, `ranks` how many he
    /// rose, and the rest what he has with them.
    Promoted {
        rank: String,
        ranks: u32,
        leadership: u32,
        commission: u64,
    },
    /// The battle ran out of rounds with nobody winning; the hero stays where he stood and the band remains.
    BattleStalled {
        band: String,
    },
    /// A battle was lost or given up: the hero is back at the place of recovery with what recovery leaves him.
    Recovered {
        gold: u64,
    },
    Ended {
        status: Status,
    },
}

/// How a stretch of time ended.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Passed {
    /// All of it passed and the expedition goes on.
    All,
    /// The deadline was reached on its last tick: what the command was for still happens.
    AtDeadline,
    /// The deadline cut it short: what the command was for does not happen.
    CutShort,
    /// A week could not be paid.
    Insolvent,
}

/// A battle of the expedition while it is fought.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Engagement {
    battle: Battle,
    band: String,
    arrival: Cell,
}

/// The traveller's number for so many creatures: the largest of the reference's sizes they reach, `None` for none
/// at all.
#[must_use]
pub fn size_of(count: u32) -> Option<u32> {
    SIZES.iter().rev().find(|&&least| count >= least).copied()
}

/// A troop a place sells to the hero as he is now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer {
    pub troop_id: String,
    /// For one creature.
    pub price: u64,
    /// How many are left; `None` where the place does not run out.
    pub left: Option<u32>,
}

/// How an enemy castle stands.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Hold {
    /// The hero has taken it.
    taken: bool,
    /// Its lord is caught, whoever holds the castle now.
    caught: bool,
    /// How many each stack of its garrison has, while it is not the hero's.
    garrison: Vec<u32>,
    /// The stacks the hero has left in it, while it is his: troop and number, in the order they were left.
    own: Vec<(String, u32)>,
}

/// What was done with a chest that has been opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Taken {
    Kept,
    Shared,
}

/// The hero's magic, in an expedition that has any.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Magic {
    /// He can cast: his class could from the start, or the alcove taught him.
    pub knows: bool,
    /// The charges he holds of each spell of the rules' book, in the book's order.
    pub charges: Vec<u32>,
    /// Steps he can still take while time stands; gone when time passes in any other way.
    pub free_steps: u32,
    /// Leadership a spell has raised until the week ends.
    pub raised: u32,
    /// The cells of water he has bridged, in the order he did.
    pub bridges: Vec<Cell>,
}

/// Gold shared out buys leadership at this much gold a point.
const GOLD_A_POINT: u64 = 50;

/// Where a hero who serves a king stands with him.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Standing {
    /// Place of the hero's rank in his class.
    pub rank: usize,
    /// What the ranks reached give together.
    pub leadership: u32,
    /// Gold from the king every week, likewise.
    pub commission: u64,
    /// Bands beaten while their contract was held.
    pub villains: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expedition {
    def: Arc<ExpeditionDef>,
    /// Goes up by one with every accepted command.
    revision: u64,
    hero: Cell,
    /// Ticks elapsed since the start.
    ticks: u32,
    gold: u64,
    /// In slot order.
    army: Vec<ArmyStack>,
    /// What the town has left, by troop ID.
    stock: BTreeMap<String, u32>,
    /// The band whose contract the hero holds.
    contract: Option<String>,
    /// Objects whose place a clue has named.
    clues: BTreeSet<String>,
    /// Bands that are beaten.
    cleared: BTreeSet<String>,
    /// The battle being fought, with the band and the cell the hero arrives on if he wins.
    engagement: Option<Engagement>,
    /// Only in an expedition with a king.
    standing: Option<Standing>,
    /// What each dwelling has left, by its object's ID.
    populations: BTreeMap<String, u32>,
    /// How many each stack of each band has, by the band's object ID, where the weeks have a creature and bands
    /// grow; empty where they do not, and a band is then what its file says.
    band_counts: BTreeMap<String, Vec<u32>>,
    /// The chests that are open, by their object's ID, each with what was done with it.
    opened: BTreeMap<String, Taken>,
    /// The castles that are not the king's, by their object's ID.
    holds: BTreeMap<String, Hold>,
    /// Where the town gives contracts in turn: the villains on offer, a place for each, empty once its villain is
    /// caught and none is left to follow him; the place whose contract was given last; and how many villains
    /// have come into the rotation.
    cycle: Vec<Option<String>>,
    last_given: usize,
    entered: usize,
    siege_weapons: bool,
    /// The hero's creatures that fell in battle, whether or not any came back; counted only where the expedition
    /// is scored.
    fallen: u64,
    /// The artefacts found, by their IDs in the rules, each with what it has added where it doubles something.
    found: BTreeMap<String, u32>,
    /// Only in an expedition with magic.
    magic: Option<Magic>,
    status: Status,
}

/// The magic of a saved expedition: whether the hero casts, and the spells he holds charges of, each by its ID
/// with more than none; nothing where the expedition has no magic.
fn saved_magic(def: &ExpeditionDef, state: &Reader<'_>) -> Result<Option<Magic>, SaveError> {
    if !def.has_magic() {
        return Ok(None);
    }
    let magic = state.object("magic")?;
    let mut charges = vec![0; def.rules.book.len()];
    for (spell, count) in magic.numbers("book")? {
        let place = def.rules.book.iter().position(|s| s.id == spell);
        match place {
            Some(place) if count > 0 => charges[place] = count,
            _ => {
                return invalid(format!(
                    "the spell {spell:?} is held, of no charge or not of the rules"
                ));
            }
        }
    }
    let mut bridges = Vec::new();
    for cell in magic.list("bridges")? {
        let from = Reader::of(cell, "a bridged cell")?;
        bridges.push(Cell {
            x: from.u32("x")?,
            y: from.u32("y")?,
        });
    }
    Ok(Some(Magic {
        knows: magic.flag("knows")?,
        charges,
        free_steps: magic.u32("free_steps")?,
        raised: magic.u32("raised")?,
        bridges,
    }))
}

/// The open chests of a saved expedition, each with what was done with it; none where there are no chests.
fn saved_chests(
    def: &ExpeditionDef,
    state: &Reader<'_>,
) -> Result<BTreeMap<String, Taken>, SaveError> {
    let mut opened = BTreeMap::new();
    if !def.chests.is_empty() {
        for (object, taken) in state.named_texts("chests")? {
            let taken = match taken.as_str() {
                "kept" => Taken::Kept,
                "shared" => Taken::Shared,
                _ => return invalid(format!("chest {object:?} is neither kept nor shared")),
            };
            opened.insert(object, taken);
        }
    }
    Ok(opened)
}

/// The contracts' rotation of a saved expedition: its places, the one given last, and how many villains have
/// come into it; nothing where the town does not give contracts in turn.
fn saved_rotation(
    def: &ExpeditionDef,
    state: &Reader<'_>,
) -> Result<(Vec<Option<String>>, usize, usize), SaveError> {
    if def.rotation.is_none() {
        return Ok((Vec::new(), 0, 0));
    }
    let rotation = state.object("rotation")?;
    let place = |villain: &Value| match villain {
        Value::Null => Ok(None),
        Value::Str(id) => Ok(Some(id.clone())),
        _ => invalid("a place of the rotation holds neither a villain nor nothing"),
    };
    let cycle: Result<Vec<Option<String>>, SaveError> =
        rotation.list("cycle")?.iter().map(place).collect();
    let number = |key| -> Result<usize, SaveError> {
        Ok(usize::try_from(rotation.u64(key)?).unwrap_or(usize::MAX))
    };
    Ok((cycle?, number("last_given")?, number("entered")?))
}

/// How the castles of a saved expedition stand, and whether the hero has siege weapons; nothing where there are
/// no castles to besiege.
fn saved_holds(
    def: &ExpeditionDef,
    state: &Reader<'_>,
) -> Result<(BTreeMap<String, Hold>, bool), SaveError> {
    let mut holds = BTreeMap::new();
    let mut siege_weapons = false;
    if !def.castles.is_empty() {
        siege_weapons = state.flag("siege_weapons")?;
        let castles = state.object("castles")?;
        for castle in &def.castles {
            let hold = castles.object(&castle.object_id)?;
            let garrison = hold.list("garrison")?.iter().map(|count| match count {
                Value::Int(n) => u32::try_from(*n).ok(),
                _ => None,
            });
            let Some(garrison) = garrison.collect::<Option<Vec<u32>>>() else {
                return invalid(format!(
                    "the garrison of {:?} is not numbers",
                    castle.object_id
                ));
            };
            let mut own = Vec::new();
            for stack in hold.list("own")? {
                let stack = Reader::of(stack, "a stack left in a castle")?;
                own.push((stack.text("troop_id")?.to_owned(), stack.u32("count")?));
            }
            let (taken, caught) = (hold.flag("taken")?, hold.flag("caught")?);
            let hold = Hold {
                taken,
                caught,
                garrison,
                own,
            };
            holds.insert(castle.object_id.clone(), hold);
        }
    }
    Ok((holds, siege_weapons))
}

impl Expedition {
    /// The expedition at its start.
    #[must_use]
    pub fn start(def: Arc<ExpeditionDef>) -> Self {
        let army = def.start_army.iter();
        let army = army
            .map(|s| ArmyStack {
                id: s.id.clone(),
                troop_id: s.troop_id.clone(),
                count: s.count,
            })
            .collect();
        let standing = def.king.as_ref().map(|_| Standing {
            rank: 0,
            leadership: def.leadership,
            commission: def.clock.weekly_income,
            villains: 0,
        });
        let populations = def.dwellings.iter().map(|dwelling| {
            let troop = def.rules.troop(&dwelling.troop);
            let full = troop.map_or(0, |troop| troop.max_population);
            (dwelling.object_id.clone(), full)
        });
        let magic = def.has_magic().then(|| Magic {
            knows: def.casts_from_start(),
            charges: vec![0; def.rules.book.len()],
            free_steps: 0,
            raised: 0,
            bridges: Vec::new(),
        });
        Self {
            standing,
            magic,
            populations: populations.collect(),
            band_counts: Self::bands_at_start(&def),
            opened: BTreeMap::new(),
            holds: Self::holds_at_start(&def),
            cycle: Self::cycle_at_start(&def),
            last_given: Self::cycle_at_start(&def).len().saturating_sub(1),
            entered: Self::cycle_at_start(&def).len(),
            siege_weapons: false,
            fallen: 0,
            found: BTreeMap::new(),
            hero: def.start,
            gold: def.start_gold,
            stock: def.stock.clone(),
            army,
            def,
            revision: 0,
            ticks: 0,
            contract: None,
            clues: BTreeSet::new(),
            cleared: BTreeSet::new(),
            engagement: None,
            status: Status::Adventure,
        }
    }

    #[must_use]
    pub fn def(&self) -> &ExpeditionDef {
        &self.def
    }

    /// The definition as the expedition holds it, for another expedition of the same: a loaded save.
    #[must_use]
    pub const fn def_shared(&self) -> &Arc<ExpeditionDef> {
        &self.def
    }

    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    #[must_use]
    pub const fn hero(&self) -> Cell {
        self.hero
    }

    #[must_use]
    pub const fn ticks(&self) -> u32 {
        self.ticks
    }

    #[must_use]
    pub const fn gold(&self) -> u64 {
        self.gold
    }

    /// The hero's standing with the king, in an expedition that has one.
    #[must_use]
    pub const fn standing(&self) -> Option<&Standing> {
        self.standing.as_ref()
    }

    /// The hero's rank, where there is a king to give one.
    #[must_use]
    pub fn rank(&self) -> Option<&Rank> {
        let (king, standing) = (self.def.king.as_ref()?, self.standing.as_ref()?);
        self.def.rules.classes[king.class].ranks.get(standing.rank)
    }

    /// The rank above the hero's, which the king gives for the villains it asks for; `None` at the highest, and
    /// without a king.
    #[must_use]
    pub fn next_rank(&self) -> Option<&Rank> {
        let (king, standing) = (self.def.king.as_ref()?, self.standing.as_ref()?);
        self.def.rules.classes[king.class]
            .ranks
            .get(standing.rank + 1)
    }

    /// The hero's leadership now: what his ranks give, or what the expedition starts with.
    #[must_use]
    pub fn leadership(&self) -> u32 {
        let base = self
            .standing
            .as_ref()
            .map_or(self.def.leadership, |s| s.leadership);
        let raised = self.magic.as_ref().map_or(0, |magic| magic.raised);
        let doubled = self.gain_of(Power::DoubleLeadership).unwrap_or(0);
        let lasting = base.saturating_add(self.chest_gains().0);
        lasting.saturating_add(doubled).saturating_add(raised)
    }

    /// The leadership that is the hero's for good, before any artefact: his ranks' and his chests'.
    fn leadership_for_good(&self) -> u32 {
        let base = self
            .standing
            .as_ref()
            .map_or(self.def.leadership, |s| s.leadership);
        base.saturating_add(self.chest_gains().0)
    }

    /// The gold a week brings now.
    #[must_use]
    pub fn weekly_income(&self) -> u64 {
        let base = self
            .standing
            .as_ref()
            .map_or(self.def.clock.weekly_income, |s| s.commission);
        let chests = self.chest_gains().1;
        base.saturating_add(chests)
            .saturating_add(self.commission_of_artefacts())
    }

    /// What the opened chests have added for good: leadership, from the gold shared out, and weekly commission.
    fn chest_gains(&self) -> (u32, u64) {
        let mut gained = (0_u32, 0_u64);
        for chest in &self.def.chests {
            match (chest.holds, self.opened.get(&chest.object_id)) {
                (Holds::Gold(gold), Some(Taken::Shared)) => {
                    let points = u32::try_from(gold / GOLD_A_POINT).unwrap_or(u32::MAX);
                    gained.0 = gained.0.saturating_add(points);
                }
                (Holds::Commission(more), Some(_)) => gained.1 = gained.1.saturating_add(more),
                _ => {}
            }
        }
        gained
    }

    /// The hero's magic, in an expedition that has any.
    #[must_use]
    pub const fn magic(&self) -> Option<&Magic> {
        self.magic.as_ref()
    }

    /// The ranks the hero has reached, the one he holds last; none without a king.
    fn ranks_reached(&self) -> &[Rank] {
        let (Some(king), Some(standing)) = (&self.def.king, &self.standing) else {
            return &[];
        };
        let ranks = &self.def.rules.classes[king.class].ranks;
        ranks.get(..=standing.rank).unwrap_or(&[])
    }

    /// The hero's spell power: what each rank reached gives, and the chests that held more.
    #[must_use]
    pub fn spell_power(&self) -> u32 {
        let ranks = self.ranks_reached().iter().map(|rank| rank.spell_power);
        let chests = self.opened_chests().filter_map(|holds| match holds {
            Holds::Power(more) => Some(more),
            _ => None,
        });
        let doubled = self.gain_of(Power::DoubleSpellPower);
        ranks
            .chain(chests)
            .chain(doubled)
            .fold(0, u32::saturating_add)
    }

    /// How many spells the hero has room for: what each rank reached gives, and the chests that held more.
    #[must_use]
    pub fn spell_limit(&self) -> u32 {
        let ranks = self.ranks_reached().iter().map(|rank| rank.max_spells);
        let chests = self.opened_chests().filter_map(|holds| match holds {
            Holds::Limit(more) => Some(more),
            _ => None,
        });
        let doubled = self.gain_of(Power::DoubleMaxSpells);
        ranks
            .chain(chests)
            .chain(doubled)
            .fold(0, u32::saturating_add)
    }

    /// What the chests that are open held.
    fn opened_chests(&self) -> impl Iterator<Item = Holds> + '_ {
        let open = |chest: &&Chest| self.opened.contains_key(&chest.object_id);
        self.def.chests.iter().filter(open).map(|chest| chest.holds)
    }

    /// How many spells the hero holds: every charge of every spell counts as one.
    #[must_use]
    pub fn spells_held(&self) -> u32 {
        let charges = self
            .magic
            .iter()
            .flat_map(|magic| magic.charges.iter().copied());
        charges.fold(0, u32::saturating_add)
    }

    /// How many charges the hero holds of this spell.
    #[must_use]
    pub fn charges(&self, spell_id: &str) -> u32 {
        let place = self
            .def
            .rules
            .book
            .iter()
            .position(|spell| spell.id == spell_id);
        let held = place.zip(self.magic.as_ref());
        held.map_or(0, |(place, magic)| magic.charges[place])
    }

    /// What the hero takes of his magic into a battle: his charges of the spells of battle and his spell power,
    /// where he can cast and holds any; nothing otherwise, and the battle is then one without magic.
    fn magic_for_battle(&self) -> (BTreeMap<String, u32>, Option<u32>) {
        let Some(magic) = self.magic.as_ref().filter(|magic| magic.knows) else {
            return (BTreeMap::new(), None);
        };
        let book = self.def.rules.book.iter().zip(&magic.charges);
        let held = book.filter(|(spell, charges)| spell.battle.is_some() && **charges > 0);
        let charges: BTreeMap<String, u32> = held
            .map(|(spell, charges)| (spell.id.clone(), *charges))
            .collect();
        let power = (!charges.is_empty()).then(|| self.spell_power().max(1));
        (charges, power)
    }

    /// The spell the town sells.
    #[must_use]
    pub fn spell_on_sale(&self) -> Option<&BookSpell> {
        self.def
            .spell_for_sale
            .map(|place| &self.def.rules.book[place])
    }

    /// The alcove the hero stands on, while he cannot cast: once he can, it has nothing to teach him.
    #[must_use]
    pub fn alcove_here(&self) -> Option<&Alcove> {
        let alcove = self.def.alcove.as_ref()?;
        let learned = self.magic.as_ref().is_some_and(|magic| magic.knows);
        (alcove.cell == self.hero && !learned).then_some(alcove)
    }

    /// The chest the hero stands on, while it is closed.
    #[must_use]
    pub fn chest_here(&self) -> Option<&Chest> {
        let chest = self.def.chest_at(self.hero)?;
        (!self.opened.contains_key(&chest.object_id)).then_some(chest)
    }

    /// Whether the chest on this object has been opened.
    #[must_use]
    pub fn is_opened(&self, object_id: &str) -> bool {
        self.opened.contains_key(object_id)
    }

    #[must_use]
    pub fn army(&self) -> &[ArmyStack] {
        &self.army
    }

    /// What the town has left of a troop.
    #[must_use]
    pub fn stock(&self, troop_id: &str) -> u32 {
        self.stock.get(troop_id).copied().unwrap_or(0)
    }

    /// Every enemy castle as the file has it: not taken, with its first garrison.
    fn holds_at_start(def: &ExpeditionDef) -> BTreeMap<String, Hold> {
        let first = |castle: &EnemyCastle| Hold {
            taken: false,
            caught: false,
            garrison: castle.garrison.iter().map(|stack| stack.count).collect(),
            own: Vec::new(),
        };
        let castles = def.castles.iter();
        castles
            .map(|castle| (castle.object_id.clone(), first(castle)))
            .collect()
    }

    /// The villains the rotation starts with: the first of its list, as many as it has places.
    fn cycle_at_start(def: &ExpeditionDef) -> Vec<Option<String>> {
        let Some(rotation) = &def.rotation else {
            return Vec::new();
        };
        let first = rotation
            .villains
            .iter()
            .take(rotation.cycle_length as usize);
        first.cloned().map(Some).collect()
    }

    /// The place of the rotation whose contract the town gives next: the first after the last given that still
    /// has a villain, going round.
    fn next_place(&self) -> Option<usize> {
        let places = self.cycle.len();
        (1..=places)
            .map(|step| (self.last_given + step) % places)
            .find(|&place| self.cycle[place].is_some())
    }

    /// What the town has heard of the castle it has word of: who holds it, and its garrison as a traveller would
    /// tell it, each troop with the smallest of the reference's sizes it reaches (`content::SIZES`). Nothing
    /// where the town has word of no castle, or the castle is the hero's.
    #[must_use]
    pub fn word_of_castle(&self) -> Option<(&str, Vec<(String, u32)>)> {
        let castle = self.def.intel_castle.as_deref()?;
        if self.is_taken(castle) {
            return None;
        }
        let garrison = self.garrison(castle).into_iter();
        let told = garrison.filter_map(|(troop, count)| Some((troop, size_of(count)?)));
        Some((castle, told.collect()))
    }

    /// Whether the hero has taken the castle on this object.
    #[must_use]
    pub fn is_taken(&self, object_id: &str) -> bool {
        self.holds.get(object_id).is_some_and(|hold| hold.taken)
    }

    /// Who stands in the castle on this object now, troop and number: its garrison while it is not the hero's,
    /// what he has left in it once it is. Empty for an object that is no such castle.
    #[must_use]
    pub fn garrison(&self, object_id: &str) -> Vec<(String, u32)> {
        let (Some(castle), Some(hold)) = (self.def.castle(object_id), self.holds.get(object_id))
        else {
            return Vec::new();
        };
        if hold.taken {
            return hold.own.clone();
        }
        let stacks = castle.garrison.iter().zip(&hold.garrison);
        stacks
            .map(|(stack, count)| (stack.troop.clone(), *count))
            .collect()
    }

    #[must_use]
    pub const fn has_siege_weapons(&self) -> bool {
        self.siege_weapons
    }

    /// The lord of the castle on this object, while he is not caught.
    fn lord_at_large(&self, object_id: &str) -> Option<&Lord> {
        let castle = self.def.castle(object_id)?;
        let caught = self.holds.get(object_id).is_some_and(|hold| hold.caught);
        castle.lord.as_ref().filter(|_| !caught)
    }

    /// Whom a contract can be taken on now, by the ID of their object: the bands that still stand, in the order of
    /// the expedition's file, then the lords at large in the order of their castles.
    #[must_use]
    pub fn contracts_on_offer(&self) -> Vec<&str> {
        let def = &self.def;
        if def.rotation.is_some() {
            // In turn: only the villain whose turn it is.
            let next = self
                .next_place()
                .and_then(|place| self.cycle[place].as_deref());
            return next.into_iter().collect();
        }
        let bands = def.encounters.iter().map(|band| band.object_id.as_str());
        let hostile = |id: &&str| def.encounter(id).is_some_and(|band| !band.friendly);
        let bands = bands.filter(|id| hostile(id) && !self.is_cleared(id));
        let lords = def.castles.iter().map(|castle| castle.object_id.as_str());
        let lords = lords.filter(|id| self.lord_at_large(id).is_some());
        bands.chain(lords).collect()
    }

    /// The castle of his own the hero stands in.
    #[must_use]
    pub fn own_castle_here(&self) -> Option<&str> {
        let object = self.def.map.object_at(self.hero)?;
        self.is_taken(&object.id).then_some(object.id.as_str())
    }

    /// The counts of every band's stacks as the file has them, in an expedition whose bands grow.
    fn bands_at_start(def: &ExpeditionDef) -> BTreeMap<String, Vec<u32>> {
        if def.clock.creature_seed.is_none() {
            return BTreeMap::new();
        }
        let counts = |band: &Encounter| band.stacks().into_iter().map(|(_, count)| count).collect();
        let bands = def.encounters.iter();
        bands
            .map(|band| (band.object_id.clone(), counts(band)))
            .collect()
    }

    /// The stacks of the band on this object as they stand now, its first first: troop and number. Empty for an
    /// object that is no band.
    #[must_use]
    pub fn band(&self, object_id: &str) -> Vec<(String, u32)> {
        let Some(encounter) = self.def.encounter(object_id) else {
            return Vec::new();
        };
        let grown = self.band_counts.get(object_id);
        let stacks = encounter.stacks().into_iter().enumerate();
        stacks
            .map(|(place, (troop, count))| {
                let now = grown.and_then(|counts| counts.get(place)).copied();
                (troop.to_owned(), now.unwrap_or(count))
            })
            .collect()
    }

    /// What the dwelling on this object has left.
    #[must_use]
    pub fn population(&self, object_id: &str) -> u32 {
        self.populations.get(object_id).copied().unwrap_or(0)
    }

    /// The creature of the week that is running, in an expedition whose weeks have one.
    #[must_use]
    pub fn week_creature(&self) -> Option<&str> {
        let week = self.ticks / self.def.clock.week_ticks;
        self.creature_of(week).map(|troop| troop.id.as_str())
    }

    /// The creature of a week, as the reference picks it: the first troop of the rules every fourth week, the
    /// first included; any other by a fixed sum over the seed and the week's number, so that a week has the same
    /// creature whenever it is asked for.
    fn creature_of(&self, week: u32) -> Option<&crate::content::Troop> {
        let seed = self.def.clock.creature_seed?;
        let troops = &self.def.rules.troops;
        let others = u32::try_from(troops.len()).ok()?.checked_sub(1)?;
        let place = if week.is_multiple_of(4) || others == 0 {
            0
        } else {
            let mixed = (seed ^ week)
                .wrapping_mul(1_664_525)
                .wrapping_add(1_013_904_223);
            1 + mixed % others
        };
        troops.get(place as usize)
    }

    /// What the place on this cell sells to the hero as he is now, wherever he stands: the town's wares in the
    /// order of their names, a dwelling's troop, or those of the king's castle troops his leadership is offered,
    /// in the order of the rules. Nothing for a cell that sells nothing.
    #[must_use]
    pub fn on_sale_at(&self, cell: Cell) -> Vec<Offer> {
        let def = &self.def;
        let offer = |troop_id: &str, price: u64, left: Option<u32>| Offer {
            troop_id: troop_id.to_owned(),
            price,
            left,
        };
        if cell == def.town {
            let wares = def.prices.iter();
            return wares
                .map(|(troop, price)| offer(troop, *price, Some(self.stock(troop))))
                .collect();
        }
        if let Some(dwelling) = def.dwelling_at(cell) {
            let troop = def.rules.troop(&dwelling.troop);
            let left = self.population(&dwelling.object_id);
            return troop
                .map(|troop| offer(&troop.id, u64::from(troop.recruit_cost), Some(left)))
                .into_iter()
                .collect();
        }
        if def.king.as_ref().is_some_and(|king| king.castle == cell) {
            let offered = |troop: &&crate::content::Troop| {
                troop.home == Some(Home::Castle) && troop.hp.saturating_mul(6) <= self.leadership()
            };
            let troops = def.rules.troops.iter().filter(offered);
            return troops
                .map(|troop| offer(&troop.id, u64::from(troop.recruit_cost), None))
                .collect();
        }
        Vec::new()
    }

    /// The expedition as it would be had the hero bought this at the place on `cell`: for a player weighing a
    /// purchase before walking there. Nothing changes; the revision is not counted on.
    ///
    /// # Errors
    /// [`Refused`] as the purchase would be refused there.
    pub fn after_buying_at(
        &self,
        cell: Cell,
        troop_id: &str,
        quantity: u32,
    ) -> Result<Self, Refused> {
        let mut there = self.clone();
        there.hero = cell;
        there.recruit(troop_id, quantity, &mut Vec::new())?;
        Ok(there)
    }

    /// What the place the hero stands on sells of this troop: its price and how many are left, `None` for as
    /// many as are asked for.
    fn offer(&self, troop_id: &str) -> Result<(u64, Option<u32>), Refused> {
        let def = &self.def;
        let not_sold = || Refused::NotSold {
            troop_id: troop_id.to_owned(),
        };
        if self.hero == def.town {
            let price = def.prices.get(troop_id).ok_or_else(not_sold)?;
            return Ok((*price, Some(self.stock(troop_id))));
        }
        let troop = def.rules.troop(troop_id);
        if let Some(dwelling) = def.dwelling_at(self.hero) {
            let troop = troop.filter(|_| dwelling.troop == troop_id);
            let troop = troop.ok_or_else(not_sold)?;
            let left = self.population(&dwelling.object_id);
            return Ok((u64::from(troop.recruit_cost), Some(left)));
        }
        if def
            .king
            .as_ref()
            .is_some_and(|king| king.castle == self.hero)
        {
            let troop = troop.filter(|troop| troop.home == Some(Home::Castle));
            let troop = troop.ok_or_else(not_sold)?;
            // Six creatures of it must be within the hero's leadership before the castle offers the troop.
            let leadership = troop.hp.saturating_mul(6);
            if self.leadership() < leadership {
                return Err(Refused::NotYetOffered { leadership });
            }
            return Ok((u64::from(troop.recruit_cost), None));
        }
        Err(Refused::NotInTown)
    }

    #[must_use]
    pub fn contract(&self) -> Option<&str> {
        self.contract.as_deref()
    }

    #[must_use]
    pub fn is_cleared(&self, object_id: &str) -> bool {
        self.cleared.contains(object_id)
    }

    /// The battle being fought, if one is.
    #[must_use]
    pub fn battle(&self) -> Option<&Battle> {
        self.engagement
            .as_ref()
            .map(|engagement| &engagement.battle)
    }

    /// The band being fought, if one is.
    #[must_use]
    pub fn engaged_with(&self) -> Option<&str> {
        self.engagement
            .as_ref()
            .map(|engagement| engagement.band.as_str())
    }

    #[must_use]
    pub const fn status(&self) -> Status {
        self.status
    }

    /// Where the objective lies, once a clue has named the place. Without a clue the player is not told, though a
    /// search there finds it all the same.
    #[must_use]
    pub fn objective_known(&self) -> Option<Cell> {
        if self.def.puzzle.is_some() {
            return self.sceptre_shown();
        }
        let named = self.def.objective_id.as_ref();
        let named = named.is_some_and(|objective| self.clues.contains(objective));
        named.then(|| self.def.objective())
    }

    /// What the army as it stands costs at the end of a week.
    #[must_use]
    pub fn upkeep(&self) -> u64 {
        self.army.iter().map(|stack| self.upkeep_of(stack)).sum()
    }

    fn upkeep_of(&self, stack: &ArmyStack) -> u64 {
        let each = match self.def.clock.upkeep {
            Upkeep::PerCreature(each) => each,
            Upkeep::TenthOfCost => {
                let troop = self.def.rules.troop(&stack.troop_id);
                troop.map_or(0, |troop| u64::from(troop.recruit_cost / 10))
            }
        };
        each.saturating_mul(u64::from(stack.count))
    }

    /// The switches of the rules the expedition is played under; all off under rules that have none.
    fn switches(&self) -> Switches {
        match &self.def.rules.profile {
            Profile::Reference(reference) => reference.switches,
            Profile::Prototype => Switches::default(),
        }
    }

    /// Takes a week's upkeep from the purse and gives what was paid, or `None` when the week cannot be paid and
    /// the rules end the expedition for it. Under the reference rules a purse too small is emptied and every
    /// stack stays; under the variant's switch the stacks are paid in slot order, each in full or not at all,
    /// and a stack not paid leaves.
    fn pay_upkeep(&mut self, events: &mut Vec<Event>) -> Option<u64> {
        let due = self.upkeep();
        if self.switches().unpaid == Unpaid::Leave {
            let mut paid = 0;
            let army = std::mem::take(&mut self.army);
            for stack in army {
                let cost = self.upkeep_of(&stack);
                if cost <= self.gold {
                    self.gold -= cost;
                    paid += cost;
                    self.army.push(stack);
                } else {
                    events.push(Event::StackLeft {
                        troop_id: stack.troop_id,
                        count: stack.count,
                    });
                }
            }
            return Some(paid);
        }
        if self.gold >= due {
            self.gold -= due;
            return Some(due);
        }
        match self.def.rules.profile {
            Profile::Prototype => None,
            Profile::Reference(_) => Some(std::mem::take(&mut self.gold)),
        }
    }

    /// Every living creature of the army.
    #[must_use]
    pub fn creatures(&self) -> u64 {
        self.army.iter().map(|stack| u64::from(stack.count)).sum()
    }

    /// Why this action would be refused now, or `None` when it would be carried out. Nothing changes either way:
    /// a screen asks this to offer only what the rules allow.
    #[must_use]
    pub fn refusal(&self, action: &Action) -> Option<Refused> {
        let command = Command {
            revision: self.revision,
            action: action.clone(),
        };
        self.clone().apply(&command).err()
    }

    /// Carries out a command, or refuses it and leaves the expedition exactly as it was.
    ///
    /// # Errors
    /// [`Refused`] saying why the command is not allowed now.
    pub fn apply(&mut self, command: &Command) -> Result<Vec<Event>, Refused> {
        let mut next = self.clone();
        let mut events = Vec::new();
        next.carry_out(command, &mut events)?;
        next.revision += 1;
        *self = next;
        Ok(events)
    }

    fn carry_out(&mut self, command: &Command, events: &mut Vec<Event>) -> Result<(), Refused> {
        if !matches!(self.status, Status::Adventure | Status::Battle) {
            return Err(Refused::Over);
        }
        if command.revision != self.revision {
            return Err(Refused::Stale {
                command: command.revision,
                expedition: self.revision,
            });
        }
        let fighting = self.status == Status::Battle;
        match &command.action {
            Action::Battle(order) if fighting => self.fight(order, events),
            Action::Surrender if fighting => {
                self.recover(events);
                Ok(())
            }
            Action::Battle(_) | Action::Surrender => Err(Refused::NotInBattle),
            _ if fighting => Err(Refused::InBattle),
            Action::Move { dx, dy } => self.step(*dx, *dy, events),
            Action::WaitTick => {
                self.pass(1, events);
                self.end_if_due(events);
                Ok(())
            }
            Action::Recruit { troop_id, quantity } => self.recruit(troop_id, *quantity, events),
            Action::TakeContract { id } => self.take_contract(id, events),
            Action::Search => {
                self.search(events);
                Ok(())
            }
            Action::Promote => self.promote(events),
            Action::BuySiegeWeapons => self.buy_siege_weapons(events),
            Action::Garrison { troop_id, quantity } => {
                self.garrison_with(troop_id, *quantity, events)
            }
            Action::Withdraw { troop_id, quantity } => self.withdraw(troop_id, *quantity, events),
            Action::AcceptBand => self.meet_band(true, events),
            Action::DismissBand => self.meet_band(false, events),
            Action::KeepChest => self.open_chest(Taken::Kept, events),
            Action::ShareChest => self.open_chest(Taken::Shared, events),
            Action::LearnMagic => self.learn_magic(events),
            Action::BuySpell => self.buy_spell(events),
            Action::CastOnMap { spell, aim } => self.cast_on_map(spell, aim, events),
        }
    }

    /// The hostile bands that stand in one of the four cells beside the hero, each with its stacks in a
    /// traveller's numbers: what he sees of a band before he chooses to step onto it or go round.
    #[must_use]
    pub fn bands_beside(&self) -> Vec<(&str, Vec<(String, u32)>)> {
        let here = self.hero;
        let beside = |cell: Cell| here.x.abs_diff(cell.x) + here.y.abs_diff(cell.y) == 1;
        let bands = self.def.encounters.iter().filter(|band| {
            let object = self.def.map.object(&band.object_id);
            let near = object.is_some_and(|object| beside(object.cell()));
            near && !band.friendly && !self.is_cleared(&band.object_id)
        });
        bands
            .map(|band| {
                let stacks = self.band(&band.object_id).into_iter();
                let seen = stacks.filter_map(|(troop, count)| Some((troop, size_of(count)?)));
                (band.object_id.as_str(), seen.collect())
            })
            .collect()
    }

    /// The friendly band the hero stands with, while it is still there.
    #[must_use]
    pub fn band_here(&self) -> Option<&Encounter> {
        let object = self.def.map.object_at(self.hero)?;
        let band = self.def.encounter(&object.id)?;
        (band.friendly && !self.is_cleared(&band.object_id)).then_some(band)
    }

    /// The hero answers a friendly band, once. Taken in, its stack joins those of its troop in the army or takes
    /// a free slot, for nothing and whatever the hero's leadership; with every slot held by another troop it
    /// goes its way instead, as it does when it is sent off. Either way it is gone from the map.
    fn meet_band(&mut self, accept: bool, events: &mut Vec<Event>) -> Result<(), Refused> {
        let band = self
            .band_here()
            .ok_or(Refused::NoBandToJoin)?
            .object_id
            .clone();
        let (troop_id, count) = self.band(&band).into_iter().next().unwrap_or_default();
        let slot = self
            .army
            .iter()
            .position(|stack| stack.troop_id == troop_id);
        let room =
            slot.is_some() || self.army.len() < self.def.rules.board.max_stacks_per_side as usize;
        self.cleared.insert(band.clone());
        if !accept || !room || count == 0 {
            events.push(Event::BandLeft { band });
            return Ok(());
        }
        if let Some(i) = slot {
            self.army[i].count = self.army[i].count.saturating_add(count);
        } else {
            let free = (0..=self.army.len())
                .map(|n| format!("p{n}"))
                .find(|id| self.army.iter().all(|s| s.id != *id));
            self.army.push(ArmyStack {
                id: free.unwrap_or_default(),
                troop_id: troop_id.clone(),
                count,
            });
        }
        events.push(Event::BandJoined {
            band,
            troop_id,
            count,
        });
        Ok(())
    }

    /// The alcove teaches a hero who cannot cast, once, for its price; he may pay with his last coin.
    fn learn_magic(&mut self, events: &mut Vec<Event>) -> Result<(), Refused> {
        let cost = self.alcove_here().ok_or(Refused::NoAlcove)?.cost;
        let Some(magic) = &mut self.magic else {
            return Err(Refused::NoAlcove);
        };
        if cost > self.gold {
            return Err(Refused::CannotAfford {
                cost,
                gold: self.gold,
            });
        }
        self.gold -= cost;
        magic.knows = true;
        events.push(Event::MagicLearned { cost });
        Ok(())
    }

    /// The town sells one charge of its spell to a hero with room for one more spell and more gold than the
    /// price: one who has exactly the price is turned away. He need not be able to cast.
    fn buy_spell(&mut self, events: &mut Vec<Event>) -> Result<(), Refused> {
        if self.hero != self.def.town {
            return Err(Refused::NotInTown);
        }
        let (held, limit) = (self.spells_held(), self.spell_limit());
        let (Some(place), Some(magic)) = (self.def.spell_for_sale, &mut self.magic) else {
            return Err(Refused::NoSpellSold);
        };
        if held >= limit {
            return Err(Refused::NoRoomForSpell { limit });
        }
        let spell = &self.def.rules.book[place];
        if spell.cost >= self.gold {
            return Err(Refused::CannotAfford {
                cost: spell.cost,
                gold: self.gold,
            });
        }
        self.gold -= spell.cost;
        magic.charges[place] += 1;
        events.push(Event::SpellBought {
            spell: spell.id.clone(),
            cost: spell.cost,
            held: held + 1,
        });
        Ok(())
    }

    /// Opens the chest the hero stands on, once. Gold is kept, or shared out for a point of leadership to every
    /// fifty gold, rounded down; anything else is the hero's either way it is asked for, but cannot be shared.
    /// A spell's charges are his whatever room he has. It takes no time.
    fn open_chest(&mut self, taken: Taken, events: &mut Vec<Event>) -> Result<(), Refused> {
        let chest = self.chest_here().ok_or(Refused::NoChest)?.clone();
        let object = chest.object_id.clone();
        match (chest.holds, taken) {
            (Holds::Gold(gold), Taken::Kept) => {
                self.gold = self.gold.saturating_add(gold);
                events.push(Event::ChestKept { object, gold });
            }
            (Holds::Gold(gold), Taken::Shared) => {
                let points = u32::try_from(gold / GOLD_A_POINT).unwrap_or(u32::MAX);
                // A hero whose leadership an artefact doubles gets twice the points.
                let twice = self.has_power(Power::DoubleLeadership);
                self.gain_more(Power::DoubleLeadership, points);
                let leadership = if twice {
                    points.saturating_mul(2)
                } else {
                    points
                };
                events.push(Event::ChestShared { object, leadership });
            }
            (Holds::Commission(commission), Taken::Kept) => {
                events.push(Event::ChestCommission { object, commission });
            }
            (Holds::Power(power), Taken::Kept) => {
                events.push(Event::ChestPower { object, power });
            }
            (Holds::Limit(more), Taken::Kept) => {
                // Likewise the room for spells.
                let twice = self.has_power(Power::DoubleMaxSpells);
                self.gain_more(Power::DoubleMaxSpells, more);
                let limit = if twice { more.saturating_mul(2) } else { more };
                events.push(Event::ChestLimit { object, limit });
            }
            (Holds::Spell { spell, count }, Taken::Kept) => {
                if let Some(magic) = &mut self.magic {
                    magic.charges[spell] = magic.charges[spell].saturating_add(count);
                }
                let spell = self.def.rules.book[spell].id.clone();
                events.push(Event::ChestSpell {
                    object,
                    spell,
                    count,
                });
            }
            (_, Taken::Shared) => return Err(Refused::NothingToShare),
        }
        self.opened.insert(chest.object_id, taken);
        Ok(())
    }

    /// The king's audience: every rank the villains caught have earned is given at once, each adding its
    /// leadership and commission to what the hero has. It takes no time.
    fn promote(&mut self, events: &mut Vec<Event>) -> Result<(), Refused> {
        let def = Arc::clone(&self.def);
        let (Some(king), Some(standing)) = (&def.king, &mut self.standing) else {
            return Err(Refused::NotAtCastle);
        };
        if self.hero != king.castle {
            return Err(Refused::NotAtCastle);
        }
        let ranks = &def.rules.classes[king.class].ranks;
        let from = standing.rank;
        while let Some(next) = ranks.get(standing.rank + 1)
            && next.villains_needed <= standing.villains
        {
            standing.rank += 1;
            standing.leadership = standing.leadership.saturating_add(next.leadership);
            standing.commission = standing
                .commission
                .saturating_add(u64::from(next.commission));
        }
        if standing.rank == from {
            return Err(Refused::NoPromotion {
                caught: standing.villains,
                needed: ranks.get(from + 1).map(|next| next.villains_needed),
            });
        }
        let (rank, risen) = (ranks[standing.rank].id.clone(), standing.rank - from);
        // What the hero has with the rank, the chests' share in it.
        events.push(Event::Promoted {
            rank,
            ranks: u32::try_from(risen).unwrap_or(u32::MAX),
            leadership: self.leadership(),
            commission: self.weekly_income(),
        });
        Ok(())
    }

    fn step(&mut self, dx: i32, dy: i32, events: &mut Vec<Event>) -> Result<(), Refused> {
        if dx.abs() + dy.abs() != 1 {
            return Err(Refused::NotAStep);
        }
        let to = self
            .hero
            .x
            .checked_add_signed(dx)
            .zip(self.hero.y.checked_add_signed(dy));
        let Some(to) = to
            .map(|(x, y)| Cell { x, y })
            .filter(|&to| self.passable(to))
        else {
            return Err(Refused::CannotEnter);
        };
        // What bars the way: a band that still stands, or a castle that is not yet the hero's.
        let bars = |o: &&MapObject| match o.kind {
            // A friendly band is stepped onto like any place.
            ObjectKind::Encounter => {
                let friendly = self.def.encounter(&o.id).is_some_and(|band| band.friendly);
                !friendly && !self.is_cleared(&o.id)
            }
            ObjectKind::Castle => self.def.castle(&o.id).is_some() && !self.is_taken(&o.id),
            _ => false,
        };
        let band = self.def.map.object_at(to).filter(bars);
        if band.is_some_and(|o| o.kind == ObjectKind::Castle) && !self.siege_weapons {
            return Err(Refused::NoSiegeWeapons);
        }
        let band = band.map(|object| object.id.clone());
        // The step takes its time whatever stands there, and happens unless the deadline cuts that time short.
        // While time stands a step costs one of the steps it stands for, and no time.
        let passed = match &mut self.magic {
            Some(magic) if magic.free_steps > 0 => {
                magic.free_steps -= 1;
                Passed::All
            }
            _ => self.pass(self.def.clock.movement_ticks, events),
        };
        match band {
            // A band bars the way: the step opens its battle, and the cell is where the hero arrives if he wins.
            // With time up at the same tick there is no battle.
            Some(band) if passed == Passed::All => self.engage(band, to, events),
            None if matches!(passed, Passed::All | Passed::AtDeadline) => {
                let from = std::mem::replace(&mut self.hero, to);
                events.push(Event::Moved { from, to });
                self.claim_artefact(events);
            }
            _ => {}
        }
        self.end_if_due(events);
        Ok(())
    }

    /// Sets up the battle against a band: the army down the player's column of the layout, in slot order, the
    /// band on the enemy's cell.
    fn engage(&mut self, band: String, arrival: Cell, events: &mut Vec<Event>) {
        let def = Arc::clone(&self.def);
        if let Some(castle) = def.castle(&band) {
            return self.besiege(castle, arrival, events);
        }
        let Some(encounter) = def.encounter(&band) else {
            return;
        };
        let layout = &def.layout;
        let height = def.rules.board.height;
        let mut stacks: Vec<StackStart> = self
            .army
            .iter()
            .zip(0..)
            .map(|(stack, slot)| StackStart {
                id: stack.id.clone(),
                side: Side::Player,
                slot,
                troop_id: stack.troop_id.clone(),
                count: stack.count,
                injury: 0,
                x: layout.player.x,
                y: (layout.player.y + slot) % height,
            })
            .collect();
        for ((troop, count), slot) in self.band(&band).into_iter().zip(0..) {
            stacks.push(StackStart {
                id: format!("e{slot}"),
                side: Side::Enemy,
                slot,
                troop_id: troop,
                count,
                injury: 0,
                x: layout.enemy.x,
                y: (layout.enemy.y + slot) % height,
            });
        }
        let scenario = Scenario {
            id: band.clone(),
            rng_state: encounter.battle_rng_state,
            obstacles: layout.obstacles.clone(),
            hero_charges: self.magic_for_battle().0,
            spell_power: self.magic_for_battle().1,
            powers: self.powers_for_battle(),
            // Only the rules that know leadership are told the hero's.
            leadership: matches!(def.rules.profile, Profile::Reference(_))
                .then_some(self.leadership()),
            stacks,
        };
        // The loader has checked the layout and every troop, so the battle starts; if it did not, the hero would
        // simply stand before the band, a step's time spent.
        if let Ok(battle) = Battle::start(Arc::clone(&def.rules), &scenario, None) {
            events.push(Event::BattleStarted { band: band.clone() });
            self.engagement = Some(Engagement {
                battle,
                band,
                arrival,
            });
            self.status = Status::Battle;
        }
    }

    /// Sets up the siege of a castle: the army at the foot of the board and the whole garrison at its head, each
    /// stack where the reference's layout puts it, with the walls between.
    fn besiege(&mut self, castle: &EnemyCastle, arrival: Cell, events: &mut Vec<Event>) {
        let def = Arc::clone(&self.def);
        let band = castle.object_id.clone();
        let army = self.army.iter().zip(SIEGE_BESIEGERS).zip(0..);
        let mut stacks: Vec<StackStart> = army
            .map(|((stack, cell), slot)| StackStart {
                id: stack.id.clone(),
                side: Side::Player,
                slot,
                troop_id: stack.troop_id.clone(),
                count: stack.count,
                injury: 0,
                x: cell.x,
                y: cell.y,
            })
            .collect();
        let garrison = self.garrison(&band).into_iter().zip(SIEGE_GARRISON);
        for (((troop, count), cell), slot) in garrison.zip(0..) {
            stacks.push(StackStart {
                id: format!("e{slot}"),
                side: Side::Enemy,
                slot,
                troop_id: troop,
                count,
                injury: 0,
                x: cell.x,
                y: cell.y,
            });
        }
        let scenario = Scenario {
            id: band.clone(),
            rng_state: castle.battle_rng_state,
            obstacles: SIEGE_WALLS.to_vec(),
            hero_charges: self.magic_for_battle().0,
            spell_power: self.magic_for_battle().1,
            powers: self.powers_for_battle(),
            leadership: matches!(def.rules.profile, Profile::Reference(_))
                .then_some(self.leadership()),
            stacks,
        };
        // As for a band: the loader has checked the castle, so the battle starts.
        if let Ok(battle) = Battle::start(Arc::clone(&def.rules), &scenario, None) {
            events.push(Event::BattleStarted { band: band.clone() });
            self.engagement = Some(Engagement {
                battle,
                band,
                arrival,
            });
            self.status = Status::Battle;
        }
    }

    /// A siege won. A castle of monsters, or of a lord whose contract is held, is the hero's: he stands in it,
    /// and the lord is a villain caught, with his reward and his clue. A lord whose contract is not held goes
    /// free: the castle is his again with its first garrison, and the hero stands before it as he did.
    fn take_castle(&mut self, castle: &EnemyCastle, arrival: Cell, events: &mut Vec<Event>) {
        let id = castle.object_id.clone();
        let held = self.contract.as_deref() == Some(id.as_str());
        let before = self.puzzle_before();
        // A lord already caught is not there to free: his castle, fallen back to monsters, is a castle of monsters.
        let lord = self.lord_at_large(&id).cloned();
        if lord.is_some() && !held {
            if let Some(hold) = self.holds.get_mut(&id) {
                hold.garrison = castle.garrison.iter().map(|stack| stack.count).collect();
            }
            events.push(Event::LordFreed { castle: id });
            return;
        }
        if let Some(hold) = self.holds.get_mut(&id) {
            hold.taken = true;
            hold.caught |= lord.is_some();
            hold.garrison.clear();
        }
        let reward = lord.as_ref().map_or(0, |lord| lord.reward);
        self.gold = self.gold.saturating_add(reward);
        events.push(Event::BattleWon {
            band: id.clone(),
            reward,
        });
        events.push(Event::CastleTaken { castle: id.clone() });
        if let Some(lord) = &lord {
            self.contract = None;
            // His place in the rotation goes to the next villain who has not yet had one, or stays empty.
            if let Some(place) = self
                .cycle
                .iter()
                .position(|v| v.as_deref() == Some(id.as_str()))
            {
                let villains = self.def.rotation.as_ref().map(|r| &r.villains);
                self.cycle[place] = villains.and_then(|v| v.get(self.entered)).cloned();
                self.entered += usize::from(self.cycle[place].is_some());
            }
            if let Some(standing) = &mut self.standing {
                standing.villains += 1;
                events.push(Event::VillainCaught {
                    band: id,
                    caught: standing.villains,
                });
            }
            if let Some(objective) = self.def.objective_id.clone()
                && lord.grants_clue
                && self.clues.insert(objective.clone())
            {
                events.push(Event::ClueGained { object: objective });
            }
            self.tell_piece_lifted(before, events);
        }
        let from = std::mem::replace(&mut self.hero, arrival);
        events.push(Event::Moved { from, to: arrival });
    }

    /// Leaves creatures of the army in the castle of his own the hero stands in: with those of their troop already
    /// there, or in a free place, of which a castle has as many as an army has slots. The army keeps at least one
    /// creature. Nothing is counted or paid: the stack is moved whole or in part, as it is.
    fn garrison_with(
        &mut self,
        troop_id: &str,
        quantity: u32,
        events: &mut Vec<Event>,
    ) -> Result<(), Refused> {
        let castle = self
            .own_castle_here()
            .ok_or(Refused::NotInOwnCastle)?
            .to_owned();
        if quantity == 0 {
            return Err(Refused::NoQuantity);
        }
        let slot = self
            .army
            .iter()
            .position(|stack| stack.troop_id == troop_id);
        let have = slot.map_or(0, |i| self.army[i].count);
        if quantity > have {
            return Err(Refused::NotSoMany { have });
        }
        let creatures: u64 = self.army.iter().map(|stack| u64::from(stack.count)).sum();
        if creatures == u64::from(quantity) {
            return Err(Refused::ArmyEmptied);
        }
        let places = self.def.rules.board.max_stacks_per_side as usize;
        let Some(hold) = self.holds.get_mut(&castle) else {
            return Err(Refused::NotInOwnCastle);
        };
        let there = hold.own.iter().position(|(troop, _)| troop == troop_id);
        match there {
            // Refused before anything is moved: a count that would run over is a count that would be lost.
            Some(i) => {
                let joined = hold.own[i].1.checked_add(quantity);
                hold.own[i].1 = joined.ok_or(Refused::StackTooLarge)?;
            }
            None if hold.own.len() < places => hold.own.push((troop_id.to_owned(), quantity)),
            None => return Err(Refused::CastleFull),
        }
        if let Some(i) = slot {
            self.army[i].count -= quantity;
        }
        self.army.retain(|stack| stack.count > 0);
        events.push(Event::Garrisoned {
            castle,
            troop_id: troop_id.to_owned(),
            count: quantity,
        });
        Ok(())
    }

    /// Takes creatures left in the castle back into the army: to those of their troop, or into a free slot. The
    /// hero's leadership is not asked: a stack too large for it is out of control when it next fights.
    fn withdraw(
        &mut self,
        troop_id: &str,
        quantity: u32,
        events: &mut Vec<Event>,
    ) -> Result<(), Refused> {
        let castle = self
            .own_castle_here()
            .ok_or(Refused::NotInOwnCastle)?
            .to_owned();
        if quantity == 0 {
            return Err(Refused::NoQuantity);
        }
        let slot = self
            .army
            .iter()
            .position(|stack| stack.troop_id == troop_id);
        if slot.is_none() && self.army.len() >= self.def.rules.board.max_stacks_per_side as usize {
            return Err(Refused::ArmyFull);
        }
        let Some(hold) = self.holds.get_mut(&castle) else {
            return Err(Refused::NotInOwnCastle);
        };
        let left = hold.own.iter_mut().find(|(troop, _)| troop == troop_id);
        let have = left.as_ref().map_or(0, |(_, count)| *count);
        let Some((_, count)) = left.filter(|_| quantity <= have) else {
            return Err(Refused::NotSoMany { have });
        };
        let joined = match slot {
            Some(i) => Some(
                self.army[i]
                    .count
                    .checked_add(quantity)
                    .ok_or(Refused::StackTooLarge)?,
            ),
            None => None,
        };
        *count -= quantity;
        hold.own.retain(|(_, count)| *count > 0);
        if let (Some(i), Some(joined)) = (slot, joined) {
            self.army[i].count = joined;
        } else {
            let free = (0..=self.army.len())
                .map(|n| format!("p{n}"))
                .find(|id| self.army.iter().all(|s| s.id != *id));
            self.army.push(ArmyStack {
                id: free.unwrap_or_default(),
                troop_id: troop_id.to_owned(),
                count: quantity,
            });
        }
        events.push(Event::Withdrawn {
            castle,
            troop_id: troop_id.to_owned(),
            count: quantity,
        });
        Ok(())
    }

    /// When a week ends, a castle of the hero's with nobody left in it is held against him again, with the
    /// garrison it first had. Its lord, if it had one, stays caught.
    fn lose_empty_castles(&mut self, events: &mut Vec<Event>) {
        let def = Arc::clone(&self.def);
        for castle in &def.castles {
            let Some(hold) = self.holds.get_mut(&castle.object_id) else {
                continue;
            };
            if hold.taken && hold.own.is_empty() {
                hold.taken = false;
                hold.garrison = castle.garrison.iter().map(|stack| stack.count).collect();
                events.push(Event::CastleLost {
                    castle: castle.object_id.clone(),
                });
            }
        }
    }

    /// Siege weapons are bought in the town, once, where the expedition has castles to use them on.
    fn buy_siege_weapons(&mut self, events: &mut Vec<Event>) -> Result<(), Refused> {
        if self.hero != self.def.town {
            return Err(Refused::NotInTown);
        }
        let Some(cost) = self.def.siege_weapons_cost.filter(|_| !self.siege_weapons) else {
            return Err(Refused::NoSiegeWeaponsToBuy);
        };
        if cost > self.gold {
            return Err(Refused::CannotAfford {
                cost,
                gold: self.gold,
            });
        }
        self.gold -= cost;
        self.siege_weapons = true;
        events.push(Event::SiegeWeaponsBought { cost });
        Ok(())
    }

    /// Passes a command to the battle and, when that ends it, writes the result back, once.
    fn fight(&mut self, order: &battle::Command, events: &mut Vec<Event>) -> Result<(), Refused> {
        let Some(engagement) = &mut self.engagement else {
            return Err(Refused::NotInBattle);
        };
        let happened = engagement.battle.apply(order).map_err(Refused::Battle)?;
        // Every creature of the hero's side that a blow kills counts against the score, whoever struck it.
        if self.def.scored {
            let own = |id: &str| {
                let stack = engagement.battle.stack(id);
                stack.is_some_and(|stack| stack.side == Side::Player)
            };
            let lost = happened.iter().filter_map(|event| match event {
                battle::Event::Damage { target, losses, .. } if own(target) => {
                    Some(u64::from(*losses))
                }
                _ => None,
            });
            self.fallen = self.fallen.saturating_add(lost.sum());
        }
        // A charge spent in the battle is spent for good, however the battle ends.
        if let Some(magic) = &mut self.magic {
            for (spell, left) in engagement.battle.hero_charges() {
                let place = self.def.rules.book.iter().position(|s| s.id == *spell);
                if let Some(place) = place {
                    magic.charges[place] = *left;
                }
            }
        }
        events.extend(happened.into_iter().map(Event::Battle));
        match engagement.battle.outcome() {
            None => {}
            Some(Outcome::PlayerWin) => self.win(events),
            Some(Outcome::EnemyWin) => self.recover(events),
            Some(Outcome::BudgetExceeded) => {
                // Nobody won. The army is what the battle left of it; the band stands as before.
                self.keep_survivors(0, events);
                if let Some(engagement) = self.engagement.take() {
                    events.push(Event::BattleStalled {
                        band: engagement.band,
                    });
                }
                self.status = Status::Adventure;
            }
        }
        Ok(())
    }

    /// The army as the battle left it: each stack with its survivors and, where the rules have it so, `returning`
    /// in a hundred of its fallen; the stacks left empty gone. Injury is healed.
    fn keep_survivors(&mut self, returning: u32, events: &mut Vec<Event>) {
        let Some(engagement) = &self.engagement else {
            return;
        };
        for stack in &mut self.army {
            let fought = engagement.battle.stack(&stack.id);
            // More than went in, for a stack that grew by what it killed: it keeps them.
            let survivors = fought.map_or(0, |fought| fought.count);
            let fallen = u64::from(stack.count.saturating_sub(survivors));
            // At most the fallen, so it fits.
            let back = u32::try_from(fallen * u64::from(returning) / 100).unwrap_or(0);
            stack.count = survivors + back;
            if back > 0 {
                events.push(Event::FallenReturned {
                    troop_id: stack.troop_id.clone(),
                    count: back,
                });
            }
        }
        self.army.retain(|stack| stack.count > 0);
    }

    /// The band is beaten: survivors kept, the hero on the band's cell, the reward paid, and the clue given if
    /// the hero held the band's contract. Battles take no time of the campaign.
    fn win(&mut self, events: &mut Vec<Event>) {
        self.keep_survivors(self.switches().fallen_return_percent, events);
        let Some(Engagement { band, arrival, .. }) = self.engagement.take() else {
            return;
        };
        self.status = Status::Adventure;
        let def = Arc::clone(&self.def);
        if let Some(castle) = def.castle(&band) {
            return self.take_castle(castle, arrival, events);
        }
        let Some(encounter) = def.encounter(&band) else {
            return;
        };
        self.gold = self.gold.saturating_add(encounter.gold_reward);
        self.cleared.insert(band.clone());
        events.push(Event::BattleWon {
            band: band.clone(),
            reward: encounter.gold_reward,
        });
        if self.contract.as_deref() == Some(band.as_str()) {
            self.contract = None;
            if let Some(standing) = &mut self.standing {
                standing.villains += 1;
                events.push(Event::VillainCaught {
                    band: band.clone(),
                    caught: standing.villains,
                });
            }
            if let Some(objective) = def.objective_id.clone()
                && encounter.grants_clue
                && self.clues.insert(objective.clone())
            {
                events.push(Event::ClueGained { object: objective });
            }
        }
        let from = std::mem::replace(&mut self.hero, arrival);
        events.push(Event::Moved { from, to: arrival });
    }

    /// A battle lost or given up: the hero is taken to the place of recovery with half his gold and the troops
    /// recovery gives; the band stands as before; contract, clues and beaten bands are kept. Then recovery takes
    /// its time, with the weeks and the deadline as for any other time.
    fn recover(&mut self, events: &mut Vec<Event>) {
        self.engagement = None;
        self.status = Status::Adventure;
        let recovery = self.def.recovery.clone();
        self.hero = recovery.destination;
        self.gold /= 2;
        self.army = vec![ArmyStack {
            id: "p0".to_owned(),
            troop_id: recovery.troop_id,
            count: recovery.count,
        }];
        events.push(Event::Recovered { gold: self.gold });
        self.pass(recovery.ticks, events);
        self.end_if_due(events);
    }

    fn recruit(
        &mut self,
        troop_id: &str,
        quantity: u32,
        events: &mut Vec<Event>,
    ) -> Result<(), Refused> {
        let (price, left) = self.offer(troop_id)?;
        let Some(troop) = self.def.rules.troop(troop_id) else {
            return Err(Refused::NotSold {
                troop_id: troop_id.to_owned(),
            });
        };
        if quantity == 0 {
            return Err(Refused::NoQuantity);
        }
        if let Some(stock) = left.filter(|&stock| quantity > stock) {
            return Err(Refused::OutOfStock { stock });
        }
        // Exactly what the purse holds is enough.
        let cost = price.saturating_mul(u64::from(quantity));
        if cost > self.gold {
            return Err(Refused::CannotAfford {
                cost,
                gold: self.gold,
            });
        }
        let slot = self
            .army
            .iter()
            .position(|stack| stack.troop_id == troop_id);
        let already = slot.map_or(0, |i| self.army[i].count);
        // The whole stack, old and new, must be within the leadership; exactly the leadership is within it.
        let health = (u64::from(already) + u64::from(quantity)) * u64::from(troop.hp);
        if health > u64::from(self.leadership()) {
            return Err(Refused::OverLeadership {
                health,
                leadership: self.leadership(),
            });
        }
        if slot.is_none() && self.army.len() >= self.def.rules.board.max_stacks_per_side as usize {
            return Err(Refused::ArmyFull);
        }

        self.gold -= cost;
        // The town's stock, or the dwelling's population; the castle's troops are not counted.
        let dwelling = self.def.dwelling_at(self.hero).map(|d| d.object_id.clone());
        match (left, dwelling) {
            (Some(stock), None) => {
                self.stock.insert(troop_id.to_owned(), stock - quantity);
            }
            (Some(stock), Some(object)) => {
                self.populations.insert(object, stock - quantity);
            }
            (None, _) => {}
        }
        if let Some(i) = slot {
            self.army[i].count += quantity;
        } else {
            // The lowest "p<n>" no stack has: among as many names as there are stacks plus one, one is free.
            let free = (0..=self.army.len())
                .map(|n| format!("p{n}"))
                .find(|id| self.army.iter().all(|s| s.id != *id));
            self.army.push(ArmyStack {
                id: free.unwrap_or_default(),
                troop_id: troop_id.to_owned(),
                count: quantity,
            });
        }
        events.push(Event::Recruited {
            troop_id: troop_id.to_owned(),
            quantity,
            cost,
        });
        Ok(())
    }

    fn take_contract(&mut self, id: &str, events: &mut Vec<Event>) -> Result<(), Refused> {
        if self.hero != self.def.town {
            return Err(Refused::NotInTown);
        }
        if self.def.rotation.is_some() {
            // The town gives its contracts in turn: the one asked for must be the one whose turn it is.
            let place = self
                .next_place()
                .filter(|&p| self.cycle[p].as_deref() == Some(id));
            let Some(place) = place else {
                return Err(Refused::NoSuchContract { id: id.to_owned() });
            };
            self.last_given = place;
        } else {
            // On a band that still stands, or on the lord of a castle while he is at large.
            let hostile = self.def.encounter(id).is_some_and(|band| !band.friendly);
            let band = hostile && !self.is_cleared(id);
            if !band && self.lord_at_large(id).is_none() {
                return Err(Refused::NoSuchContract { id: id.to_owned() });
            }
        }
        self.contract = Some(id.to_owned());
        events.push(Event::ContractTaken { id: id.to_owned() });
        Ok(())
    }

    /// A search takes its time whatever it finds. It finds the objective only if all of that time passes; a
    /// search that ends exactly on the deadline still counts.
    fn search(&mut self, events: &mut Vec<Event>) {
        let passed = self.pass(self.def.clock.search_ticks, events);
        if matches!(passed, Passed::All | Passed::AtDeadline) {
            let found = self.hero == self.def.objective();
            events.push(Event::Searched { found });
            if found {
                self.end(Status::Victory, events);
            }
        }
        self.end_if_due(events);
    }

    /// Lets `ticks` pass, one by one: each week that ends is settled once, income first and then upkeep, and the
    /// clock stops at the deadline. Settlement comes before anything else that falls on the same tick.
    fn pass(&mut self, ticks: u32, events: &mut Vec<Event>) -> Passed {
        let clock = &self.def.clock;
        let (week_ticks, deadline) = (clock.week_ticks, clock.deadline_ticks);
        let income = self.weekly_income();
        let from = self.ticks;
        let mut passed = Passed::All;
        // Time that passes ends whatever was left of time standing.
        if let Some(magic) = self.magic.as_mut().filter(|_| ticks > 0) {
            magic.free_steps = 0;
        }
        for done in 1..=ticks {
            self.ticks += 1;
            if self.ticks.is_multiple_of(week_ticks) {
                // Leadership a spell raised lasts the week.
                if let Some(magic) = &mut self.magic {
                    magic.raised = 0;
                }
                self.gold = self.gold.saturating_add(income);
                let Some(upkeep) = self.pay_upkeep(events) else {
                    passed = Passed::Insolvent;
                    break;
                };
                events.push(Event::WeekSettled {
                    week: self.ticks / week_ticks,
                    income,
                    upkeep,
                    gold: self.gold,
                });
                self.lose_empty_castles(events);
                self.begin_week(self.ticks / week_ticks, events);
            }
            if self.ticks >= deadline {
                passed = if done == ticks {
                    Passed::AtDeadline
                } else {
                    Passed::CutShort
                };
                break;
            }
        }
        events.push(Event::TimePassed {
            from,
            to: self.ticks,
        });
        if passed == Passed::Insolvent {
            self.end(Status::Insolvent, events);
        }
        passed
    }

    /// A new week begins under its creature, where the weeks have one: the dwellings of that troop are full
    /// again, and no other. In the week of the first troop of the rules, a stack of the army that grows by what
    /// it kills turns into that troop, creature for creature, as the reference has it of Ghosts and Peasants.
    fn begin_week(&mut self, week: u32, events: &mut Vec<Event>) {
        let def = Arc::clone(&self.def);
        let Some(creature) = self.creature_of(week).cloned() else {
            return;
        };
        events.push(Event::WeekCreature {
            week,
            troop_id: creature.id.clone(),
        });
        for dwelling in def.dwellings.iter().filter(|d| d.troop == creature.id) {
            let full = creature.max_population;
            if self.population(&dwelling.object_id) != full {
                self.populations.insert(dwelling.object_id.clone(), full);
                events.push(Event::DwellingRefilled {
                    object: dwelling.object_id.clone(),
                    population: full,
                });
            }
        }
        // And in every castle that is not the hero's.
        for castle in &def.castles {
            let Some(hold) = self
                .holds
                .get_mut(&castle.object_id)
                .filter(|hold| !hold.taken)
            else {
                continue;
            };
            for (stack, count) in castle.garrison.iter().zip(hold.garrison.iter_mut()) {
                if stack.troop == creature.id && creature.growth > 0 {
                    *count = count.saturating_add(creature.growth);
                    events.push(Event::BandGrew {
                        band: castle.object_id.clone(),
                        troop_id: creature.id.clone(),
                        count: *count,
                    });
                }
            }
        }
        // The creature's own kind grows in every band that still stands.
        for band in &def.encounters {
            if self.cleared.contains(&band.object_id) {
                continue;
            }
            let Some(counts) = self.band_counts.get_mut(&band.object_id) else {
                continue;
            };
            for ((troop, _), count) in band.stacks().into_iter().zip(counts.iter_mut()) {
                if troop == creature.id && creature.growth > 0 {
                    *count = count.saturating_add(creature.growth);
                    events.push(Event::BandGrew {
                        band: band.object_id.clone(),
                        troop_id: creature.id.clone(),
                        count: *count,
                    });
                }
            }
        }
        if def
            .rules
            .troops
            .first()
            .is_some_and(|first| first.id == creature.id)
        {
            for stack in &mut self.army {
                let absorbs = def
                    .rules
                    .troop(&stack.troop_id)
                    .is_some_and(|troop| troop.has(Ability::Absorb));
                if absorbs {
                    let from = std::mem::replace(&mut stack.troop_id, creature.id.clone());
                    events.push(Event::Turned {
                        from,
                        to: creature.id.clone(),
                        count: stack.count,
                    });
                }
            }
        }
    }

    /// Ends the expedition as timed out when the deadline has come and nothing else has ended it.
    fn end_if_due(&mut self, events: &mut Vec<Event>) {
        if self.status == Status::Adventure && self.ticks >= self.def.clock.deadline_ticks {
            self.end(Status::Timeout, events);
        }
    }

    fn end(&mut self, status: Status, events: &mut Vec<Event>) {
        self.status = status;
        events.push(Event::Ended { status });
    }

    /// The parts of the state only some expeditions have, each written only where the expedition has it, so that
    /// every other is encoded, and digested, as before.
    fn write_what_some_have(&self, members: &mut BTreeMap<String, Value>) {
        // Written only where there is a king, so every other expedition is encoded as before.
        if let Some(standing) = &self.standing {
            members.insert(
                "standing".to_owned(),
                Value::object([
                    ("commission", standing.commission.into()),
                    ("leadership", standing.leadership.into()),
                    ("rank", (standing.rank as u64).into()),
                    ("villains", standing.villains.into()),
                ]),
            );
        }
        // Written only where there are dwellings.
        if !self.populations.is_empty() {
            let left = self.populations.iter();
            let left = left.map(|(object, left)| (object.clone(), Value::from(*left)));
            members.insert("dwellings".to_owned(), Value::Object(left.collect()));
        }
        // Written only where bands grow.
        if !self.band_counts.is_empty() {
            let bands = self.band_counts.iter().map(|(band, counts)| {
                let counts = counts.iter().map(|count| Value::from(*count));
                (band.clone(), Value::Array(counts.collect()))
            });
            members.insert("bands".to_owned(), Value::Object(bands.collect()));
        }
        // Written only where the town gives contracts in turn.
        if self.def.rotation.is_some() {
            let places = self.cycle.iter();
            let places = places.map(|villain| villain.as_deref().map_or(Value::Null, Value::str));
            let rotation = Value::object([
                ("cycle", Value::Array(places.collect())),
                ("entered", (self.entered as u64).into()),
                ("last_given", (self.last_given as u64).into()),
            ]);
            members.insert("rotation".to_owned(), rotation);
        }
        // Written only where there are castles to besiege: how each stands, and whether the hero can besiege.
        if !self.def.castles.is_empty() {
            let holds = self.holds.iter().map(|(castle, hold)| {
                let garrison = hold.garrison.iter().map(|count| Value::from(*count));
                let own = hold.own.iter().map(|(troop, count)| {
                    Value::object([("count", (*count).into()), ("troop_id", Value::str(troop))])
                });
                let hold = Value::object([
                    ("caught", hold.caught.into()),
                    ("garrison", Value::Array(garrison.collect())),
                    ("own", Value::Array(own.collect())),
                    ("taken", hold.taken.into()),
                ]);
                (castle.clone(), hold)
            });
            members.insert("castles".to_owned(), Value::Object(holds.collect()));
            members.insert("siege_weapons".to_owned(), self.siege_weapons.into());
        }
        // Written only where artefacts lie: those found, each with what it has added.
        if !self.def.artefacts.is_empty() {
            let found = self.found.iter();
            let found = found.map(|(id, gain)| (id.clone(), Value::from(*gain)));
            members.insert("artefacts".to_owned(), Value::Object(found.collect()));
        }
        // Written only where the expedition is scored.
        if self.def.scored {
            members.insert("fallen".to_owned(), self.fallen.into());
        }
        // Written only where there is magic: whether the hero casts, and the spells he holds charges of.
        if let Some(magic) = &self.magic {
            let book = self.def.rules.book.iter().zip(&magic.charges);
            let book = book.filter(|(_, count)| **count > 0);
            let book = book.map(|(spell, count)| (spell.id.clone(), Value::from(*count)));
            let bridges = magic.bridges.iter();
            let bridges = bridges.map(|c| Value::object([("x", c.x.into()), ("y", c.y.into())]));
            let magic = Value::object([
                ("book", Value::Object(book.collect())),
                ("bridges", Value::Array(bridges.collect())),
                ("free_steps", magic.free_steps.into()),
                ("knows", magic.knows.into()),
                ("raised", magic.raised.into()),
            ]);
            members.insert("magic".to_owned(), magic);
        }
        // Written only where there are chests: the open ones, each with what was done with it.
        if !self.def.chests.is_empty() {
            let open = self.opened.iter().map(|(object, taken)| {
                let taken = match taken {
                    Taken::Kept => "kept",
                    Taken::Shared => "shared",
                };
                (object.clone(), Value::str(taken))
            });
            members.insert("chests".to_owned(), Value::Object(open.collect()));
        }
    }

    /// The whole state that decides what happens next, as a value with one fixed encoding.
    #[must_use]
    pub fn to_canon(&self) -> Value {
        let ids =
            |set: &BTreeSet<String>| Value::Array(set.iter().map(|id| Value::str(id)).collect());
        let army = self.army.iter().map(|stack| {
            Value::object([
                ("count", stack.count.into()),
                ("id", Value::str(&stack.id)),
                ("troop_id", Value::str(&stack.troop_id)),
            ])
        });
        let stock = self
            .stock
            .iter()
            .map(|(troop, left)| (troop.clone(), Value::from(*left)))
            .collect();
        let mut value = Value::object([
            ("army", Value::Array(army.collect())),
            ("cleared", ids(&self.cleared)),
            ("clues", ids(&self.clues)),
            (
                "contract",
                self.contract.as_deref().map_or(Value::Null, Value::str),
            ),
            ("expedition_id", Value::str(&self.def.id)),
            ("gold", self.gold.into()),
            (
                "hero",
                Value::object([("x", self.hero.x.into()), ("y", self.hero.y.into())]),
            ),
            ("revision", self.revision.into()),
            ("status", Value::str(self.status.name())),
            ("stock", Value::Object(stock)),
            ("ticks", self.ticks.into()),
        ]);
        if let Value::Object(members) = &mut value {
            self.write_what_some_have(members);
        }
        // The battle being fought is part of the state, with where it leads.
        if let (Some(engagement), Value::Object(members)) = (&self.engagement, &mut value) {
            members.insert("battle".to_owned(), engagement.battle.to_canon());
            members.insert("band".to_owned(), Value::str(&engagement.band));
            let arrival = engagement.arrival;
            members.insert(
                "arrival".to_owned(),
                Value::object([("x", arrival.x.into()), ("y", arrival.y.into())]),
            );
        }
        value
    }

    /// The expedition of `def` that a value written by [`Expedition::to_canon`] describes, with its battle given
    /// apart, as a save keeps it. A state the rules could not have reached is refused.
    ///
    /// # Errors
    /// [`SaveError::Invalid`] naming the first thing that cannot be.
    pub fn from_canon(
        def: Arc<ExpeditionDef>,
        value: &Value,
        battle: Option<&Value>,
    ) -> Result<Self, SaveError> {
        let state = Reader::of(value, "the campaign")?;
        if state.text("expedition_id")? != def.id {
            return invalid(format!(
                "it is the expedition {:?}",
                state.text("expedition_id")?
            ));
        }
        let cell = |key| -> Result<Cell, SaveError> {
            let from = state.object(key)?;
            Ok(Cell {
                x: from.u32("x")?,
                y: from.u32("y")?,
            })
        };
        let Some(status) = Status::from_name(state.text("status")?) else {
            return invalid(format!(
                "the expedition stands as {:?}",
                state.text("status")?
            ));
        };
        let mut army = Vec::new();
        for item in state.list("army")? {
            let from = Reader::of(item, "a stack of the army")?;
            army.push(ArmyStack {
                id: from.text("id")?.to_owned(),
                troop_id: from.text("troop_id")?.to_owned(),
                count: from.u32("count")?,
            });
        }
        let engagement = match battle {
            None => None,
            Some(battle) => Some(Engagement {
                battle: Battle::from_canon(Arc::clone(&def.rules), battle)?,
                band: state.text("band")?.to_owned(),
                arrival: cell("arrival")?,
            }),
        };
        // Only an expedition with a king has a standing, and it always has.
        let standing = if def.king.is_some() {
            let from = state.object("standing")?;
            Some(Standing {
                rank: usize::try_from(from.u64("rank")?).unwrap_or(usize::MAX),
                leadership: from.u32("leadership")?,
                commission: from.u64("commission")?,
                villains: from.u32("villains")?,
            })
        } else {
            None
        };
        let populations = if def.dwellings.is_empty() {
            BTreeMap::new()
        } else {
            state.numbers("dwellings")?
        };
        let opened = saved_chests(&def, &state)?;
        let magic = saved_magic(&def, &state)?;
        let fallen = if def.scored { state.u64("fallen")? } else { 0 };
        let found = if def.artefacts.is_empty() {
            BTreeMap::new()
        } else {
            state.numbers("artefacts")?
        };
        let band_counts = if def.clock.creature_seed.is_some() {
            state.number_lists("bands")?
        } else {
            BTreeMap::new()
        };
        let (holds, siege_weapons) = saved_holds(&def, &state)?;
        let (cycle, last_given, entered) = saved_rotation(&def, &state)?;
        let expedition = Self {
            standing,
            magic,
            opened,
            holds,
            siege_weapons,
            fallen,
            found,
            cycle,
            last_given,
            entered,
            band_counts,
            populations,
            revision: state.u64("revision")?,
            hero: cell("hero")?,
            ticks: state.u32("ticks")?,
            gold: state.u64("gold")?,
            army,
            stock: state.numbers("stock")?,
            contract: match state.opt("contract") {
                None => None,
                Some(Value::Str(band)) => Some(band.clone()),
                Some(_) => return invalid("the contract is not text"),
            },
            clues: state.texts("clues")?.into_iter().collect(),
            cleared: state.texts("cleared")?.into_iter().collect(),
            engagement,
            status,
            def,
        };
        expedition.check()?;
        Ok(expedition)
    }

    /// What must hold of any expedition the rules produce. A saved one that fails this is refused.
    fn check(&self) -> Result<(), SaveError> {
        let def = &self.def;
        if !self.passable(self.hero) {
            return invalid("the hero stands where nobody can");
        }
        if self.ticks > def.clock.deadline_ticks {
            return invalid("the clock is past the deadline");
        }
        for (i, stack) in self.army.iter().enumerate() {
            let known = def
                .rules
                .troops
                .iter()
                .any(|troop| troop.id == stack.troop_id);
            if !known || stack.count == 0 || self.army[..i].iter().any(|other| other.id == stack.id)
            {
                return invalid(format!(
                    "the army's stack {:?} is empty, a twin, or of no troop",
                    stack.id
                ));
            }
        }
        if self.army.len() > def.rules.board.max_stacks_per_side as usize {
            return invalid("the army has more stacks than a side may field");
        }
        let stocked =
            |(troop, left): (&String, &u32)| def.stock.get(troop).is_some_and(|all| left <= all);
        if self.stock.len() != def.stock.len() || !self.stock.iter().all(stocked) {
            return invalid("the town's stock is not of this expedition");
        }
        let band = |id: &String| def.encounter(id).is_some();
        // A contract is on a band, or on the lord of a castle.
        let lord = |id: &String| def.castle(id).is_some_and(|castle| castle.lord.is_some());
        let contracted = |id: &String| band(id) || lord(id);
        if !self.cleared.iter().all(band) || !self.contract.iter().all(contracted) {
            return invalid("a beaten band or the contract names no band");
        }
        self.check_standing()?;
        self.check_magic()?;
        self.check_artefacts()?;
        self.check_places()?;
        if let Some(unknown) = self.clues.iter().find(|id| def.map.object(id).is_none()) {
            return invalid(format!("a clue names {unknown:?}, which is not on the map"));
        }
        match (&self.engagement, self.status) {
            (Some(engagement), Status::Battle) => {
                let besieged = def.castle(&engagement.band).is_some()
                    && !self.is_taken(&engagement.band)
                    && self.siege_weapons;
                let fought = band(&engagement.band) && !self.cleared.contains(&engagement.band);
                if !fought && !besieged {
                    return invalid("the battle is against no band, or one already beaten");
                }
                if !self.passable(engagement.arrival) {
                    return invalid("the battle leads where nobody can stand");
                }
                if engagement.battle.outcome().is_some() {
                    return invalid("the battle is over and its result not written back");
                }
                Ok(())
            }
            (None, Status::Battle) | (Some(_), _) => {
                invalid("an expedition is in battle exactly when it has one")
            }
            (None, _) => Ok(()),
        }
    }

    /// What must hold of the places an expedition has: castles, bands, chests and dwellings.
    fn check_places(&self) -> Result<(), SaveError> {
        let def = &self.def;
        self.check_rotation()?;
        // A castle the hero has taken has no enemy garrison; one he has not has the stacks its file gives it, grown
        // or as they were. Nobody takes a castle without siege weapons.
        let stands = |castle: &EnemyCastle| {
            let first = castle.garrison.iter().map(|stack| stack.count);
            self.holds.get(&castle.object_id).is_some_and(|hold| {
                // Only a lord can be caught, and what the hero leaves in a castle are stacks of known troops, one
                // to a troop, none of them empty and no more of them than a side may field.
                let known =
                    |(troop, count): &(String, u32)| *count > 0 && def.rules.troop(troop).is_some();
                let troops: BTreeSet<&String> = hold.own.iter().map(|(troop, _)| troop).collect();
                let left = hold.own.iter().all(known)
                    && troops.len() == hold.own.len()
                    && hold.own.len() <= def.rules.board.max_stacks_per_side as usize;
                if hold.caught && castle.lord.is_none() {
                    return false;
                }
                if hold.taken {
                    hold.garrison.is_empty() && self.siege_weapons && left
                } else {
                    hold.own.is_empty()
                        && hold.garrison.len() == castle.garrison.len()
                        && first.zip(&hold.garrison).all(|(was, now)| was <= *now)
                }
            })
        };
        if self.holds.len() != def.castles.len() || !def.castles.iter().all(stands) {
            return invalid("the castles are not this expedition's, or stand as they cannot");
        }
        // A band has grown or stayed, never shrunk, and has the stacks its file gives it.
        let at_start = Self::bands_at_start(def);
        let grown = |(band, counts): (&String, &Vec<u32>)| {
            at_start.get(band).is_some_and(|first| {
                first.len() == counts.len() && first.iter().zip(counts).all(|(was, now)| was <= now)
            })
        };
        if self.band_counts.len() != at_start.len() || !self.band_counts.iter().all(grown) {
            return invalid("the bands are not this expedition's, or have shrunk");
        }
        let lawful = |(object, taken): (&String, &Taken)| {
            let chest = def.chests.iter().find(|c| c.object_id == *object);
            chest
                .is_some_and(|c| matches!((c.holds, taken), (Holds::Gold(_), _) | (_, Taken::Kept)))
        };
        if !self.opened.iter().all(lawful) {
            return invalid(
                "an open chest is not this expedition's, or was shared and held no gold",
            );
        }
        let holds = |dwelling: &Dwelling| {
            let full = def.rules.troop(&dwelling.troop).map(|t| t.max_population);
            let left = self.populations.get(&dwelling.object_id);
            left.zip(full).is_some_and(|(left, full)| *left <= full)
        };
        if self.populations.len() != def.dwellings.len() || !def.dwellings.iter().all(holds) {
            return invalid("the dwellings are not this expedition's, or hold more than they can");
        }
        Ok(())
    }

    /// The rotation has the places it started with; the villains in them are lords at large, each once, from
    /// among those who have come into it; and a contract held is on one of them.
    fn check_rotation(&self) -> Result<(), SaveError> {
        let Some(rotation) = &self.def.rotation else {
            return if self.cycle.is_empty() {
                Ok(())
            } else {
                invalid("there is a rotation of contracts where the town gives none in turn")
            };
        };
        let entered = rotation.villains.get(..self.entered);
        let in_turn: Vec<&String> = self.cycle.iter().flatten().collect();
        let once: BTreeSet<&String> = in_turn.iter().copied().collect();
        let lawful = entered.is_some_and(|entered| {
            in_turn
                .iter()
                .all(|id| entered.contains(id) && self.lord_at_large(id).is_some())
        });
        let held = self.contract.iter().all(|id| in_turn.contains(&id));
        if self.cycle.len() != Self::cycle_at_start(&self.def).len()
            || self.last_given >= self.cycle.len().max(1)
            || once.len() != in_turn.len()
            || !lawful
            || !held
        {
            return invalid("the rotation of contracts stands as it cannot");
        }
        Ok(())
    }

    /// A hero casts only where his class does or an alcove could have taught him, and holds no more spells than
    /// his room and the chests he opened allow, each the town's or a chest's.
    fn check_magic(&self) -> Result<(), SaveError> {
        let def = &self.def;
        let Some(magic) = &self.magic else {
            return Ok(());
        };
        if magic.knows && !def.casts_from_start() && def.alcove.is_none() {
            return invalid("the hero casts, and nothing could have taught him");
        }
        let mut found = vec![0_u32; def.rules.book.len()];
        for holds in self.opened_chests() {
            if let Holds::Spell { spell, count } = holds {
                found[spell] = found[spell].saturating_add(count);
            }
        }
        let lawful = magic
            .charges
            .iter()
            .zip(&found)
            .enumerate()
            .all(|(place, (held, found))| held <= found || def.spell_for_sale == Some(place));
        let most = found
            .iter()
            .fold(self.spell_limit(), |most, n| most.saturating_add(*n));
        if !lawful || self.spells_held() > most {
            return invalid("the hero holds spells he could not have come by");
        }
        // What a spell of the map leaves behind is a caster's, and a bridge lies over water, once.
        let water = |cell: &Cell| {
            def.map
                .terrain(*cell)
                .is_some_and(|terrain| terrain.id == "water")
        };
        let once = |(i, cell): (usize, &Cell)| !magic.bridges[..i].contains(cell);
        let cast = magic.free_steps > 0 || magic.raised > 0 || !magic.bridges.is_empty();
        if (cast && !magic.knows)
            || !magic.bridges.iter().all(water)
            || !magic.bridges.iter().enumerate().all(once)
        {
            return invalid(
                "time stands, leadership is raised or water is bridged as no spell could have",
            );
        }
        // In a battle with magic the hero's charges are the battle's, and its spell power his.
        if let Some(battle) = self
            .engagement
            .as_ref()
            .map(|engagement| &engagement.battle)
            && let Some(power) = battle.spell_power()
        {
            let same = battle
                .hero_charges()
                .iter()
                .all(|(spell, left)| self.charges(spell) == *left);
            if !same || !magic.knows || power != self.spell_power().max(1) {
                return invalid("the battle's magic is not the hero's");
            }
        }
        Ok(())
    }

    /// A standing is what its rank makes it: the leadership and commission of the ranks reached, and at least
    /// the villains the rank asks for, none of them more than the bands beaten.
    fn check_standing(&self) -> Result<(), SaveError> {
        let (Some(king), Some(standing)) = (&self.def.king, &self.standing) else {
            return if self.def.king.is_none() && self.standing.is_none() {
                Ok(())
            } else {
                invalid("a hero stands with a king exactly when the expedition has one")
            };
        };
        let ranks = &self.def.rules.classes[king.class].ranks;
        let Some(reached) = ranks.get(..=standing.rank) else {
            return invalid("the hero holds a rank his class does not have");
        };
        let leadership: u64 = reached.iter().map(|r| u64::from(r.leadership)).sum();
        let commission: u64 = reached.iter().map(|r| u64::from(r.commission)).sum();
        let earned = reached
            .last()
            .is_some_and(|r| r.villains_needed <= standing.villains);
        if u64::from(standing.leadership) != leadership
            || standing.commission != commission
            || !earned
            || standing.villains as usize > self.cleared.len() + self.lords_caught()
        {
            return invalid("the hero's standing is not what his rank and his villains make it");
        }
        Ok(())
    }

    /// An artefact found is one that lies on this map, and has added no more than the hero has besides: it
    /// doubled what he had, and only the three that double add anything.
    fn check_artefacts(&self) -> Result<(), SaveError> {
        let def = &self.def;
        for (id, gain) in &self.found {
            let mut laid = def
                .artefacts
                .iter()
                .map(|laid| &def.rules.artefacts[laid.artefact]);
            let Some(artefact) = laid.find(|artefact| artefact.id == *id) else {
                return invalid(format!(
                    "the artefact {id:?} is found and does not lie on this map"
                ));
            };
            let besides = match artefact.power {
                Power::DoubleLeadership => self.leadership_for_good(),
                Power::DoubleSpellPower => self.spell_power().saturating_sub(*gain),
                Power::DoubleMaxSpells => self.spell_limit().saturating_sub(*gain),
                _ => 0,
            };
            if *gain > besides {
                return invalid(format!(
                    "the artefact {id:?} has added more than it could have"
                ));
            }
        }
        if let Some(engagement) = &self.engagement
            && engagement.battle.powers() != self.powers_for_battle()
        {
            return invalid("the battle's artefacts are not the hero's");
        }
        Ok(())
    }

    /// The score, where the expedition is scored: so much for each villain caught, each artefact found and each
    /// castle the hero holds now, less so much for each of his creatures that fell, and never less than nothing.
    #[must_use]
    pub fn score(&self) -> Option<u64> {
        let scoring = self.def.rules.scoring.filter(|_| self.def.scored)?;
        let villains = self
            .standing
            .as_ref()
            .map_or(0, |standing| u64::from(standing.villains));
        let castles = self.holds.values().filter(|hold| hold.taken).count() as u64;
        let artefacts = self.found.len() as u64;
        let earned = villains
            .saturating_mul(scoring.per_villain)
            .saturating_add(artefacts.saturating_mul(scoring.per_artefact))
            .saturating_add(castles.saturating_mul(scoring.per_castle));
        Some(earned.saturating_sub(self.fallen.saturating_mul(scoring.per_fallen)))
    }

    /// The hero's creatures that fell in battle; counted only where the expedition is scored.
    #[must_use]
    pub const fn fallen(&self) -> u64 {
        self.fallen
    }

    /// The lords whose castles the hero has taken: each of them a villain caught.
    fn lords_caught(&self) -> usize {
        let lords = self
            .def
            .castles
            .iter()
            .filter(|castle| castle.lord.is_some());
        let caught = |castle: &&EnemyCastle| {
            let hold = self.holds.get(&castle.object_id);
            hold.is_some_and(|hold| hold.caught)
        };
        lords.filter(caught).count()
    }

    /// The digest of [`Expedition::to_canon`].
    #[must_use]
    pub fn digest(&self) -> String {
        // Every key above is ASCII and every number unsigned, so the encoding cannot fail.
        self.to_canon().digest().unwrap_or_default()
    }
}

mod artefacts;
mod puzzle;
mod spells;
#[cfg(test)]
mod tests;
