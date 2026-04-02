# abuse-rs Plan

This project follows a compatibility-first strategy against the legacy Abuse source
and data formats.

## Guiding Principles

- Preserve original data semantics before redesigning systems.
- Keep parsers testable and independent from rendering/runtime.
- Favor deterministic simulation behavior.
- Introduce strict and lenient loading modes for legacy edge cases.

## Milestones

### M0 - Workspace Bootstrap

- [x] Create Cargo workspace and crate boundaries.
- [x] Add initial Bevy app bootstrap and runtime plugin group.
- [x] Add plan and architecture docs.

### M1 - SPEC Container Compatibility

- [ ] Implement `.spe` directory parser (`SPEC1.0`).
- [ ] Implement entry name/type lookup and raw payload reads.
- [ ] Add fixture-driven tests against known legacy files.
- [ ] Build CLI inspector in `crates/tools`.

Acceptance:

- Inspecting a `.spe` file lists entry count, names, types, size, offset.
- Parser handles name termination quirks and malformed data gracefully.

### M2 - Level Compatibility

- [ ] Decode `fgmap` and `bgmap` sections.
- [ ] Decode level options, object list, links, and lights metadata.
- [ ] Emit debug dump format (JSON or ron) for comparison.

Acceptance:

- `levels/level00.spe` loads with expected dimensions and non-empty object set.

### M3 - Runtime Visual Baseline

- [ ] Render loaded tile layers in Bevy.
- [ ] Add camera framing and map extents.
- [ ] Show placeholder entities for decoded objects.

Acceptance:

- Can launch executable and view loaded level geometry.

### M4 - Script Bootstrap Compatibility

- [x] Add initial Lisp parser and startup load extraction.
- [ ] Load startup script assets and defaults.
- [ ] Implement minimal builtin subset required for boot flow.
- [ ] Provide diagnostics for missing symbols/functions.

Acceptance:

- Game reaches playable loop with startup data loaded.

### M5 - Behavior Slice Parity

- [ ] Player movement + aiming + one weapon.
- [ ] One enemy behavior path.
- [ ] Tile/entity collision + damage loop.

Acceptance:

- One complete combat slice playable from legacy data.

## Initial Crate Responsibilities

- `crates/data`: data formats (`.spe`, level sections, script source loading).
- `crates/runtime`: Bevy plugins for rendering/input/audio plus simulation state and rule execution.
- `crates/game`: executable wiring and configuration.
- `crates/tools`: inspection and conversion tools.

## References

- Legacy source root: local path provided at runtime
- Key files:
  - `src/imlib/specs.h`
  - `src/imlib/specs.cpp`
  - `src/level.cpp`
  - `src/game.cpp`
