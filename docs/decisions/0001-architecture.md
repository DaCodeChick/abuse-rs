# 0001 - Compatibility-first architecture

## Status

Accepted

## Context

`abuse-rs` targets behavioral and content compatibility with legacy Abuse data and
code flow. The legacy engine uses custom container formats (`.spe`), map/object
structures, and script-driven behavior.

## Decision

Use a layered workspace with dedicated crates:

- `data` for format parsing and legacy data modeling.
- `sim` for deterministic game-state updates.
- `runtime` for Bevy-facing systems and rendering.
- `game` as executable entrypoint.
- `tools` for data validation and inspection.

## Consequences

- Enables isolated testing of binary parser correctness.
- Allows simulation to evolve independently from renderer concerns.
- Adds up-front structure overhead but reduces future rewrite risk.
