//! Constants for the legacy debug viewer.

/// Default fallback tile size used when archive tile dimensions are unknown.
pub const FG_TILE_SIZE: f32 = 32.0;

/// Foreground tile archives loaded in order for tile lookup.
pub const FG_TILE_SPE_FILES: &[&str] = &[
    "art/fore/foregrnd.spe",
    "art/fore/techno.spe",
    "art/fore/techno2.spe",
    "art/fore/techno3.spe",
    "art/fore/techno4.spe",
    "art/fore/cave.spe",
    "art/fore/alien.spe",
    "art/fore/trees.spe",
    "art/fore/endgame.spe",
    "art/fore/trees2.spe",
];

/// Background tile archives loaded in order for tile lookup.
pub const BG_TILE_SPE_FILES: &[&str] = &[
    "art/back/backgrnd.spe",
    "art/back/intro.spe",
    "art/back/city.spe",
    "art/back/cave.spe",
    "art/back/tech.spe",
    "art/back/alienb.spe",
    "art/back/green2.spe",
    "art/back/galien.spe",
];

/// Object sprite archives used by the object render mapper.
pub const OBJECT_SPE_FILES: &[&str] = &[
    "art/door.spe",
    "art/chars/door.spe",
    "art/chars/tdoor.spe",
    "art/chars/teleport.spe",
    "art/chars/platform.spe",
    "art/chars/lightin.spe",
    "art/chars/lava.spe",
    "art/chars/step.spe",
    "art/ball.spe",
    "art/compass.spe",
    "art/rob2.spe",
    "art/misc.spe",
];
