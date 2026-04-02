# abuse-rs

`abuse-rs` is a Rust + Bevy compatibility-first port of the classic side-scroller **Abuse**.

The immediate goal is to load and understand original data formats (`.spe`, levels,
scripts) and then drive a modern runtime from that data with behavior parity.

## Project Status

This repository is currently in bootstrap phase:

- Workspace and crate layout are in place.
- Implementation details are tracked in `PLAN.md`.
- Legacy source reference is expected at:
  `/home/admin/Downloads/abuse-0.8`

## Repository Layout

```
.
|- Cargo.toml            # Workspace manifest
|- PLAN.md               # Milestones and compatibility plan
|- crates/
|  |- data/              # Legacy format readers (.spe, levels, scripts)
|  |- sim/               # Deterministic gameplay simulation core
|  |- runtime/           # Bevy integration (render, input, audio)
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

## Non-Goals (for now)

- Full editor parity
- Network multiplayer
- Perfect script runtime parity on day one

## Legal Note

This repository is for engine/runtime code. Original game assets and data files are not
included.
