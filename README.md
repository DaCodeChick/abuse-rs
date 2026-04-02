# abuse-rs

`abuse-rs` is a Rust + Bevy compatibility-first port of the classic side-scroller **Abuse**.

The immediate goal is to load and understand original data formats (`.spe`, levels,
scripts) and then drive a modern runtime from that data with behavior parity.

## Project Status

This repository is currently in bootstrap phase:

- Workspace and crate layout are in place.
- Implementation details are tracked in `PLAN.md`.
- Legacy source files are expected to be available locally.

## Repository Layout

```
.
|- Cargo.toml            # Workspace manifest
|- PLAN.md               # Milestones and compatibility plan
|- crates/
|  |- runtime/           # Bevy integration + simulation + data readers
|  |- game/              # Main executable
|  `- tools/             # Data inspection and conversion tools
`- docs/
   `- decisions/         # Architecture and format notes
```

## Runtime Viewer Modules

Viewer implementation is split into runtime modules under `crates/runtime/src/viewer`:

- `assets.rs`: legacy SPE palette/image decoding and texture library loading
- `object_render.rs`: object type/state/frame -> sprite mapping + placement offsets
- `audio.rs`: object-driven one-shot SFX state and control systems
- `camera.rs`: viewport/pan/zoom control
- `hud.rs`: debug overlay and information display
- `scene.rs`: level scene setup and world state initialization

This keeps `crates/game/src/main.rs` focused on app composition and scene wiring.

## Getting Started

```bash
cargo run -p abuse-game
```

Current executable is a bootstrap app; gameplay systems and data loading are staged in
the plan.

Level viewer mode:

```bash
cargo run -p abuse-game -- /path/to/levels/level00.spe
```

This renders a debug view of FG/BG tiles plus object/light markers from parsed legacy
level data.

When legacy tile archives are found under `data/art`, the viewer uses decoded Abuse tile
textures. Otherwise it falls back to debug-colored tiles.

Viewer controls:

- `WASD` or arrow keys: pan
- Mouse wheel: zoom
- `Q` / `E`: zoom out / in
- `F1`: toggle debug HUD
- `M`: mute/unmute object SFX
- `-` / `+`: lower/raise object SFX volume

The viewer also plays contextual one-shot legacy SFX when relevant object types are present
(teleporters, spring, lava, force-field/electric hazards).

Current object rendering includes a mapped subset (eg `TP_DOOR`, `SWITCH_DOOR`,
`TRAP_DOOR2/3`, `TELE2`, `FORCE_FIELD`, `LIGHTIN`, `STEP`, basic `SWITCH*`) based on
level object type/state names and known sprite archives.

Additional mappings now include key level objects like `NEXT_LEVEL`, `SPRING`, `LAVA`,
powerups (`HEALTH`, `POWER_FAST`, etc.), and `WHO`.

Lisp bootstrap tooling is available:

```bash
cargo run -p abuse-tools -- lisp-loads /path/to/abuse.lsp
```

This parses a Lisp file and lists the discovered `(load "...")` dependencies.

SPE file inspection:

```bash
cargo run -p abuse-tools -- spe-list /path/to/file.spe
```

This lists all entries in an SPE archive with type, flags, size, offset, and name.

Level summary:

```bash
cargo run -p abuse-tools -- level-summary /path/to/levels/level00.spe
```

This displays a human-readable summary of level dimensions, object counts, lights, and links.

Level dump for validation:

```bash
cargo run -p abuse-tools -- level-dump /path/to/levels/level00.spe --format json
cargo run -p abuse-tools -- level-dump /path/to/levels/level00.spe --format ron
```

This outputs a structured, machine-comparable dump of the level data in JSON or RON format
(default is JSON). Useful for validation against legacy level parsing.

## Testing

Run all tests:

```bash
cargo test --workspace
```

Level parsing baseline validation tests are available when `ABUSE_LEGACY_ROOT` is set:

```bash
ABUSE_LEGACY_ROOT=/path/to/abuse-0.8 cargo test -p abuse-runtime --test level_baselines
```

These tests validate the level parser against 49 baseline dumps of real legacy levels
(22 main campaign + 27 Frabs addon levels). See `tests/baselines/README.md` for details.

## Non-Goals (for now)

- Full editor parity
- Network multiplayer
- Perfect script runtime parity on day one

## Legal Note

This repository is for engine/runtime code. Original game assets and data files are not
included.
