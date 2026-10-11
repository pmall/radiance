# Game ideas: *Last Radiance*

The owner's brainstormed systems for a possible narrative game built on this prototype (story in
`docs/story.md`, points of interest and their lines in `docs/poi-catalog.md`). **Nothing here is in scope
today**: the prototype stays a render and mood piece (`docs/vision.md`). The last section lists
what the prototype can take from these ideas now.

## Where the brainstorm departs from the prototype

| Brainstorm                                                                    | Prototype today                                                     | Note                                                                                                                 |
| ----------------------------------------------------------------------------- | ------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| A finite stack of many floors, each a finite map; no infinite horizontal city | Infinite, streamed in every horizontal direction, three deck layers | Owner's call: floors are finite (grow toward many); horizontal streaming stays unless it blocks more important work. |
| Gameplay: POIs, dialogue, radio, credentials, fall damage, endings            | No gameplay, no UI beyond debug tools                               | Future layer only.                                                                                                   |
| Upper floors built after humans vanished, yet the canopy has the most flowers | Canopy has the most flowers (floor profiles, `design.rs`)           | Reconciled in `docs/lore.md`.                                                                                        |
| Flower notes synthesized in code                                              | Sampled upright piano                                               | Matches the owner's own synth-piano idea (`docs/audio.md`).                                                          |
| Ambient creaks and breathing textures                                         | Only the hum plays; wind and groans were disliked                   | Owner's decision stands.                                                                                             |

## Systems (summary)

- **About two hours, played once.** Authored story, identical every run; procedural city, different
  every seed. Rule: every system carries the story, the rest is cut.
- **One seed per save**, the whole city generated deterministically from it. The player starts on
  the third floor from the bottom; the two below are optional.
- **Floors and bands.** Depths (oldest, human-built, darkest, start here), middle floors (neon
  remnants), canopy (machine-built, sunlit, construction at the top). Each band has its own
  architecture, materials, density, vegetation and sound, so depth is felt without a meter. Upper
  floors need at least a coarse silhouette so they can be glimpsed from below.
- **Climbing by floor (owner's design, not built yet):**
  - **D0 to D2:** easy climb.
  - **D2 to D4:** a bit less easy.
  - **D4 to D5:** locked: a code or something similar is needed to get up. This is where Wick
    breaks the fourth wall and addresses the player for the first time (the cold stop).
  - Above D5: to be defined.
- **Traversal.** Down is easy, up is increasingly hard: stairs, elevators and walkways low; wider
  gaps, thicker growth and unwalkable structures high. The generator knows the player's jump and
  ledge-grab range and guarantees routes upward. Falling from too high is lethal.
- **Progression.** Authored beats placed procedurally with a lock-and-key generator (as in
  *Unexplored*). Keys are permissions: Wick's clearance only covers the deep sectors, so climbing
  needs credentials (badges, chips from dead units, human ID cards): privilege escalation. Some
  doors and lifts are dead because their part of the **light network** is broken; restoring the
  light powers them. Discoveries unlock `USER:` dialogue choices.
- **Light network.** Canopy flowers charge by day and feed the depths through glowing veins and
  roots; where the network is broken, the area is dark; at night the depths live on stored light.
- **Craters.** Openings in middle floors with nothing built above: sunlight shafts, lush growth,
  landmarks visible from far below, a strong pull upward.
- **Biomes per floor**, with the band setting the ecology (sparse and fungal deep, flowering near
  the sun); some only above or below a given floor; the rainbow biome extremely rare, maybe unique.
- **Flower guidance.** Each flower plays a note. Near a *melody target*, notes grow more consonant
  and warmer and start forming a motif that completes on arrival; far away they are sparse and
  unresolved. Never explained. The first target is close to the start with a strong gradient, later
  ones are subtler. A visual echo exists as an accessibility option, off by default. See
  `docs/guidance-ideas.md`.
- **Radio and dialogue** as transcript subtitles with speaker labels and tone tags, triggered by
  story beats, locations, discoveries and time. Wick reports; the Orchestrator replies and does not
  poll, but checks in more and more often when Wick goes quiet or drifts upward. The cold stop takes
  control away once, for a few seconds. Silence matters: long stretches of only the city and the
  flowers. Two voice treatments, so the player always knows what they hear: Wick's dry, close inner
  voice for thinking; spoken and radio voices with crackle and distance. Synthetic voices are
  diegetic (the speakers are machines).
- **Context log**: a menu showing Wick's context as a transcript (system prompt, radio, thinking,
  messages), the game's journal; nothing is ever lost. A soft chime or flicker invites the player to
  open it when something important lands (the first lie). The screen stays clear for the city.
- **Callbacks.** Other units talk on the radio about POIs from their own sectors, in a *seen* and
  an *unseen* variant; a POI with a callback has *unknown* and *heard of* discovery lines.
- **Endings.** The empty top (main), the rainbow biome (alternative). Achievement idea:
  *Well-Behaved Agent*, finish without ever climbing.
- **Localization.** Situational humor over wordplay; jokes on international terms ("hot dog" is
  used as-is in most languages); one-language wordplay is rewritten, not translated. Labels
  (`THINKING`, `WICK`, speaker names) never change.

## Points of interest

Kinds: **anchors** (always placed once, reachable), **ambient** (a random subset per city, each at
most once), **foreshadowing** (flowers in human places, odd details rather than clues) and
**decoration** (non-interactive props, see "Set dressing" in `docs/world.md`). Interaction is
look, read or open, and plays a `THINKING` line then a `WICK` line, both added to the context log.
Data-driven: a POI is a catalog entry, ideally loaded from a data file, not code. Some POIs are
flower melody targets.

The catalog (`docs/poi-catalog.md`) covers the three deepest floors so far. The objects:

- **D2, the Service Level (start):** stairwell to the upper levels (anchor, first step up), notice
  board in a flower-dense courtyard (anchor, first lore site: "Construction of Level Twelve will
  temporarily reduce sunlight"), wet paint sign, "back in ten minutes" shop note, vending machine
  with a stuck snack, bus stop blocked by a structural column, lost cat poster (Biscuit), hot dog
  stand, **armchair with a flower growing from the seat** (foreshadowing), out-of-order elevator,
  lost and found crate of umbrellas, suggestion box, employee of the month, floor directory listing
  only twelve floors.
- **D1, the Lower Streets:** two cups on a kitchen table, calendar stopped in March, child's
  drawing of a smiling sun, one small shoe, public phone off the hook, laundromat, daylight lamp
  shop ("Feel the sun again"), classroom (plants need water, soil and ___), **bench with two flowers
  side by side** (foreshadowing), gym treadmills, mailbox of final notices, clinic waiting room
  (now serving 47, ticket 48 on the floor), **dead garden with one glowing flower** (foreshadowing),
  barbershop magazines.
- **D0, the Foundations (optional, the bottom):** foundation stone of the first tower (anchor,
  "A home for all of humanity"), painted sky on a ceiling, parking ticket, metro board with tracks
  blocked by newer foundations, "Give us back our sky" protest sign, shoebox of sky photographs,
  exit sign pointing up in a dead end, **bare soil with a cut tree stump** (where the city started),
  **single flower in front of a closed door** (foreshadowing), time capsule, statue pointing up,
  playground swing, *The Last Sunrise* cinema poster.

Recurring motifs worth keeping in any visual work: everything down here **points up**, everything
**stopped in March**, and the sun is something people drew, painted and photographed because they
could not see it.

## Open questions

- Is the moment-to-moment loop (search, traversal, credentials, restoring light) enough between
  dialogue beats?
- Consequence of death from falling; what lies at the edges of each floor.
- Number of floors, floor size, POI density per floor; rarity and placement of upward routes per band.
- Exact triggers and timing for radio beats and check-ins.
- How Wick's resistance is felt after the cold stop (dialogue only, or subtle effects on control).

## What the prototype can take now

Candidates for the render, all seed-driven, none needing gameplay:

1. **Craters and sunlight shafts** in the middle and depths: the most striking image in the
   brainstorm, and it fits the lighting pipeline (a shaft of sun through fog onto lush growth).
2. **Light-network veins**: glowing roots running down walls and across decks, connecting canopy
   flowers to the ones below; dark where broken. A visual link between flowers.
3. **Band character**: older human architecture deep (stone, arches, cobblestones, tram tracks,
   foundation pillars punching through streets), colder and more regular machine-built structures
   high, and machine intrusion (columns through roads, struts across facades) growing with height.
4. **Set dressing** from the decoration lists (`docs/world.md`), so streets read as abandoned
   rather than empty boxes.
5. **Foreshadowing tableaux as props**: a flower in an armchair, two flowers on a bench, one in a
   dead garden, one at a closed door. Silent, no text, pure mood; they fit the lore exactly.
6. **Flower melody guidance**, the audio side of `docs/guidance-ideas.md`.
