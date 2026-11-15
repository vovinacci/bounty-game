# Regression suites and baselines

A suite lists battles and eight seeds for each, played by the pilot `approach-v1` on both sides. All paths in a suite
file are relative to that file. The battles test the engine; they are not a calibration of King's Bounty balance.

| Suite | Rules | Battles | Prices for valuing losses | Baseline |
|---|---|---|---|---|
| `suite.json` | prototype (`prototype/`) | 4 x 8 | `metrics.json` | `baselines/prototype-0001.json` |
| `reference-suite.json` | reference (`content/reference/`) | 10 x 8 | `reference-metrics.json` | `baselines/reference-0002.json` |
| `modern-suite.json` | the variant (`content/modern/`), on the reference's battles | 10 x 8 | `reference-metrics.json` | `baselines/modern-0002.json` |

The prototype's numbers are never set beside the others': other rules, other troops. The reference's and the variant's
are, on purpose and by hand, in `docs/spec/10-GAMEPLAY-GAPS.md`; the tools hold each to its own baseline.

A test (`each_suite_gives_its_recorded_baseline`) and `make balance-check` compare the code as it stands with the
baselines, battle by battle, down to the final position. So:

- a change that is not meant to move results (a refactor, a new feature beside the old ones) must leave both green;
- a change that is meant to move results fails them, and that is the point where the change gets looked at.

## The recorded baselines

`baselines/prototype-0001.json`: the prototype suite at revision `84956d1`, accepted by the maintainer on 12 November
2026.

| Case | Player wins | Enemy wins | Round limit | Creatures lost, player / enemy |
|---|---|---|---|---|
| `melee_exchange` | 7 | 1 | 0 | 55 / 78 |
| `blocked_shooter` | 7 | 1 | 0 | 77 / 62 |
| `flight_barrier` | 0 | 8 | 0 | 64 / 30 |
| `unreachable_melee` | 0 | 0 | 8, as the case expects | 0 / 0 |

`baselines/reference-0001.json`: the reference suite on top of revision `ba1a341`, with the case `loss_of_control_deep`
added at the maintainer's word on 12 November 2026.

| Case | What it is | Player wins | Enemy wins | Creatures lost, player / enemy |
|---|---|---|---|---|
| `even_fight` | equal armies of Sprites, Wolves and Archers | 6 | 2 | 448 / 500 |
| `morale_sprites_wolves` | equal armies; the player's Sprites have Low morale, the enemy's none | 3 | 5 | 542 / 482 |
| `loss_of_control` | the player's Wolves a little over the leadership; back under control after first losses | 7 | 1 | 131 / 132 |
| `loss_of_control_deep` | the player's Wolves at twice the leadership; out of control most of the battle | 0 | 8 | 608 / 403 |
| `overmatch` | a fight the player must win | 8 | 0 | 0 / 80 |

Eight seeds are few: over 500 seeds `loss_of_control` is won 69 times in 100, not 7 in 8. The baseline pins the engine
down; it is not an estimate of how hard a battle is.

`baselines/modern-0001.json`: the same battles under `modern-v0.1` (the leadership caps the fighters; morale by the
army-view rule), on top of revision `e734935`, as the maintainer set the variant on 13 November 2026.

| Case | Player wins | Enemy wins | Creatures lost, player / enemy | Against the reference |
|---|---|---|---|---|
| `even_fight` | 3 | 5 | 501 / 491 | fewer wins: by the army-view rule the player's Wolves are at Low beside Sprites |
| `morale_sprites_wolves` | 7 | 1 | 160 / 556 | the ten Wolves are halved, not the sixty Sprites |
| `loss_of_control` | 8 | 0 | 98 / 144 | no mutiny; all but one of the Wolves fight |
| `loss_of_control_deep` | 6 | 2 | 489 / 499 | no mutiny; half the Wolves fight |
| `overmatch` | 8 | 0 | 0 / 80 | the same |

## Expedition routes

Battles alone do not show what a rule costs over a whole game. `bounty-tools expedition run` plays the routes an
expedition file carries (its `tests`), the pilot fighting the battles, and reports for each route its time, the gold
that came in (weekly income, rewards) and went out (upkeep, recruits, recovery), the creatures lost and how it ended. A
route whose gold does not add up, that is refused a command or that does not repeat is a problem, not a row.

`baselines/expedition-demo-0001.json`: the four routes of `content/demo/` under the prototype rules. A test
(`each_expedition_gives_its_recorded_baseline`) and `make balance-check` hold the code to it, field by field.

| Route | Ends | Ticks | Gold in: income + rewards | Gold out: upkeep + recruits + recovery | Purse, 100 at start | Creatures lost |
|---|---|---|---|---|---|---|
| `complete-route` | victory | 15 | 0 + 90 | 0 + 40 + 0 | 150 | none |
| `surrender-first-encounter` | goes on | 7 | 0 + 0 | 0 + 40 + 30 | 30 | 12 Wardens, the army given up |
| `timeout-at-town` | timeout | 24 | 20 + 0 | 10 + 0 + 0 | 110 | none |
| `success-on-deadline` | victory | 24 | 20 + 90 | 12 + 40 + 0 | 158 | none |

These are the fixture's own numbers, synthetic like its rules: they pin the engine down and say nothing about the
original's economy.

`baselines/expedition-meadow-reference-0005.json` and `expedition-meadow-modern-0005.json`: the two routes of
`content/meadow/`, each of which hands the whole expedition to a pilot, under the reference rules and under the variant
(`--rules` names the rules file; without it the expedition's own are used). `errand` is played by `errand-v1`, which
takes the one contract that gives the clue; `career` by `errand-v2`, which holds a contract on every band it fights and
goes to the king when a rank is earned.

| Rules | Route | Ends | Ticks | Purse, 3000 at start | Bought | Creatures lost | Returned | Left at the end |
|---|---|---|---|---|---|---|---|---|
| reference | `errand` | victory | 24 | 3525 | 10 Archers, 5 Wolves | 7 Archers, 20 Sprites, 5 Wolves | none | 3 Archers |
| reference | `career` | victory | 38 | 2514 | 10 Archers, 24 Wolves | 20 Sprites, 5 Wolves | none | 10 Archers, 19 Wolves |
| variant | `errand` | victory | 24 | 3424 | 10 Archers, 5 Wolves | 3 Archers, 19 Sprites, 5 Wolves | 1 Archers, 6 Sprites, 1 Wolves | 7 Archers, 1 Sprites |
| variant | `career` | victory | 38 | 2534 | 10 Archers, 24 Wolves | 1 Archers, 19 Sprites, 4 Wolves | 6 Sprites, 1 Wolves | 9 Archers, 1 Sprites, 20 Wolves |

Against the `-0004` baselines, which had the one route `errand` and no king: the meadow's hero is now a Knight, whose
commission of 1000 a week replaces the flat 500, so `errand` ends with 500 more gold and is otherwise the same row.
`career` is new. It takes 14 ticks longer, for the walks back to the town and to the castle; it ends as a General with
two villains caught. Back in the town between the bands, it buys 24 Wolves where `errand` buys 5, and under the
reference rules it loses no Archer where `errand` loses seven.

`baselines/expedition-vale-reference-0005.json` and `expedition-vale-modern-0005.json`: the three routes of
`content/vale/`, a Knight with 7500 gold, twenty Militia and two Archers, 200 ticks to find the prize. `conquest` is
played by `errand-v5`, which besieges; `muster` by `errand-v3`, which recruits wherever troops are sold; `treasure` by
`errand-v4`, which also opens the vale's three chests.

| Rules | Route | Ends | Ticks | Purse | Battles won | Bought | Creatures lost | Left at the end |
|---|---|---|---|---|---|---|---|---|
| reference | `conquest` | victory | 158 | 1601 | 6 of 6 | 18 Archers, 30 Militia, 23 Orcs, 31 Pikemen, 115 Skeletons, 106 Wolves | 3 Archers, 50 Militia, 10 Pikemen, 76 Skeletons, 51 Wolves | 17 Archers, 22 Orcs, 21 Pikemen, 39 Skeletons, 54 Wolves |
| reference | `muster` | timeout | 200 | 5973 | 4 of 4 | 17 Archers, 30 Militia, 20 Orcs, 20 Pikemen, 33 Skeletons, 70 Wolves | 10 Archers, 50 Militia, 1 Orcs, 10 Pikemen, 5 Skeletons, 44 Wolves | 9 Archers, 19 Orcs, 10 Pikemen, 28 Skeletons, 26 Wolves |
| reference | `treasure` | timeout | 200 | 6486 | 4 of 4 | 12 Archers, 30 Militia, 23 Orcs, 18 Pikemen, 39 Skeletons, 57 Wolves | 3 Archers, 50 Militia, 7 Pikemen, 22 Wolves | 11 Archers, 23 Orcs, 11 Pikemen, 39 Skeletons, 35 Wolves |
| variant | `conquest` | victory | 154 | 2081 | 6 of 6 | 17 Archers, 89 Militia, 23 Orcs, 41 Pikemen, 90 Wolves | 9 Archers, 99 Militia, 5 Orcs, 26 Pikemen, 51 Wolves | 10 Archers, 10 Militia, 16 Orcs, 15 Pikemen, 39 Wolves |
| variant | `muster` | timeout | 200 | 5852 | 4 of 4 | 16 Archers, 74 Militia, 20 Orcs, 18 Pikemen, 51 Wolves | 8 Archers, 47 Militia, 10 Pikemen, 24 Wolves | 10 Archers, 47 Militia, 20 Orcs, 8 Pikemen, 27 Wolves |
| variant | `treasure` | timeout | 200 | 5452 | 4 of 4 | 17 Archers, 58 Militia, 23 Orcs, 19 Pikemen, 53 Wolves | 8 Archers, 57 Militia, 9 Pikemen, 17 Wolves | 11 Archers, 21 Militia, 23 Orcs, 10 Pikemen, 36 Wolves |

Against `-0004`: the vale's town gives its contracts in turn now, on the lords of its two castles and on nobody else;
the keep, which was held by monsters, has a lord. A band is beaten for its gold and is no villain.

- `conquest` beats the four bands and then both lords, the keep's first, as the town's turn has it; the second lord's
  clue and the king's rank of General come together. It ends with far less gold than the others, 3000 of it gone on
  siege weapons.
- `muster` and `treasure` beat the four bands and catch nobody, so they stay Knights; then they wait out the clock.
- The route `career` is gone. Its pilot, `errand-v2`, buys only in a town and fights only under contract, and the vale's
  town sells no troops and gives no contract on a band: it did nothing for 200 ticks.
- To let `conquest` finish under the variant too, the hold's garrison was made lighter (two Knights, eight Pikemen, six
  Archers, twenty Militia): with three, ten, eight and thirty the variant lost to it three times.
- While the vale was made, its last band had five Ghosts, and `muster` lost to it four times under the reference rules:
  an army of Militia and Sprites feeds Ghosts, which grow by what they kill. With two Ghosts it wins. The numbers of the
  vale are this project's own and were set so that a route can be finished.

One run each pins the engine down; what the rules cost over 500 runs is in `docs/spec/10-GAMEPLAY-GAPS.md`. The
variant's battle baseline `modern-0001.json` stands: its rules file gained the campaign's section, which no battle
reads, so the comparison notes another hash of the rules and no battle that ended differently.

`baselines/reference-0002.json` and `baselines/modern-0002.json`: both suites with five more battles, on top of revision
`fec8fd4`, once the rules had all twenty-five troops and their abilities (14 November 2026). The forty rows of the first
five cases are those of the `-0001` baselines, unchanged. The new cases, under the reference rules:

| Case | What it is | Player wins | Enemy wins | Creatures lost, player / enemy |
|---|---|---|---|---|
| `five_groups` | equal armies with a troop of each morale group; only the player has a hero | 7 | 1 | 1002 / 1124 |
| `undead_host` | Vampires, Ghosts, Zombies against castle troops of the same price | 4 | 4 | 318 / 175 |
| `trolls_hold` | Trolls and Ogres against three cheaper stacks of the same price | 8 | 0 | 2 / 288 |
| `scythe` | Demons and Cavalry against Giants, Dwarves, Gnomes of the same price | 1 | 7 | 60 / 80 |
| `magic_on_dragons` | Druids, Archmages, Knights against Dragons and Elves | 0 | 8 | 136 / 0 |

and under the variant:

| Case | What it is | Player wins | Enemy wins | Creatures lost, player / enemy |
|---|---|---|---|---|
| `five_groups` | equal armies with a troop of each morale group; only the player has a hero | 3 | 5 | 1109 / 1071 |
| `undead_host` | Vampires, Ghosts, Zombies against castle troops of the same price | 4 | 4 | 308 / 187 |
| `trolls_hold` | Trolls and Ogres against three cheaper stacks of the same price | 8 | 0 | 2 / 288 |
| `scythe` | Demons and Cavalry against Giants, Dwarves, Gnomes of the same price | 1 | 7 | 63 / 81 |
| `magic_on_dragons` | Druids, Archmages, Knights against Dragons and Elves | 0 | 8 | 136 / 0 |

What they say, with eight seeds each and the pilot that walks up and strikes:

- `trolls_hold`: not one Troll is lost in eight battles against the same gold. An injury that is gone every round means
  a stack of large creatures is only hurt by a blow that kills one outright.
- `magic_on_dragons`: the enemy loses nothing. The pilot shoots at the first stack it finds, the Dragons, which magic
  cannot hurt, and so spends every magic shot there. This is the pilot's doing, recorded as it is: the case will move
  when the pilot learns, and that change is to be measured as one.
- `scythe`: the Demons' side loses seven of eight, six of them within three rounds without killing a Giant or a Dwarf.
  Three Demons and five Cavalry are few against four Giants that shoot.
- `undead_host`: four wins each under the reference rules. In three of the four wins the ten Ghosts end as 22 to 28; of
  the six Vampires at most three are left. In the losses nothing of the side is.
- `five_groups`: the first battle in which all five morale groups stand side by side. The player wins seven of eight
  under the reference rules and three of eight under the variant, where morale is read from the army as it stands.

## Replacing a baseline

Only for a change that is meant to move results, in the same pull request as that change:

1. `make balance`, then compare with the old baseline (`make balance-check` prints every battle that moved).
2. Read the differences. Each one is either what the change was for, or a surprise to explain before going on.
3. Save the new report under `baselines/` with the next number (`prototype-0002.json`, `modern-0002.json`), point
   `baseline_path` in the suite file at it (for an expedition: `BALANCE_EXPEDITIONS` in the `Makefile` and the test
   `each_expedition_gives_its_recorded_baseline`), and keep the old file: the history of baselines stays readable
   without git.
4. Say in the pull request description what moved and why, with the numbers. That text becomes the commit message, so
   the reason travels with the baseline.

Not for a change that leaves every row as it was. A comparison notes another revision, another rules file or, for an
expedition, another hash of its content ("run differs: ...") and is still a pass: a troop added to the rules that no
battle or route meets needs no new baseline.

Never replace a baseline to make a failing check pass without having read why it fails. A report that lists problems (a
refused command, an outcome the case does not allow, a run that does not repeat) cannot become a baseline.
