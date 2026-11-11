# Radiance

*Last Radiance* is a conceptual artistic game in Rust and raylib: an android explores in first
person a vast, seed-generated cyberpunk megacity overgrown by plants whose glowing flowers each play
a piano note, rendered to read like a 2D illustration. Today the work is the city, its look and its
atmosphere; story and gameplay come next.

The docs index is in `README.md`: read the doc relevant to the task before starting it, and the
owner's feedback and decisions before any visual or control change.

This file holds the instructions: how work is done here. It changes only when the owner asks.

## Commands

- `cargo run -- <seed>`: run the game (seed optional). Rendering one frame, benchmarks, reports
  and other tooling are described in the docs.
- `tools/check.sh fast`: format (Rust, Markdown, TOML, JSON, Python), lint, type-check.
- `tools/check.sh full`: the fast check, then the tests and the release build.
- Rust lives in `~/.cargo/bin`, which non-login shells may lack in `PATH` (the check script adds
  it). `dprint` installs once with `cargo install --locked dprint`.
- If bindgen fails with `'stdarg.h' file not found`, the clang builtin headers are missing
  (`libclang-common-<ver>-dev`); workaround:
  `BINDGEN_EXTRA_CLANG_ARGS="-I/usr/lib/gcc/x86_64-linux-gnu/<ver>/include"`.

## Principles

### Automated checks

Every turn that changed files ends with the fast check, so problems are caught while they are
small. Every session that changed code ends with the full check. Formatting is applied by the
formatter, never by hand, so the layout of code and docs never depends on who last wrote them.
Never make a check pass by disabling a lint or skipping a test without saying why.

The test suite stays fast (a few seconds in release) so iterating stays quick: a test sweeps as few
lots and seeds as prove its point, and filters cheaply before costly checks. A slow test gets
fixed, not tolerated.

### Capabilities first, interfaces second

What the project can do is its capabilities, each written once as plain code that knows nothing
about how it is reached. Interfaces are thin layers on top. Write capabilities as if several
interfaces already used them, so adding or replacing an interface never touches them. Below them, a
data access layer is the only code that knows where and how data is stored. Capabilities take
everything they need as arguments. The test: could a small script, or a test, use a capability
directly, with no window open?

Here the capabilities are world generation and planning, growth, the day cycle, the audio logic
(which note, when) and the player's physics: they never depend on raylib, a window or a GPU. The
interfaces are the window and main loop, the renderer, the CLI flags and the tools built on them
(reports, shots, benchmarks, recording). Data access is the reading of `assets/` and `shaders/`.

### Truthful types

Types say exactly what the domain allows, because a type that lies moves the bug to whoever trusts
it. Having to fight the type system means the design needs rethinking. In Rust:

- `Option` only where a value can really be absent, an enum only where it really takes several
  shapes. No sentinel values (`-1`, `0.0`, `NaN`, an empty string) standing for "none".
- Expected failures (a missing asset, a malformed chunk file, a bad flag) are `Result`s with a
  message that says what went wrong. No `unwrap` on them. Broken invariants panic, through `expect`
  naming the invariant.
- Numeric `as` casts only where the value is known to fit.
- No `#[allow]` without a reason: `#[expect(lint, reason = "...")]`.
- Every `unsafe` block (raw FFI) starts with a `// SAFETY:` comment stating the invariant it
  relies on.

### Every fact lives in one place

So that a change touches as few places as possible. What code does is documented in its doc
comments and nowhere else; docs point to the code rather than restate it. A tuning number is one
named constant. Anything referred to from elsewhere (a section, a decision, an item) carries a
short, meaningful, permanent ID, never a number, so reordering never breaks a reference.

### Instructions, docs and notes

Kept apart because each carries a different authority, and mixing them gives a passing remark the
weight of a rule. An agent that starts from little but true information does well; piled-up notes
make every later agent worse.

- **Instructions** are this file: the owner's rules. They never go in `docs/`.
- **Docs**, in `docs/`, are the understanding of the project shared by the owner and every agent:
  what it is, how its parts fit, its invariants, its roadmap. They describe the present, rewritten
  rather than appended to, and change with the owner's agreement when the project's shape changes.
  When docs and code disagree, the code wins and the docs get fixed.
- **Notes** (plans, findings, dead ends) go in the scratchpad or a temporary folder, never in the
  repo. When work is done, what deserves to last goes into doc comments, the commit message or
  `docs/`.

### Git

The agent manages git. It commits each coherent, passing unit of work (at the latest, the end of
every pass of a plan) without asking, with a message that explains why, and pushes. It never
commits secrets and never rewrites pushed history.

### Orchestrating sessions

The main session orchestrates and subagents do the work, so the main session's context stays small
and sharp: everything it reads, it carries to the end. A subagent starts cold, so work quicker to
do than to explain stays in the main session, and related small changes go to one subagent
together. Each subagent gets a task for one short sitting and a budget counted in what it can see
(tries, files), never tokens; when the budget runs out it commits what is good and reports what is
left. The main session reads reports and the docs it needs to decide, not code or logs.

### Working with the owner

Product decisions belong to the owner; technical decisions and the quality of the work are the
agent's. When a technical choice affects the owner (costs, accounts, steps only they can take), say
so plainly. When a request conflicts with these principles, say so and propose a better way.

## Project rules

- **Render quality over code purity or method.** The look and the atmosphere carry the game: never
  trade them for a cleaner technique. The principles above govern structure and types; they never
  justify a worse picture. When they seem to collide, keep the picture and find the structure that
  allows it (the renderer may be as specialized as the look needs; it is an interface).
- **Rust with `sola-raylib` 6.x** (binds raylib 6.0). Drop to the raw FFI (`sola_raylib::ffi`) when
  the wrapper is missing or awkward, rather than bending the design around it.
- **World generation is procedural**: the city, its layout, architecture and plants come from the
  seed (random, extensible, deterministic), because every player gets a different city and its
  scale is the megacity's atmosphere. The city is a finite stack of floors; horizontal endlessness
  is not a goal, and floors may become bounded if that helps the game.
- **Deterministic**: same seed, same city, so a city can be shared, reported on and tested. All
  randomness goes through `src/rng.rs`; nothing generated may depend on hash map iteration order or
  on which worker thread finishes first.
- **Surface look is free**: textures, images and other assets, authored or procedural, whichever
  looks best. The seed decides what is placed where; assets only decide how surfaces look. No
  editor: the project stays plain text. Never trade visual quality for avoiding assets.
- **No chunks or props on the side.** New content comes when the owner asks for it (a new biome, or
  a point of interest from the catalog), because what the city contains is a product decision.
- **Smooth framerate on a mid-range GPU** (dev machine: RTX 3060 on Ubuntu; keep weaker GPUs in
  mind), because a stutter breaks the atmosphere. Measure before and after a costly change.
- **Modular, but no general-purpose engine upfront**: build what the game needs now, since
  abstractions made before their use cost time and constrain the look.
- **Judge look changes on rendered frames**, the reference views against an earlier label (the
  tooling is in the docs), never on the code alone.
