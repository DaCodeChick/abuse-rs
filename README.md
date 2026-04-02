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

Viewer controls:

- `WASD` or arrow keys: pan
- Mouse wheel: zoom
- `Q` / `E`: zoom out / in
- `F1`: toggle debug HUD

Lisp bootstrap tooling is available:

```bash
cargo run -p abuse-tools -- lisp-loads /path/to/abuse.lsp
```

This parses a Lisp file and lists the discovered `(load "...")` dependencies.

## Non-Goals (for now)

- Full editor parity
- Network multiplayer
- Perfect script runtime parity on day one

## Legal Note

This repository is for engine/runtime code. Original game assets and data files are not
included.
