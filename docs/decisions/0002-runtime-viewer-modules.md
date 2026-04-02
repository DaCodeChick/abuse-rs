# 0002 - Runtime viewer module split

## Status

Accepted

## Context

The level viewer in `crates/game/src/main.rs` had grown into a large mixed-responsibility
module (startup wiring, legacy asset decoding, object render mapping, audio behavior,
camera, and HUD). This violated the optimization policy goals around simplification,
separation of concerns, and splitting large modules.

## Decision

Move viewer subsystems into `abuse-runtime` under `crates/runtime/src/viewer`:

- `constants.rs`: static viewer constants and archive lists
- `assets.rs`: legacy palette/image decoding and sprite/tile library loading
- `object_render.rs`: type/state/frame to sprite mapping and placement adjustments
- `audio.rs`: object-driven one-shot SFX state and systems

`crates/game/src/main.rs` remains the application composition entrypoint and uses these
runtime modules via imports.

Also replace custom enum conversion helper with standard trait usage:

- `SpecType::from_u8` -> `impl TryFrom<u8> for SpecType`

## Consequences

- Better separation between app composition and viewer implementation.
- Smaller, more focused modules and easier policy-compliant iteration.
- Reusable runtime viewer building blocks for future tooling and tests.
- Slightly larger public runtime surface area that should be kept curated.
