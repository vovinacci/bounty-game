//! The expedition's screen outside battle: the map, the hero's purse, clock and army, and the keys that do
//! something where he stands. What a key does is asked of the rules (`Expedition::refusal`); nothing is decided
//! here.

use bounty_core::campaign::{Action, Holds, ObjectKind};
use bounty_core::content::{Cell, MODERN_RULESET, REFERENCE_RULESET};
use gamekit::t;
use gamekit::text::{text_at, text_centered_fit, text_fit};
use macroquad::prelude::*;

use crate::adventure::Adventure;
use crate::view::{CURSOR, ENEMY, GROUND, LINE, PANEL, PLAYER, SIDEBAR, TARGET, banner, foot, log};
use crate::words::{named, place_name, status_text};

/// Side of a map cell, and where the map's top-left corner is.
const LARGEST_CELL: f32 = 56.0;
const MAP: Vec2 = vec2(32.0, 80.0);
/// Where the log's panel begins, which the map must stay above.
const LOG_TOP: f32 = 640.0;
const ROCK: Color = Color::new(0.2, 0.2, 0.22, 1.0);
const WATER: Color = Color::new(0.08, 0.14, 0.26, 1.0);
const WAVE: Color = Color::new(0.2, 0.32, 0.5, 1.0);

/// The side of a cell for a map of this size: as large as fits between the header, the sidebar and the log,
/// and no larger than suits the marks.
fn cell_size(width: u32, height: u32) -> f32 {
    let across = (SIDEBAR.x - 16.0 - MAP.x) / width.max(1) as f32;
    let down = (LOG_TOP - 16.0 - MAP.y) / height.max(1) as f32;
    across.min(down).min(LARGEST_CELL).floor()
}

pub fn draw(adventure: &Adventure) {
    header();
    map(adventure);
    sidebar(adventure);
    log(&adventure.log);
    if adventure.over() {
        banner(&status_text(adventure.expedition.status()));
    }
}

fn header() {
    text_at(t!("title"), 16.0, 32.0, 28.0, WHITE);
    text_fit(
        &t!("header-expedition"),
        150.0,
        32.0,
        620.0,
        18.0,
        LIGHTGRAY,
    );
    // Neither the expedition nor its rules are the original game's; the screen says so, always.
    text_fit(
        &t!("expedition-note"),
        SIDEBAR.x,
        32.0,
        SIDEBAR.w,
        18.0,
        CURSOR,
    );
}

/// Which rules the expedition is played under, where it can be more than one set.
fn rules_line(id: &str) -> Option<String> {
    match id {
        MODERN_RULESET => Some(t!("modern-note")),
        REFERENCE_RULESET => Some(t!("reference-note")),
        _ => None,
    }
}

fn map(adventure: &Adventure) {
    let expedition = &adventure.expedition;
    let def = expedition.def();
    let size = cell_size(def.map.width, def.map.height);
    let corner = |cell: Cell| MAP + vec2(cell.x as f32, cell.y as f32) * size;
    for y in 0..def.map.height {
        for x in 0..def.map.width {
            let cell = Cell { x, y };
            let at = corner(cell);
            let water = def.map.terrain(cell).is_some_and(|t| t.id == "water");
            if def.map.passable(cell) {
                draw_rectangle(at.x, at.y, size, size, GROUND);
            } else if water {
                // Ground nobody can walk is marked by its lines as well as its colour: waves on water.
                draw_rectangle(at.x, at.y, size, size, WATER);
                for row in [size / 3.0, size * 2.0 / 3.0] {
                    draw_line(
                        at.x + 10.0,
                        at.y + row,
                        at.x + size - 10.0,
                        at.y + row,
                        2.0,
                        WAVE,
                    );
                }
            } else {
                // And a peak on anything else that bars the way.
                draw_rectangle(at.x, at.y, size, size, ROCK);
                let (left, top, right) = (
                    at + vec2(10.0, size - 12.0),
                    at + vec2(size / 2.0, 12.0),
                    at + vec2(size - 10.0, size - 12.0),
                );
                draw_line(left.x, left.y, top.x, top.y, 2.0, LINE);
                draw_line(top.x, top.y, right.x, right.y, 2.0, LINE);
            }
            draw_rectangle_lines(at.x, at.y, size, size, 1.0, LINE);
        }
    }
    let clue = expedition.objective_known();
    for object in &def.map.objects {
        let centre = corner(object.cell()) + vec2(size / 2.0, size / 2.0 - 6.0);
        match object.kind {
            // A house: the town.
            ObjectKind::Town => {
                draw_rectangle(centre.x - 13.0, centre.y - 4.0, 26.0, 18.0, LIGHTGRAY);
                let roof = [vec2(0.0, -20.0), vec2(-18.0, -4.0), vec2(18.0, -4.0)];
                draw_triangle(
                    centre + roof[0],
                    centre + roof[1],
                    centre + roof[2],
                    LIGHTGRAY,
                );
            }
            // A keep with two towers: the king's castle.
            ObjectKind::Castle => {
                // The king's and the hero's own in the colour of the land's houses; one still to be taken in
                // the enemy's.
                let hostile = def.castle(&object.id).is_some() && !expedition.is_taken(&object.id);
                let colour = if hostile { ENEMY } else { LIGHTGRAY };
                draw_rectangle(centre.x - 14.0, centre.y - 6.0, 28.0, 20.0, colour);
                draw_rectangle(centre.x - 18.0, centre.y - 18.0, 9.0, 32.0, colour);
                draw_rectangle(centre.x + 9.0, centre.y - 18.0, 9.0, 32.0, colour);
            }
            // A tent: a dwelling.
            ObjectKind::Dwelling => {
                let tent = [vec2(0.0, -18.0), vec2(-17.0, 13.0), vec2(17.0, 13.0)];
                draw_triangle_lines(
                    centre + tent[0],
                    centre + tent[1],
                    centre + tent[2],
                    4.0,
                    LIGHTGRAY,
                );
            }
            // A box with its lid: a chest still closed.
            ObjectKind::Chest if !expedition.is_opened(&object.id) => {
                draw_rectangle(centre.x - 12.0, centre.y - 6.0, 24.0, 16.0, CURSOR);
                draw_rectangle_lines(centre.x - 12.0, centre.y - 12.0, 24.0, 8.0, 3.0, CURSOR);
            }
            // A hollow square, as the enemy's stacks are in battle: a band that still stands.
            ObjectKind::Encounter if !expedition.is_cleared(&object.id) => {
                draw_rectangle_lines(centre.x - 15.0, centre.y - 15.0, 30.0, 30.0, 8.0, ENEMY);
            }
            // A diamond: the prize, once a clue has named its place.
            ObjectKind::Objective if clue.is_some() => {
                draw_poly_lines(centre.x, centre.y, 4, 17.0, 0.0, 4.0, CURSOR);
            }
            ObjectKind::Encounter | ObjectKind::Objective | ObjectKind::Chest => continue,
        }
        let name = place_name(&object.id);
        let foot = corner(object.cell()) + vec2(size / 2.0, size - 5.0);
        text_centered_fit(&name, foot.x, foot.y, size * 2.0 - 8.0, 12.0, WHITE);
    }
    // The hero: a filled circle, as the player's own are filled in battle; in a corner of the cell, so that what
    // stands there still shows.
    let hero = corner(expedition.hero()) + vec2(size - 14.0, 14.0);
    draw_circle(hero.x, hero.y, 8.0, PLAYER);
    draw_circle_lines(hero.x, hero.y, 11.0, 2.0, WHITE);
}

fn sidebar(adventure: &Adventure) {
    let expedition = &adventure.expedition;
    let def = expedition.def();
    draw_rectangle(SIDEBAR.x, SIDEBAR.y, SIDEBAR.w, SIDEBAR.h, PANEL);
    draw_rectangle_lines(SIDEBAR.x, SIDEBAR.y, SIDEBAR.w, SIDEBAR.h, 2.0, LINE);
    // The lines are gathered first and set out afterwards, so that however many there are (a castle sells up
    // to five troops) they share the panel's height.
    let mut lines: Vec<(String, f32, Color)> = Vec::new();
    let mut line = |text: &str, size: f32, color: Color| lines.push((text.to_owned(), size, color));
    let clock = &def.clock;
    let time = t!(
        "adv-time",
        tick = expedition.ticks(),
        deadline = clock.deadline_ticks
    );
    line(&time, 22.0, WHITE);
    let to_week = clock.week_ticks - expedition.ticks() % clock.week_ticks;
    let upkeep = expedition.upkeep();
    let week = t!(
        "adv-week",
        n = to_week,
        income = expedition.weekly_income(),
        upkeep = upkeep
    );
    line(&week, 18.0, LIGHTGRAY);
    if let Some(creature) = expedition.week_creature() {
        let creature = t!("adv-week-creature", troop = named("troop", creature));
        line(&creature, 18.0, LIGHTGRAY);
    }
    line(&t!("adv-gold", n = expedition.gold()), 18.0, LIGHTGRAY);
    // In the king's service: the rank beside the leadership it gives, and how the next is earned.
    if let (Some(rank), Some(standing)) = (expedition.rank(), expedition.standing()) {
        let rank = t!(
            "adv-rank",
            rank = named("rank", &rank.id),
            n = expedition.leadership()
        );
        line(&rank, 18.0, LIGHTGRAY);
        let villains = expedition.next_rank().map_or_else(
            || t!("adv-villains-top", n = standing.villains),
            |next| {
                t!(
                    "adv-villains",
                    n = standing.villains,
                    needed = next.villains_needed
                )
            },
        );
        line(&villains, 18.0, LIGHTGRAY);
    } else {
        line(
            &t!("adv-leadership", n = expedition.leadership()),
            18.0,
            LIGHTGRAY,
        );
    }
    let army = expedition.army().iter();
    let army = army.map(|s| {
        t!(
            "adv-stack",
            troop = named("troop", &s.troop_id),
            n = s.count
        )
    });
    let army: Vec<String> = army.collect();
    let army = if army.is_empty() {
        t!("adv-no-army")
    } else {
        army.join(", ")
    };
    line(&format!("{}: {army}", t!("adv-army")), 18.0, LIGHTGRAY);
    let contract = expedition.contract().map_or_else(
        || t!("adv-no-contract"),
        |band| t!("adv-contract", name = place_name(band)),
    );
    line(&contract, 18.0, LIGHTGRAY);
    if expedition.objective_known().is_some() {
        line(&t!("adv-clue"), 18.0, CURSOR);
    }
    if let Some(object) = def.map.object_at(expedition.hero()) {
        let known = object.kind != ObjectKind::Objective || expedition.objective_known().is_some();
        if known {
            line(
                &t!("adv-here", name = place_name(&object.id)),
                18.0,
                LIGHTGRAY,
            );
        }
    }
    if let Some(rules) = rules_line(&def.rules.id) {
        line(&rules, 16.0, GRAY);
    }
    if adventure.unsaved {
        line(&t!("adv-save-failed"), 18.0, TARGET);
    }
    line("", 10.0, WHITE);

    keys(adventure, &mut line);
    line("", 10.0, WHITE);
    foot(&mut line);
    set_out(&lines);
}

/// Draws the sidebar's lines from its top: each a little below the last, as far apart as is comfortable, and
/// closer together when there are more than the panel holds at that distance.
fn set_out(lines: &[(String, f32, Color)]) {
    const COMFORTABLE: f32 = 1.45;
    const CLOSEST: f32 = 1.12;
    let (x, width) = (SIDEBAR.x + 16.0, SIDEBAR.w - 32.0);
    let top = SIDEBAR.y + 34.0;
    let sizes: f32 = lines.iter().map(|(_, size, _)| size).sum();
    // The last line needs no room below it but its own descenders.
    let room = SIDEBAR.h - 34.0 - 10.0;
    let apart = (room / sizes.max(1.0)).clamp(CLOSEST, COMFORTABLE);
    let mut y = top;
    for (text, size, color) in lines {
        text_fit(text, x, y, width, *size, *color);
        y += size * apart;
    }
}

/// The keys that do something now, as lines of the sidebar.
fn keys(adventure: &Adventure, line: &mut impl FnMut(&str, f32, Color)) {
    let expedition = &adventure.expedition;
    let def = expedition.def();
    let clock = &def.clock;
    if adventure.over() {
        line(&t!("adv-key-new"), 20.0, WHITE);
    } else {
        line(&t!("actions"), 20.0, WHITE);
        line(
            &t!("adv-key-step", ticks = clock.movement_ticks),
            18.0,
            LIGHTGRAY,
        );
        // What a place sells is listed only where it can be bought: in the town, on a dwelling, in the king's
        // castle. A line is dim when the rules would refuse.
        let buy = |troop: &str| Action::Recruit {
            troop_id: troop.to_owned(),
            quantity: 1,
        };
        for (place, (troop, price, left)) in adventure.on_sale().into_iter().enumerate() {
            let key = (place + 1).to_string();
            let name = named("troop", &troop);
            let text = match left {
                Some(stock) => t!(
                    "adv-key-recruit",
                    key = key,
                    troop = name,
                    price = price,
                    stock = stock
                ),
                None => t!(
                    "adv-key-recruit-any",
                    key = key,
                    troop = name,
                    price = price
                ),
            };
            let can = expedition.refusal(&buy(&troop)).is_none();
            line(&text, 18.0, if can { LIGHTGRAY } else { DARKGRAY });
        }
        // Siege weapons are offered in the town until they are bought; dim while the purse is too light.
        if let Some(cost) = def.siege_weapons_cost
            && expedition.hero() == def.town
            && !expedition.has_siege_weapons()
        {
            let can = expedition.refusal(&Action::BuySiegeWeapons).is_none();
            line(
                &t!("adv-key-siege", cost = cost),
                18.0,
                if can { LIGHTGRAY } else { DARKGRAY },
            );
        }
        // A closed chest under the hero: what it holds and what can be done with it.
        if let Some(chest) = expedition.chest_here() {
            match chest.holds {
                Holds::Gold(gold) => {
                    line(&t!("adv-key-chest-keep", gold = gold), 18.0, LIGHTGRAY);
                    let share = t!("adv-key-chest-share", leadership = gold / 50);
                    line(&share, 18.0, LIGHTGRAY);
                }
                Holds::Commission(more) => {
                    line(&t!("adv-key-chest-commission", n = more), 18.0, LIGHTGRAY);
                }
            }
        }
        // The audience is offered in the king's castle; dim while he has no rank to give.
        if def
            .king
            .as_ref()
            .is_some_and(|king| king.castle == expedition.hero())
        {
            let can = expedition.refusal(&Action::Promote).is_none();
            line(
                &t!("adv-key-promote"),
                18.0,
                if can { LIGHTGRAY } else { DARKGRAY },
            );
        }
        if let Some(band) = adventure.contract_offer() {
            line(
                &t!("adv-key-contract", name = place_name(band)),
                18.0,
                LIGHTGRAY,
            );
        }
        line(
            &t!("adv-key-search", ticks = clock.search_ticks),
            18.0,
            LIGHTGRAY,
        );
        line(&t!("adv-key-wait"), 18.0, LIGHTGRAY);
    }
}
