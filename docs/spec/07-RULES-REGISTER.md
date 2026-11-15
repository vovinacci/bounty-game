# 07. Rules and fidelity register

## Baseline

Target: original DOS King's Bounty. **Exact executable/version/hash: not selected.** No reference executable was run for
this draft. This blocks an exact-DOS label, not an implementation using explicitly provisional source-backed rules.

Record the legal source of the reference copy, executable hash, displayed version if any, language, emulator
configuration and relevant data-file hashes during M0 when available. Otherwise record the missing source explicitly;
source support cannot be claimed until pinned. A packaging version from a download site is not necessarily the game's
executable version.

## Small confirmed-documentation anchor

The original manual describes per-stack leadership, a five-day week, a siege-equipment prerequisite, a limited spell
inventory, one hero combat spell per round, and a search-based victory. It permits only one retaliation per troop per
round and forbids shooting beside an enemy. A clue can come from a contracted villain or an artefact; failed searches
cost more time than successful ones. These are **M**, not executable-verified conclusions. [S1]

The combat headers corroborate a square board of six columns by five rows and five army slots. [S4]

The reference project/catalogue describes four classes, four continents, 25 troop types, 14 spells, 17 villains and
eight artefacts. Treat those counts as **R** until the selected DOS edition is audited. [S3, S7]

Do not infer hexes, later-game mana/rage, rival kingdom turns or HoMM-style building queues from the franchise name.

## Source conflicts matter

The OpenBounty spec explicitly records differences from both OpenKB and DOS, including purchase-boundary behaviour.
Copying a modern engine can copy its deviations. [S3] Likewise, a manual's descriptive wording may leave an equality
case ambiguous. Only a recorded decision or an executable test should settle such a boundary in Classic.

## Research queue

Each implemented row needs a source-backed decision and a test. Independent DOS observation is required for a
verified-parity claim, not for every provisional release. "Observed" means reproduced against the selected original;
"adopted" means a declared rule choice. Unimplemented subsystems may remain deferred.

| ID | Question | Verification experiment / record | Gate |
|---|---|---|---|
| U-01 | Which exact DOS edition is authoritative? | Record executable/data hashes, version, language and emulator settings. | M0 |
| U-02 | Starting values and progression for every class/difficulty? | Capture start states and each promotion; compare tables and exact thresholds. | M3 |
| U-03 | Travel cost, terrain penalties and boundary ordering? | Walk controlled paths at day/week edges; record before/after state and enemy updates. | M3 |
| U-04 | Sailing/flight access, landing, expiry and region transitions? | Test valid and invalid terrain, lost eligibility, coast transitions and return routes. | M3 |
| U-05 | Weekly income, upkeep, recruitment refresh and insolvency? Reference lead, implemented for `content/meadow/`: upkeep is a tenth of a creature's recruiting cost a week, rounded down, taken after the income; a purse too small is emptied and every troop stays (OpenBounty spec 13.4, REQ-264). Not observed in the original. | Test exact funds, one below, zero, multiple boundaries and temporary modifiers. | M3 |
| U-06 | Leadership equality, injury interactions and loss of control? | Test below/at/above threshold, growth during battle and modifier expiry. | M1b implemented subset |
| U-07 | Permitted army operations and garrison behaviour? | Test same-type recruitment, full army, splitting/merging, dismissal and transfers. | M3 |
| U-08 | Battle turn order, wait queue and phase resets? | Compare slot order with differing movement rates, waits and deaths; do not assume speed initiative. | M1b implemented subset |
| U-09 | Movement, diagonal obstruction, flight and siege layout? | Small obstacle cases and every boundary cell; record costs and reachability. | M1b implemented subset |
| U-10 | Damage formula, skill comparison, rolls and rounding? | Controlled attack pairs and sufficient samples; separate exact arithmetic from distribution estimates. | M1b implemented subset |
| U-11 | Retaliation count source and injury persistence? | Compare first/second attack, partial wounds, death, end of activation and end of battle. | M1b implemented subset |
| U-12 | Ammunition, adjacent shooters, range and special abilities? | Isolate each ability; test interactions with attack, death and control changes. | Implemented scope / parity claim |
| U-13 | Spell eligibility, exact effects, limits and expiry? | One fixture per spell plus invalid-target, cancellation, stacking and boundary cases. | Implemented scope / parity claim |
| U-14 | Contracts, expiry/rotation, escape, relocation and clue rewards? | Defeat the same villain with/without the correct contract; inspect persistent state. | M3 |
| U-15 | Defeat and surrender recovery? | Record all changed and retained fields; compare repeated defeats and depleted funds. | M3 |
| U-16 | Objective placement, puzzle mapping, search cost and final-day precedence? | Known objective fixture, wrong tile, early solution and deadline boundary. | M3 |
| U-17 | Exact maps, random placement pools, counts and reachability? | Compare multiple starts; isolate fixed layout from variable placements. | Implemented scope / parity claim |
| U-18 | Artefact stacking, morale and score formulas? | Controlled before/after records and modifier combinations. | Implemented scope / parity claim |
| U-19 | Enemy battle and wandering policies? | Repeat legal choice situations; record tie-breaks and information available. | Reference AI label |
| U-20 | All content statistics and identifiers? | Populate a provenance-backed catalogue; validate each imported field and ability. | Implemented scope / parity claim |

## Evidence-record template

```text
Evidence ID:
Rule IDs:
Reference build/data hashes:
Initial state or save identification:
Input sequence:
Observed outcome:
Manual statement, if relevant:
Reimplementation lead, if relevant:
Disagreement / uncertainty:
Chosen remake behaviour:
Status: deferred | reference_derived | observed_dos | adopted_deviation
Test fixture and expected result:
Reviewer/date:
```

Store only evidence you have permission to distribute. Private reference saves and captures may remain outside the
public repository; publish a concise reproducible description where appropriate.

## Numeric catalogue policy

Every original numeric value needs a record with field path, value, units, source ID, evidence ID and confidence/status.
Missing values are not zero and are not prototype defaults. The catalogue validator should distinguish unknown from
intentionally absent.

Before a complete campaign profile is release-ready, fill its starting statistics, troop properties/abilities, spell
effects, timing, prices, upkeep, morale, encounters, progress thresholds, artefact modifiers, scoring and maps. This
pack intentionally does not pretend that those historical tables have already been reconstructed.

## Deviation policy

Preserve meaningful rules when known; document intentional fixes. Compatibility does not require reproducing a crash,
unsafe file handling or unreadable controls. A gameplay-changing fix belongs to a named rule revision or an explicit
experimental profile. Explain the difference in release notes.

## Practical research method and stop rule

### Evidence states

Use `observed_dos`, `reference_derived`, `adopted_deviation`, or `deferred`. A pinned OpenKB/OpenBounty implementation
is a permissible provisional semantic reference. Read its documentation and corresponding code; record path/function,
revision and known discrepancies. Two descendant projects are not independent confirmation. Copying source or assets
still requires their applicable licenses.

U-08 concerns activation order, not an instruction to add a speed initiative system. OpenBounty explicitly documents
player-first, slot-order activation [S11]. Test the selected DOS edition if exact ordering is important; do not infer
later HoMM mechanics from the word "speed".

### Investigation sequence

1. Inspect documented formats and source-derived logic first [S8, S9]. Separate extractable tables/maps from executable
   algorithms such as damage or AI.
2. Make a small before/after save-file or memory comparison around one action. Search a bounded range for the known
   visible value; test a second unrelated value and a second initial state before accepting a candidate location.
3. When necessary, use a debugger-enabled DOSBox-X build [S10]. Start it from a terminal on Linux/macOS, use its own
   HELP to verify commands, inspect a known segment:offset with D and dump a bounded region with MEMDUMPBIN. Record the
   executable hash, load segment, encoding, width and signedness. Addresses are edition/run-specific; do not invent a
   universal memory offset.
4. Set an appropriate breakpoint/watchpoint supported by that exact build and inspect the transition, not a screenshot
   approximation. Memory-watch syntax varies; this pack does not pretend a protected-mode breakpoint is a valid
   real-mode KB watch command. Use controlled dumps when a watchpoint is unavailable.
5. Store a concise input/observed-output fixture. Private dumps/saves stay local. Automate repeated inputs and dump
   comparison after the first case is reliable; do not promise wall-clock keyboard scripts are intrinsically
   deterministic.

Tools required initially: source reader, byte/hex viewer, a small local dump-diff script if needed, and the selected
emulator's debugger. A reusable debugger bridge or automated reverse-engineering service is not a project prerequisite.

### Proposed effort cap

Per disputed rule: one source pass and one controlled experiment, capped at four developer-hours unless the maintainer
explicitly chooses to investigate further. This is a proposed project budget, not an estimate for work performed here.
At the cap, adopt the best source-backed behaviour with a visible provisional/deviation label, defer the affected
optional feature, or explicitly prioritise a true blocker. Do not let a low-impact quirk indefinitely block a useful
hobby release.

Prioritise: damage/skill and injury, activation/retaliation, morale/control, affordability/weekly finance, then the
final objective and recovery. Source-backed starting decisions are sufficient to code tests; exact historical claims
require stronger evidence.

No DOS executable, memory experiment or binary parser has been run for v0.3.

## Reference-derived rules in use: `reference-v0.1`

The first rules and numbers of the original game in this repository. All of them are `reference_derived`: read from the
OpenBounty project [S2] at revision `aace551ca625e0c723adff0cb13b22ca2f806dee` (MIT), restated here and typed into
`content/reference/rules.json`. No code was copied. None has been observed in the DOS game by this project, and
OpenBounty describes itself as descended from OpenKB, so the two are one line of evidence, not two.

Files read: `assets/kings-bounty/game.json` (troop table, morale chart), `docs/OPENBOUNTY-SPEC.md` sections 13, 14 and
25, `engine/combat.c`.

### Troops

All twenty-five. A troop is never fielded without an ability it has: a rules file that names one the rules lack is
refused. The first three in the repository were Sprites, Wolves and Archers, chosen to cover the three roles and to make
Low morale reachable (groups A, B and C never give Low among themselves; it needs a troop of group D or E).

| Troop | Role | Skill | Health | Movement | Melee | Shot | Shots | Flies | Abilities | Group | Cost | Home | Holds | Grows |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Peasants | melee | 1 | 1 | 1 | 1-1 |  | 0 | no |  | A | 10 | plains | 250 | 6 |
| Sprites | flying | 1 | 1 | 1 | 1-2 |  | 0 | yes |  | C | 15 | forest | 200 | 6 |
| Militia | melee | 2 | 2 | 2 | 1-2 |  | 0 | no |  | A | 50 | castle |  | 5 |
| Wolves | melee | 2 | 3 | 3 | 1-3 |  | 0 | no |  | D | 40 | plains | 150 | 5 |
| Skeletons | melee | 2 | 3 | 2 | 1-2 |  | 0 | no | undead | E | 40 | dungeon | 150 | 5 |
| Zombies | melee | 2 | 5 | 1 | 2-2 |  | 0 | no | undead | E | 50 | dungeon | 100 | 5 |
| Gnomes | melee | 2 | 5 | 1 | 1-3 |  | 0 | no |  | C | 60 | forest | 250 | 5 |
| Orcs | ranged | 2 | 5 | 2 | 2-3 | 1-2 | 10 | no |  | D | 75 | hill | 200 | 5 |
| Archers | ranged | 2 | 10 | 2 | 1-2 | 1-3 | 12 | no |  | B | 250 | castle |  | 5 |
| Elves | ranged | 3 | 10 | 3 | 1-2 | 2-4 | 24 | no |  | C | 200 | forest | 100 | 4 |
| Pikemen | melee | 3 | 10 | 2 | 2-4 |  | 0 | no |  | B | 300 | castle |  | 4 |
| Nomads | melee | 3 | 15 | 2 | 2-4 |  | 0 | no |  | C | 300 | plains | 150 | 4 |
| Dwarves | melee | 3 | 20 | 1 | 2-4 |  | 0 | no |  | C | 350 | hill | 100 | 4 |
| Ghosts | melee | 4 | 10 | 3 | 3-4 |  | 0 | no | absorb, undead | E | 400 | dungeon | 25 | 3 |
| Knights | melee | 5 | 35 | 1 | 6-10 |  | 0 | no |  | B | 1000 | castle |  | 3 |
| Ogres | melee | 4 | 40 | 1 | 3-5 |  | 0 | no |  | D | 750 | hill | 200 | 3 |
| Barbarians | melee | 4 | 40 | 3 | 1-6 |  | 0 | no |  | C | 750 | plains | 100 | 3 |
| Trolls | melee | 4 | 50 | 1 | 2-5 |  | 0 | no | regeneration | D | 1000 | forest | 25 | 3 |
| Cavalry | melee | 4 | 20 | 4 | 3-5 |  | 0 | no |  | B | 800 | castle |  | 2 |
| Druids | ranged | 5 | 25 | 2 | 2-3 | 10 | 3 | no | magic shot | C | 700 | forest | 25 | 2 |
| Archmages | flying | 5 | 25 | 1 | 2-3 | 25 | 2 | yes | magic shot | C | 1200 | plains | 25 | 2 |
| Vampires | flying | 5 | 30 | 1 | 3-6 |  | 0 | yes | leech, undead | E | 1500 | dungeon | 50 | 2 |
| Giants | ranged | 5 | 60 | 3 | 10-20 | 5-10 | 6 | no |  | C | 2000 | hill | 50 | 2 |
| Demons | flying | 6 | 50 | 1 | 5-7 |  | 0 | yes | scythe | E | 3000 | dungeon | 25 | 1 |
| Dragons | flying | 6 | 200 | 1 | 25-50 |  | 0 | yes | immune | D | 5000 | hill | 25 | 1 |

A flying troop that also shoots is listed as flying. The reference writes a magic shot as three numbers of which it
reads the first; here it is that one number.

"Home" is where the troop is recruited: a dwelling of that kind, or the king's castle. "Holds" is what a dwelling of the
troop has when it is full; the castle's troops are sold without limit (the reference gives the Knights a number that
nothing reads, left out here). "Grows" is by how many a stack of the troop outside the hero's army grows in the troop's
week.

The troop table, and the table of the heroes' classes below, are exceptions to the rule that the original's numbers stay
out of the repository. The maintainer allowed three troops first, so that the reference rules could be tested in CI; on
14 November 2026 the whole troop table, and on the same day the classes and their ranks: without them neither the rules,
nor a troop's ability, nor a hero's promotion can be tested or measured on what they are for. The exception covers what
a battle and a recruiter need of a troop (the columns above; dwelling, population and growth when the campaign reads
them) and what the king gives a hero. Maps, texts, pictures and the rest of the catalogue stay out.

The same day the maintainer allowed the tables of the economy as they are needed: what a chest gives and what the
services of a town cost. Names, armies and places of villains and castles stay out with the maps; an expedition of this
repository has its own.

### The hero's classes and ranks

Four classes of four ranks each (U-02), read from the reference's `game.json` and its text, section 8. Every number of a
rank but the villains it asks for is what the rank adds: a hero has the leadership, commission, spells and spell power
of all the ranks reached, together with whatever was gained elsewhere. The king gives a rank at an audience, once the
hero has caught the villains it asks for; with enough of them, more than one rank at once.

**Knight**: sets out with 7500 gold and 20 Militia, 2 Archers.

| Rank | Villains | Leadership | Spells | Spell power | Commission | Knows magic | Instant army |
|---|---|---|---|---|---|---|---|
| Knight | 0 | 100 | 2 | 1 | 1000 | no | Peasants |
| General | 2 | 100 | 3 | 1 | 1000 | no | Militia |
| Marshal | 8 | 300 | 4 | 1 | 2000 | no | Archers |
| Lord | 14 | 500 | 5 | 2 | 4000 | no | Knights |

**Paladin**: sets out with 10000 gold and 20 Peasants, 20 Militia.

| Rank | Villains | Leadership | Spells | Spell power | Commission | Knows magic | Instant army |
|---|---|---|---|---|---|---|---|
| Paladin | 0 | 80 | 3 | 1 | 1000 | no | Peasants |
| Crusader | 2 | 80 | 3 | 1 | 1000 | no | Militia |
| Avenger | 7 | 240 | 6 | 2 | 2000 | no | Archers |
| Champion | 13 | 400 | 5 | 2 | 4000 | no | Cavalry |

**Sorceress**: sets out with 10000 gold and 30 Peasants, 10 Sprites.

| Rank | Villains | Leadership | Spells | Spell power | Commission | Knows magic | Instant army |
|---|---|---|---|---|---|---|---|
| Sorceress | 0 | 60 | 5 | 2 | 3000 | yes | Sprites |
| Magician | 3 | 60 | 8 | 3 | 1000 | no | Gnomes |
| Mage | 6 | 180 | 10 | 5 | 1000 | no | Elves |
| Archmage | 12 | 300 | 12 | 5 | 1000 | no | Druids |

**Barbarian**: sets out with 7500 gold and 20 Wolves.

| Rank | Villains | Leadership | Spells | Spell power | Commission | Knows magic | Instant army |
|---|---|---|---|---|---|---|---|
| Barbarian | 0 | 100 | 2 | 0 | 2000 | no | Peasants |
| Chieftain | 1 | 100 | 2 | 1 | 2000 | no | Wolves |
| Warlord | 5 | 300 | 3 | 1 | 2000 | no | Orcs |
| Overlord | 10 | 500 | 3 | 1 | 2000 | no | Ogres |

"Knows magic" is set by one class's first rank only; the others learn it at a place on the map. "Instant army" names the
troop a spell of that name raises at the rank. "Knows magic", "Spells" and "Spell power" are read where an expedition
has magic (see "The hero's magic"); "Instant army" waits for its spell.

How it is carried out here, in an expedition whose file names a king (his castle on the map, the hero's class):

- The hero sets out with what the first rank gives. The file must say the same leadership and weekly income, so that
  there is one answer to what a Knight starts with.
- A band beaten while its contract is held is a villain caught; beaten without the contract it is only beaten (gap C4 of
  `10-GAMEPLAY-GAPS.md` is the reference's rule, kept).
- In the king's castle the hero asks for his rank. Every rank the villains caught have earned is given at once, each
  adding its leadership and commission; the commission is the week's income from then on, and recruiting and battles use
  the new leadership. Asked elsewhere, or with too few villains, or at the highest rank, the command is refused and
  nothing changes.
- An audience takes no time. Status: `adopted_deviation`; the reference does not say what an audience costs, and nothing
  on the way to the castle is free.
- Spell limit and spell power are not kept in the hero's state: they are worked out from the ranks he has reached and
  the chests he has opened, where the expedition has magic.

An expedition without a king is as it was: leadership and income stay what it starts with, and its saved state is
written as before. The hero's standing (rank, leadership, commission, villains caught) is written only where there is a
king, and a save whose standing is not what its rank and villains make it is refused.

The meadow has a king and a Knight for a hero. In the game the audience is the key P in the castle; the expedition pilot
`errand-v2` holds a contract on every band it fights and goes to the king when a rank is earned.

### Dwellings, the week's creature and the king's castle as a recruiter

Read from the reference's text, sections 13.4, 17.4, 18 and 24 (U-05, U-07). In an expedition whose file has dwellings,
and a seed for the weeks' creatures:

- A dwelling holds one troop, is full when the expedition starts, and sells that troop at the rules' price until none
  are left. What limits a purchase is what limits it in the town: the purse, and the leadership over the whole stack.
- Every week has a creature. Every fourth week, the first included, it is the first troop of the rules (the Peasants);
  any other week's is the troop at place 1 + ((seed xor week) x 1664525 + 1013904223) mod (troops - 1), in 32 bits. So a
  week has the same creature whenever it is asked for.
- When a week begins, the dwellings of its creature are full again. No dwelling grows at any other time, which is gap C5
  of `10-GAMEPLAY-GAPS.md`: kept, as the reference's rule.
- In the week of the first troop, a stack of the army that grows by what it kills (Ghosts) becomes that troop, creature
  for creature, in its place in the army.
- The king's castle sells the troops whose home is the castle, as many as are asked for, each only to a hero whose
  leadership is at least six of its creatures' health.

Differences and what is left out:

- In the reference the hero is thrown back from a dwelling and deals with it from outside; here he stands on it, as on
  the town. Status: `adopted_deviation`.
- The week's creature also grows in every band that still stands, by what the troop grows in a week (the table's last
  column). The reference does the same in castles the hero does not own; those come with the castles. A band's numbers
  are state only where the weeks have a creature; elsewhere a band is what its file says.
- The reference creates a dwelling's troop from the world's seed where a map does not fix it. Here an expedition names
  the troop of every dwelling.

The vale (`content/vale/`) has four dwellings, a seed for its weeks and the king's castle; its town sells no troops, so
an army there comes from these. The tests of the rules themselves write a den of Wolves into the meadow.

### Castles, siege weapons and their lords

Read from the reference's text, sections 16, 17 and 21, and its siege layout from `engine/combat.c` (U-07, U-09, U-14).
In an expedition whose file has castles beside the king's:

- A castle is held by monsters, or by a lord, with a garrison of one to five stacks. All of them fight a siege, where a
  band in the open fields what its file says.
- A castle is not entered without siege weapons, which a town sells once, at the expedition's price. Without them the
  step onto the castle is refused and nothing changes.
- A siege is fought on fixed ground, for a board of six columns and five rows: walls down both sides and in the two
  lower corners; the besiegers at the foot, in the order of the army, at (2,4), (3,4), (2,3), (3,3), (1,3); the garrison
  at the head, at (2,0), (3,0), (1,0), (4,0), (2,1).
- A castle of monsters that falls is the hero's, and he stands in it.
- A lord's castle that falls while his contract is held is the hero's too: the lord is a villain caught, his reward is
  paid, and he gives his clue if he has one.
- A lord's castle that falls without the contract is not kept: he goes free, the castle is his again with the garrison
  it first had, and the hero stands before the gate. He has to be fought again with the contract. This is gap C4 of
  `10-GAMEPLAY-GAPS.md`: kept, as the reference's rule.
- A contract is taken in the town on a band that stands, as before, or on a lord at large. Monsters have none.
- The week's creature grows in the castles that are not the hero's as it does in the bands.
- In a castle of his own the hero leaves creatures of the army and takes them back: to those of their troop already
  there, or into a free place, of which a castle has as many as an army has slots. None is lost or made on the way, and
  the army keeps at least one creature. Those left in a castle cost no upkeep.
- When a week ends, a castle of the hero's with nobody in it is held against him again. Its lord, if it had one, stays
  caught: it is then a castle of monsters, taken without a contract and without a reward.

Differences and what is left out:

- The reference turns the hero back from a gate without siege weapons and says nothing; here the command is refused with
  its reason, as every refused command is. Status: `adopted_deviation`.
- In the reference only castle lords are villains, and the town gives their contracts in turn. An expedition whose file
  has that rotation (below) follows it; one without, as the meadow, still lets a contract be taken by name, on a band or
  on a lord.
- A castle that falls back to monsters has the garrison its file first gave it. The reference rolls a fresh one by the
  region's tier, which belongs to a generated world. Status: `adopted_deviation`.
- Taking creatures back does not ask the hero's leadership, as the reference does not: a stack too large for it is out
  of control when it next fights.
- The original, by the reference's own account, looked only at a castle's first place when it decided whether the castle
  stood empty; the reference looks at all of them, and so does this.
- The reference gives a siege its random field from the castle's name; here an expedition gives each castle the state
  its siege starts from, as it does for a band.

How the castles stand, and whether the hero has siege weapons, is written into a save only where there are castles to
besiege. The vale has a keep of monsters and a lord's hold, and its clue is the lord's; the tests of the rules
themselves write a keep and a hall into the meadow.

### The town's contracts, in turn, and its word of a castle

Read from the reference's text, sections 16 and 21, and the sizes from its `game.json` (U-14). In an expedition whose
file gives the contracts a rotation:

- Contracts are on the lords of castles and on nobody else. A band is beaten for its gold.
- The town has so many contracts on offer at a time, five in the reference: the first of the list of villains to begin
  with. Asked for a contract, it gives the next of them in turn, round and round; the new contract takes the place of
  the one held.
- A villain caught gives his place to the next of the list who has not yet had one. When none is left his place stays
  empty, and when every place is empty the town has no contract to give.
- A town may have word of one castle: who is in it, each stack in a traveller's number, which is the largest of 1, 10,
  20, 50, 100 and 500 that the stack reaches. Of a castle that is the hero's it has nothing to tell.

Differences and what is left out:

- The reference's town has a command for "the next contract". Here the command names the villain, as it does without a
  rotation, and is refused unless it is his turn; a screen asks the rules whose turn it is.
- The reference's town tells the castle's owner, and marks a villain's castle as known once the hero has been at its
  gate; the contract then names the castle. Here a contract is named by its castle from the start, and the map shows
  every castle. What the player has seen and not seen comes with the world.

The vale's town gives its contracts in turn, on the lords of its keep and its hold, and has word of the hold.

### Bands that are gone round, and bands that would join

Read from the reference's text, section 15.4 (U-07).

- A hostile band is seen before it is fought. The reference shows it once the hero has stepped onto it and lets him step
  back; here he sees it from the cell beside it, each stack in a traveller's number, and chooses to step onto it or go
  round. Stepping onto it is the fight. Status: `adopted_deviation`; what is shown, exact numbers or a traveller's, the
  reference's text does not say, and the sizes are used as for a castle.
- A friendly band is one stack that offers to join. The hero steps onto it as onto any place and answers once. Taken in,
  it joins those of its troop in the army or takes a free slot, for nothing and whatever his leadership; with every slot
  held by another troop it goes its way, as it does when it is sent off. Either way it is gone.
- A friendly band is no villain: there is no contract on it, no reward and no clue.

Differences: the reference rolls afresh what a friendly band offers each time the hero comes; here an expedition's file
says who they are, and the week's creature grows in them as in any band.

The vale has wandering Gnomes who would join.

### Chests

Read from the reference's text, section 22 (U-05). A chest is opened once and holds one thing:

- Gold. The hero keeps it, or shares it out among his men: a point of leadership for every fifty gold, rounded down, for
  good. What the king's ranks give is added to it.
- A commission: so much more gold every week, for good. It cannot be shared out.

Opening a chest takes no time. Differences and what is left out:

- In the reference a chest opens under the hero's feet and asks at once; here he stands on it and chooses, or walks on
  and leaves it closed. Status: `adopted_deviation`.
- The reference decides what a chest holds by a roll over a hash of the world's seed and the chest's place, against a
  table by the region's tier: gold, a commission, spell power, a larger spell limit, a spell, or nothing. Here an
  expedition says what each chest holds, as the reference lets a map do for gold ("a declared purse"). The roll belongs
  to a world that is generated, which there is none of yet. The three outcomes about spells are under "The hero's
  magic".
- The amounts in the vale are within what the table's first tier gives: gold in hundreds from 100 to 500, a commission
  from 10 to 50.

Which chests are open, and whether a chest's gold was kept or shared out, is written into a save only where there are
chests. The leadership and commission they gave are worked out from that, not kept beside it.

### The hero's magic

Read from the reference's text, sections 8.3, 16, 18.5, 19.2 and 22.2 (U-13). This is what the hero has of magic and how
he comes by it; what the spells of battle do is below, and the spells of the map are not here yet.

- He can cast or he cannot. One class's first rank can; any other hero is taught at the alcove, once, for gold: the
  reference asks 5000, and a hero with exactly the price can pay it.
- His spell power and the room he has for spells are what every rank he has reached gives, added up, as with leadership.
- He holds charges of spells. Every charge of every spell takes one place of his room.
- A town sells one spell, a charge at a time, to a hero who has room for one more and more gold than the price: one with
  exactly the price is turned away (the reference's own strictness, kept). He need not be able to cast to buy; he must
  to cast.
- A chest may hold spell power (one point in the reference), room for more spells (one or two by the region's tier), or
  charges of a spell (one up to the tier's number). The charges are his whatever room he has, and count against it
  afterwards. None of the three can be shared out.

The spells and what a town asks for a charge, under the allowance for the economy's tables:

| Spell | Cast | Price |
|---|---|---|
| Clone | in battle | 2000 |
| Teleport | in battle | 500 |
| Fireball | in battle | 1500 |
| Lightning | in battle | 500 |
| Freeze | in battle | 300 |
| Resurrect | in battle | 5000 |
| Turn Undead | in battle | 2000 |
| Bridge | on the map | 100 |
| Time Stop | on the map | 200 |
| Find Villain | on the map | 1000 |
| Castle Gate | on the map | 1000 |
| Town Gate | on the map | 500 |
| Instant Army | on the map | 1000 |
| Raise Control | on the map | 500 |

Differences and what is left out:

- In the reference the alcove asks as the hero steps onto it; here he stands on it and chooses, as with a chest. Status:
  `adopted_deviation`.
- Every zone of the reference's world has an alcove and its own price where it names one; an expedition here has at most
  one alcove, with the price in its file.
- A chest gives at most 99 of what it holds of magic. The reference has no such bound; it keeps the hero's numbers from
  running over. Status: `adopted_deviation`.
- The artefacts that double spell power and room, and the packs of the reference that teach magic zone by zone, hold
  each spell apart or renew spells weekly, are not here.
- An expedition with an alcove, a spell for sale or a chest of magic needs a king: without ranks nothing says how many
  spells a hero can hold.

Whether the hero casts and the charges he holds are written into a save only where the expedition has magic: an alcove,
a spell for sale, a chest of magic, or a class that casts from the start. Spell power and room are worked out, not kept.

#### The spells of battle

Read from the reference's text, section 25.8, and its code (`combat.c`). A hero who can cast takes his charges of the
seven spells of battle into a battle, with his spell power, at least 1. He casts once a round, during the turn of one of
his stacks, and the cast does not use that turn up. A charge spent is spent however the battle ends.

| Spell | Cast on | What it does |
|---|---|---|
| Fireball | an enemy stack | 25 damage for a point of spell power |
| Lightning | an enemy stack | 10 damage for a point of spell power |
| Turn Undead | an enemy stack of the undead | 50 damage for a point of spell power |
| Freeze | an enemy stack | the stack stands out its turn; a new round thaws it |
| Clone | a stack of the hero's | 10 health for a point of spell power, with the stack's injury, makes whole creatures of its troop; what is left over is the stack's injury afterwards |
| Resurrect | a stack of the hero's that has lost creatures | a creature for a point of spell power comes back, up to the number the stack began with, and the stack is unhurt |
| Teleport | any stack, and a free cell | the stack stands on that cell |

A spell's damage is not rolled and nothing scales it: no skill, no morale. Nobody strikes back at it. The three spells
of damage and Freeze do nothing to a troop that is immune.

Differences and what is left out:

- In the reference a spell that harms, cast on an immune troop, says that it has no effect and costs neither the charge
  nor the round's cast. Here such a cast is refused and not offered, which leaves the same state. Status:
  `adopted_deviation`.
- Resurrect on a stack that has lost nothing costs nothing in the reference either; here it is refused.
- Clone works on the injury as the reference's code does: a stack whose injury and ten health a point do not make a
  whole creature is left with more injury than it had. The reference's own code; not checked against DOS.
- Clones raise the number the stack began with, so Resurrect can bring them back, and a troop that leeches can regain up
  to it. They stay with the army after a battle won, as creatures a stack grew by do.
- A stack that Clone makes too large for the hero's leadership is not out of control for it, as a stack that grew is
  not: control is lost only at the start of a battle. Status: `adopted_deviation`.
- A frozen stack still strikes back. The reference's freezing is read only where a stack's turn begins.
- Only the player's hero casts; no enemy has a hero.

A battle keeps the spell power, the number each of the hero's stacks began with, and which stacks are frozen only where
the hero casts from his book, so every other battle is written, and digested, as before.

### Rules, and how they differ from the prototype

| ID | Rule as the reference has it | Prototype | Doubt |
|---|---|---|---|
| U-08 | The player's stacks act in slot order, then the enemy's. There is no speed order. | The same order. | None known. |
| U-08 | Flags and allowances (struck back, movement, flights) are renewed for every stack before each side's turn, so a stack can strike back once in its own side's turn and once in the other's. | Renewed once a round. | The reference's own code; not checked against DOS. |
| U-09 | A step goes to any of the eight neighbours; a diagonal step is not stopped by what stands beside it. Stepping onto an enemy is the melee attack. | Diagonal needs both cells beside it free; attack is a separate action. | None known. |
| U-09 | A flyer has two flights in its turn. A flight costs no movement; the second ends the turn. | One flight, costing a movement point. | None known. |
| U-10 | Damage: roll x creatures x (attacker's skill + 5 - target's skill) / 10, each step in whole numbers, rounding down. | Roll x creatures. | The reference calls its routine a transcription of the original's. |
| U-10 | The count used is the stack's count before the exchange, for the answer as well: creatures killed by a blow still strike back. | The answer uses the survivors. | As above. |
| U-10 | Morale then scales the damage of a stack that is on the hero's side and under control: Low halves, High adds half. The enemy has no hero, so no morale. | No morale. | See below. |
| U-11 | One answer per stack between renewals; none to a shot. Injury carries over inside a battle, as a health pool. | The same pool; one answer a round. | None known. |
| U-12 | A stack cannot shoot with an enemy in one of the eight cells around it. A shot uses one of the troop's shots whatever it does. | The same. | None known. |
| U-12 | Regeneration (Trolls): when a round begins, a stack that regenerates has no injury, on either side. Not when the enemy's part of the round begins. | No abilities. | None known. |
| U-12 | Absorb (Ghosts): after a blow of the stack, struck back included, the creatures the blow killed join it. There is no limit, so the stack can outgrow the hero's leadership and go out of control. | No abilities. | Where the blow kills the whole target the reference counts the target's number at the start of the turn, not the number the blow killed; here it is what the blow killed, as for U-10. |
| U-12 | Leech (Vampires): after a blow of the stack, one creature returns for every whole health of its own in what the blow took, which is counted as the full health of the creatures killed plus the injury now on the target. Above the number the stack began the battle with, it has that number and no injury; at or below it, the injury stays. | No abilities. | That the target's earlier injury counts as taken by this blow reads like an accident of the reference's sum; kept, as it is what the reference does. The same doubt about a target killed whole as for Absorb. |
| U-12 | Undead (Skeletons, Zombies, Ghosts, Vampires): a mark that one spell reads. Nothing else in a battle does; morale treats the undead like any troop. | No abilities. | None known. |
| U-12 | Scythe (Demons): before every blow of the stack, struck back included, a number from 1 to 100 is rolled; above 89 the scythe takes half the target as it stands, rounded up, at full health each, on top of the blow. It is rolled for before the blow's damage and added after morale has scaled the blow. | No abilities. | The reference's text calls it 11 in 100, which the roll gives; nothing says what the original rolls. |
| U-12 | Magic shot (Druids, Archmages): the shot's number is fixed and nothing is rolled; skill and morale scale it like any shot. Against an immune troop it does nothing at all, and the shot is spent. | No abilities. | None known. |
| U-12 | Immune (Dragons): magic shots and the spells that harm do nothing to the troop. Arrows and blows do. | No abilities. | None known. |
| U-06 | A stack is out of control when health of one x count is more than the hero's leadership (equal is still in control). It keeps its turns and counts as an enemy of its own side, until losses bring it back under; checked before every stack acts. | No leadership in battle. | Who moves it during the player's turn is the reference's choice (its computer player). |
| | Random numbers: a 64-bit generator, state x 25214903917 + 11, the top 32 bits taken modulo the size of the range; a range of one number draws nothing. | A 32-bit generator with rejection. | The modulo favours low numbers slightly; kept as the reference has it. |

### Adopted where the reference is silent or impractical

- **Waiting.** In the reference a stack that waits is passed over and comes up again after the other stacks of its side;
  nothing there says what a second wait does. Here, as in the prototype, a stack may wait once before it has moved, acts
  after the others of its side, and a second wait ends its turn. Status: `adopted_deviation`.
- **Seeding.** The reference derives a battle's first random state from the world's seed and the encounter's identity.
  There is no world yet; a battle here starts from the state the scenario or the suite gives it.

### Implemented so far

Everything in the table above: turn order by sides with the renewal of flags, the step rule, flight, shots, damage
scaled by skill with the count from before the exchange, morale by the ported rule, loss and return of control, and the
reference generator; and every ability of a troop.

A stack that grew by absorbing keeps its number after the battle: the army has what the battle left. What a leech began
with is part of a saved battle, for the troops that leech and no others, so that every other battle is encoded as it
was.

How loss of control is carried out here:

- A scenario gives the leadership of the player's hero. Without it the player's side has no hero in that battle: no
  morale and no stack out of control. The enemy never has a hero.
- A stack out of control and any other stack fight each other, whatever their sides: it may be struck and shot by its
  own side, it may strike and shoot its own side, and a stack of its side standing beside it stops a shooter as an enemy
  would. Its creatures still count for its side when the battle is decided.
- It has no morale.
- Control comes back, and only comes back: a stack that starts within the leadership stays in control, as in the
  reference, which sets the mark when the battle is set up and only ever clears it.
- The rules do not say who commands it. The reference hands its turn to its computer player; here the pilot plays it, in
  the runs and on screen, where the stack is marked and the sidebar says that it moves by itself.

### The two morale rules

The reference documents two ways to find a stack's morale and says which content uses which:

- **Ported**, for the King's Bounty content, described there as the original's behaviour "including the documented bug":
  every living stack of the side, the stack itself included, is looked up in the chart as (other, self); the ranks are
  ordered Normal, Low, High, and the lowest found wins. So a mixed army falls to Normal as soon as one pairing is
  Normal, and reaches Low only when no pairing is Normal and one is Low.
- **Army view**, for its other content: a stack alone is High; every other stack is looked up as (self, other); any Low
  gives Low, all High gives High, else Normal. This is what the army screen shows the player.

`reference-v0.1` uses the ported rule. The two disagree, for instance, on an army of Sprites and Wolves: ported makes
the Sprites Low and the Wolves Normal; army view makes the Wolves Low and the Sprites Normal. That army is the morale
case of the reference suite. Which of the two the DOS game really does is open (U-18) and worth one experiment, since it
decides what the player is told against what the dice do.

### Not in this subset

Refused on load, not approximated: artefact powers, siege layout, the random field obstacles of an encounter, spoils.
Each comes with the first troop or encounter that needs it.

## A variant, not a source: `modern-v0.1`

`content/modern/rules.json` holds the reference rules with four switches set otherwise, to try the rule changes proposed
in `10-GAMEPLAY-GAPS.md`. Nothing in it is evidence about the original, and nothing it changes is ever written back into
`reference-v0.1`. The loader lets only this ruleset set a switch; the reference file with a variant's value is refused.
The variant plays the reference's battles and keeps its own baselines.
