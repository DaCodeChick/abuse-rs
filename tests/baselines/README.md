# Level Parsing Baselines

This directory contains baseline dumps of legacy Abuse level files for validation
and regression testing of the level parser.

## Purpose

The baseline dumps serve as ground truth for verifying that the level parser
correctly reads and interprets all fields from the original Abuse level format.
Any changes to the parser should be validated against these baselines to ensure
backward compatibility.

## Structure

```
baselines/
└── levels/
    ├── level00.json   # Main campaign levels (level00-level21)
    ├── level01.json
    ├── ...
    ├── frabs00.json   # Frabs addon levels
    ├── frabs01.json
    └── ...
```

## Running Baseline Tests

Baseline validation tests are located in `crates/runtime/tests/level_baselines.rs`.

To run the tests, set the `ABUSE_LEGACY_ROOT` environment variable to point to
your local copy of the legacy Abuse source distribution:

```bash
ABUSE_LEGACY_ROOT=/path/to/abuse-0.8 cargo test -p abuse-runtime --test level_baselines
```

If `ABUSE_LEGACY_ROOT` is not set, the tests will be skipped with a message.

## Regenerating Baselines

Baselines should only be regenerated when:

1. The level format understanding is improved (e.g., new fields discovered)
2. Parser bugs are fixed that change output
3. The dump format itself is intentionally changed

To regenerate all baselines:

```bash
cd /path/to/abuse-rs

# Regenerate main campaign levels
for level in /path/to/abuse-0.8/data/levels/level*.spe; do
  basename=$(basename "$level" .spe)
  echo "Processing $basename..."
  cargo run -q -p abuse-tools -- level-dump "$level" --format json > \
    "tests/baselines/levels/${basename}.json"
done

# Regenerate frabs addon levels
for level in /path/to/abuse-0.8/data/levels/frabs*.spe; do
  basename=$(basename "$level" .spe)
  echo "Processing $basename..."
  cargo run -q -p abuse-tools -- level-dump "$level" --format json > \
    "tests/baselines/levels/${basename}.json"
done
```

## What's Included in Baselines

Each baseline JSON file contains:

- **Level metadata**: first_name (if present)
- **Map data**: foreground/background dimensions, tile counts, and tile samples (first 10 tiles)
- **Background scroll rate**: parallax multiplier/divisor values
- **Object data**: counts, type/state names counts, and object samples with position/state/hp
- **Light data**: min_light_level, counts, and light samples with positions/radii
- **Link data**: object-to-object and object-to-light link counts and samples

The baseline format uses samples (first 10 items) rather than full data dumps to
keep file sizes manageable while still providing meaningful validation coverage.

## Current Coverage

- **22 main campaign levels**: level00.spe through level21.spe
- **27 Frabs addon levels**: frabs00.spe through frabs74.spe (with gaps)
- **Total**: 49 level baselines

All baselines are automatically tested when `ABUSE_LEGACY_ROOT` is set.
