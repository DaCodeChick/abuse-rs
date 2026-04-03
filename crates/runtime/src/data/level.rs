//! Level data loading and parsing.
//!
//! This module provides functionality for loading game level data from binary files
//! that use the SPE (Special Purpose Entry) directory format. It handles parsing of
//! tilemaps, game objects, lighting, and their interconnections.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use byteorder::{LittleEndian, ReadBytesExt};
use thiserror::Error;

use crate::data::spe::{SpeDirectory, SpeError};

/// Record type markers used in the binary level format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RecordType {
    /// 8-bit record type.
    U8 = 0,
    /// 16-bit record type.
    U16 = 1,
    /// 32-bit record type.
    U32 = 2,
}

impl RecordType {
    /// Converts a u8 value to a RecordType.
    ///
    /// # Errors
    ///
    /// Returns `None` if the value is not a valid record type marker.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::U8),
            1 => Some(Self::U16),
            2 => Some(Self::U32),
            _ => None,
        }
    }
}

impl From<RecordType> for u8 {
    fn from(rt: RecordType) -> Self {
        rt as u8
    }
}

/// Object variable indices for accessing standard object properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum ObjectVar {
    /// Fade direction.
    FadeDir = 0,
    /// Frame direction.
    FrameDir = 1,
    /// Movement direction.
    Direction = 2,
    /// Whether gravity is enabled.
    GravityOn = 3,
    /// Current fade counter.
    FadeCount = 4,
    /// Maximum fade value.
    FadeMax = 5,
    /// Whether object is active.
    Active = 6,
    /// Object flags.
    Flags = 7,
    /// AI type identifier.
    AiType = 8,
    /// Horizontal velocity (integer part).
    XVel = 9,
    /// Horizontal velocity (fractional part).
    FxVel = 10,
    /// Vertical velocity (integer part).
    YVel = 11,
    /// Vertical velocity (fractional part).
    FyVel = 12,
    /// Horizontal acceleration (integer part).
    XAcel = 13,
    /// Horizontal acceleration (fractional part).
    FxAcel = 14,
    /// Vertical acceleration (integer part).
    YAcel = 15,
    /// Vertical acceleration (fractional part).
    FyAcel = 16,
    /// X-coordinate (integer part).
    X = 17,
    /// X-coordinate (fractional part).
    Fx = 18,
    /// Y-coordinate (integer part).
    Y = 19,
    /// Y-coordinate (fractional part).
    Fy = 20,
    /// Health points.
    Hp = 21,
    /// Magic/mana points.
    Mp = 22,
    /// Magic/mana points (fractional part).
    Fmp = 23,
    /// Current animation frame.
    CurFrame = 24,
    /// AI state identifier.
    AiState = 25,
    /// Time in current AI state.
    AiStateTime = 26,
    /// Whether object can be targeted.
    Targetable = 27,
}

impl ObjectVar {
    /// Returns the total number of object variables.
    pub const fn count() -> usize {
        28
    }

    /// Converts a usize index to an ObjectVar.
    ///
    /// # Errors
    ///
    /// Returns `None` if the index is out of range.
    pub fn from_usize(value: usize) -> Option<Self> {
        match value {
            0 => Some(Self::FadeDir),
            1 => Some(Self::FrameDir),
            2 => Some(Self::Direction),
            3 => Some(Self::GravityOn),
            4 => Some(Self::FadeCount),
            5 => Some(Self::FadeMax),
            6 => Some(Self::Active),
            7 => Some(Self::Flags),
            8 => Some(Self::AiType),
            9 => Some(Self::XVel),
            10 => Some(Self::FxVel),
            11 => Some(Self::YVel),
            12 => Some(Self::FyVel),
            13 => Some(Self::XAcel),
            14 => Some(Self::FxAcel),
            15 => Some(Self::YAcel),
            16 => Some(Self::FyAcel),
            17 => Some(Self::X),
            18 => Some(Self::Fx),
            19 => Some(Self::Y),
            20 => Some(Self::Fy),
            21 => Some(Self::Hp),
            22 => Some(Self::Mp),
            23 => Some(Self::Fmp),
            24 => Some(Self::CurFrame),
            25 => Some(Self::AiState),
            26 => Some(Self::AiStateTime),
            27 => Some(Self::Targetable),
            _ => None,
        }
    }
}

impl From<ObjectVar> for usize {
    fn from(var: ObjectVar) -> Self {
        var as usize
    }
}

/// Total number of predefined object variables.
const TOTAL_OBJECT_VARS: usize = ObjectVar::count();

/// Specifications for object variable entries in the level file.
///
/// Each entry specifies the variable name and its storage type (8-bit, 16-bit, or 32-bit).
/// The order of entries in this array determines the variable index.
const OBJECT_VAR_SPECS: [(&str, RecordType); TOTAL_OBJECT_VARS] = [
    ("fade_dir", RecordType::U8),
    ("frame_dir", RecordType::U8),
    ("direction", RecordType::U8),
    ("gravity_on", RecordType::U8),
    ("fade_count", RecordType::U8),
    ("fade_max", RecordType::U8),
    ("active", RecordType::U8),
    ("flags", RecordType::U8),
    ("aitype", RecordType::U8),
    ("xvel", RecordType::U32),
    ("fxvel", RecordType::U8),
    ("yvel", RecordType::U32),
    ("fyvel", RecordType::U8),
    ("xacel", RecordType::U32),
    ("fxacel", RecordType::U8),
    ("yacel", RecordType::U32),
    ("fyacel", RecordType::U8),
    ("x", RecordType::U32),
    ("fx", RecordType::U8),
    ("y", RecordType::U32),
    ("fy", RecordType::U8),
    ("hp", RecordType::U16),
    ("mp", RecordType::U16),
    ("fmp", RecordType::U16),
    ("cur_frame", RecordType::U16),
    ("aistate", RecordType::U16),
    ("aistate_time", RecordType::U16),
    ("targetable", RecordType::U8),
];

/// Represents a complete game level with all its components.
#[derive(Debug, Clone)]
pub struct LevelData {
    /// Display name or path of the level.
    pub name: String,
    /// Optional first name field (purpose varies by level format).
    pub first_name: Option<String>,
    /// Width of the foreground tilemap in tiles.
    pub fg_width: u32,
    /// Height of the foreground tilemap in tiles.
    pub fg_height: u32,
    /// Foreground tile indices (row-major order).
    pub fg_tiles: Vec<u16>,
    /// Width of the background tilemap in tiles.
    pub bg_width: u32,
    /// Height of the background tilemap in tiles.
    pub bg_height: u32,
    /// Background tile indices (row-major order).
    pub bg_tiles: Vec<u16>,
    /// Background horizontal scroll rate multiplier.
    pub bg_xmul: u32,
    /// Background horizontal scroll rate divisor.
    pub bg_xdiv: u32,
    /// Background vertical scroll rate multiplier.
    pub bg_ymul: u32,
    /// Background vertical scroll rate divisor.
    pub bg_ydiv: u32,
    /// Number of objects in the level (if specified).
    pub object_count: Option<u32>,
    /// All game objects in the level.
    pub objects: Vec<LoadedObject>,
    /// Names of all object types.
    pub object_type_names: Vec<String>,
    /// State names for each object type.
    pub object_state_names: Vec<Vec<String>>,
    /// Minimum light level for the entire level.
    pub min_light_level: Option<u32>,
    /// All light sources in the level.
    pub lights: Vec<LoadedLight>,
    /// Relationships between objects.
    pub object_links: Vec<ObjectLink>,
    /// Relationships between objects and lights.
    pub light_links: Vec<LightLink>,
}

/// Represents a game object loaded from a level file.
#[derive(Debug, Clone)]
pub struct LoadedObject {
    /// Numeric identifier for the object's type.
    pub type_id: u16,
    /// Numeric identifier for the object's current state.
    pub state_id: u16,
    /// Human-readable type name (if available).
    pub type_name: Option<String>,
    /// Human-readable state name (if available).
    pub state_name: Option<String>,
    /// Level-specific variables (lvars) for this object.
    pub lvars: Vec<i32>,
    /// Standard object variables (position, velocity, etc.).
    pub vars: [i32; TOTAL_OBJECT_VARS],
}

/// Represents a light source in the level.
#[derive(Debug, Clone)]
pub struct LoadedLight {
    /// Type of light source.
    pub light_type: u8,
    /// X-coordinate of the light.
    pub x: i32,
    /// Y-coordinate of the light.
    pub y: i32,
    /// Horizontal offset for light positioning.
    pub xshift: i32,
    /// Vertical offset for light positioning.
    pub yshift: i32,
    /// Inner radius where light is at full intensity.
    pub inner_radius: i32,
    /// Outer radius where light fades to zero.
    pub outer_radius: i32,
}

/// Links one object to another object.
#[derive(Debug, Clone)]
pub struct ObjectLink {
    /// Index of the source object.
    pub from_object: i32,
    /// Index of the target object.
    pub to_object: i32,
}

/// Links an object to a light source.
#[derive(Debug, Clone)]
pub struct LightLink {
    /// Index of the object.
    pub from_object: i32,
    /// Index of the light source.
    pub to_light: i32,
}

impl LoadedObject {
    /// Retrieves the value of a standard object variable by index.
    ///
    /// # Arguments
    ///
    /// * `index` - The variable to retrieve
    ///
    /// # Returns
    ///
    /// The variable value if the index is valid, or `None` otherwise.
    pub fn var(&self, var: ObjectVar) -> Option<i32> {
        self.vars.get(var as usize).copied()
    }
}

/// Errors that can occur when loading level data.
#[derive(Debug, Error)]
pub enum LevelError {
    /// Failed to open the level file.
    #[error("failed to open level file at {path}: {source}")]
    Open {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// Failed to read from the level file.
    #[error("failed to read level file at {path}: {source}")]
    Read {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// Failed to parse the SPE directory structure.
    #[error("failed to parse level spe directory: {source}")]
    Spe {
        #[from]
        source: SpeError,
    },
    /// A required entry is missing from the level file.
    #[error("missing mandatory entry: {entry}")]
    MissingEntry { entry: &'static str },
    /// Tile count overflowed when multiplying dimensions.
    #[error("map tile count overflow for dimensions {width}x{height}")]
    TileCountOverflow { width: u32, height: u32 },
    /// String data contains invalid UTF-8.
    #[error("invalid utf-8 string in entry {entry}")]
    InvalidString { entry: &'static str },
    /// Object array has an unexpected type marker.
    #[error("invalid object array marker for {entry}: expected {expected:?}, got {actual}")]
    InvalidObjectArrayMarker {
        entry: &'static str,
        expected: RecordType,
        actual: u8,
    },
}

impl LevelData {
    /// Opens and parses a level file from the given path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the level file
    ///
    /// # Errors
    ///
    /// Returns a [`LevelError`] if the file cannot be opened, read, or parsed.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, LevelError> {
        let path_ref = path.as_ref();
        let directory = SpeDirectory::open_lenient(path_ref)?;
        let mut file = File::open(path_ref).map_err(|source| LevelError::Open {
            path: path_ref.to_path_buf(),
            source,
        })?;

        let first_name = match directory.find_by_name("first name") {
            Some(entry) => Some(read_len_prefixed_string(
                &mut file,
                path_ref,
                u64::from(entry.offset),
                "first name",
            )?),
            None => None,
        };

        let (fg_width, fg_height, fg_tiles) = read_map(&directory, &mut file, path_ref, "fgmap")?;
        let (bg_width, bg_height, bg_tiles) = read_map(&directory, &mut file, path_ref, "bgmap")?;
        let (bg_xmul, bg_xdiv, bg_ymul, bg_ydiv) =
            read_bg_scroll_rate(&directory, &mut file, path_ref)?;
        let object_count = read_object_count(&directory, &mut file, path_ref)?;
        let (object_type_names, object_state_names) =
            read_object_descriptions(&directory, &mut file, path_ref)?;
        let objects = read_objects(
            &directory,
            &mut file,
            path_ref,
            object_count,
            &object_type_names,
            &object_state_names,
        )?;
        let (min_light_level, lights) = read_lights(&directory, &mut file, path_ref)?;
        let object_links = read_object_links(&directory, &mut file, path_ref)?;
        let light_links = read_light_links(&directory, &mut file, path_ref)?;

        Ok(Self {
            name: path_ref
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string(),
            first_name,
            fg_width,
            fg_height,
            fg_tiles,
            bg_width,
            bg_height,
            bg_tiles,
            bg_xmul,
            bg_xdiv,
            bg_ymul,
            bg_ydiv,
            object_count,
            objects,
            object_type_names,
            object_state_names,
            min_light_level,
            lights,
            object_links,
            light_links,
        })
    }
}

/// Reads light data from the level file.
///
/// # Returns
///
/// A tuple containing the minimum light level (if present) and a vector of lights.
fn read_lights(
    directory: &SpeDirectory,
    file: &mut File,
    path: &Path,
) -> Result<(Option<u32>, Vec<LoadedLight>), LevelError> {
    let Some(entry) = directory.find_by_name("lights") else {
        return Ok((None, Vec::new()));
    };

    file.seek(SeekFrom::Start(u64::from(entry.offset)))
        .map_err(|source| LevelError::Read {
            path: path.to_path_buf(),
            source,
        })?;

    let total = file
        .read_u32::<LittleEndian>()
        .map_err(|source| LevelError::Read {
            path: path.to_path_buf(),
            source,
        })?;
    let min_light_level = file
        .read_u32::<LittleEndian>()
        .map_err(|source| LevelError::Read {
            path: path.to_path_buf(),
            source,
        })?;

    let mut lights = Vec::with_capacity(usize::try_from(total).unwrap_or(0));
    for _ in 0..total {
        let x = file
            .read_i32::<LittleEndian>()
            .map_err(|source| LevelError::Read {
                path: path.to_path_buf(),
                source,
            })?;
        let y = file
            .read_i32::<LittleEndian>()
            .map_err(|source| LevelError::Read {
                path: path.to_path_buf(),
                source,
            })?;
        let xshift = file
            .read_i32::<LittleEndian>()
            .map_err(|source| LevelError::Read {
                path: path.to_path_buf(),
                source,
            })?;
        let yshift = file
            .read_i32::<LittleEndian>()
            .map_err(|source| LevelError::Read {
                path: path.to_path_buf(),
                source,
            })?;
        let inner_radius = file
            .read_i32::<LittleEndian>()
            .map_err(|source| LevelError::Read {
                path: path.to_path_buf(),
                source,
            })?;
        let outer_radius = file
            .read_i32::<LittleEndian>()
            .map_err(|source| LevelError::Read {
                path: path.to_path_buf(),
                source,
            })?;
        let light_type = file.read_u8().map_err(|source| LevelError::Read {
            path: path.to_path_buf(),
            source,
        })?;

        lights.push(LoadedLight {
            light_type,
            x,
            y,
            xshift,
            yshift,
            inner_radius,
            outer_radius,
        });
    }

    Ok((Some(min_light_level), lights))
}

/// Reads object-to-object link data from the level file.
fn read_object_links(
    directory: &SpeDirectory,
    file: &mut File,
    path: &Path,
) -> Result<Vec<ObjectLink>, LevelError> {
    let Some(entry) = directory.find_by_name("object_links") else {
        return Ok(Vec::new());
    };

    file.seek(SeekFrom::Start(u64::from(entry.offset)))
        .map_err(|source| LevelError::Read {
            path: path.to_path_buf(),
            source,
        })?;

    let marker = file.read_u8().map_err(|source| LevelError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    if marker != RecordType::U32 as u8 {
        return Err(LevelError::InvalidObjectArrayMarker {
            entry: "object_links",
            expected: RecordType::U32,
            actual: marker,
        });
    }

    let total = file
        .read_u32::<LittleEndian>()
        .map_err(|source| LevelError::Read {
            path: path.to_path_buf(),
            source,
        })?;

    let mut links = Vec::with_capacity(usize::try_from(total).unwrap_or(0));
    for _ in 0..total {
        let from_object = file
            .read_i32::<LittleEndian>()
            .map_err(|source| LevelError::Read {
                path: path.to_path_buf(),
                source,
            })?;
        let to_object = file
            .read_i32::<LittleEndian>()
            .map_err(|source| LevelError::Read {
                path: path.to_path_buf(),
                source,
            })?;
        links.push(ObjectLink {
            from_object,
            to_object,
        });
    }

    Ok(links)
}

/// Reads object-to-light link data from the level file.
fn read_light_links(
    directory: &SpeDirectory,
    file: &mut File,
    path: &Path,
) -> Result<Vec<LightLink>, LevelError> {
    let Some(entry) = directory.find_by_name("light_links") else {
        return Ok(Vec::new());
    };

    file.seek(SeekFrom::Start(u64::from(entry.offset)))
        .map_err(|source| LevelError::Read {
            path: path.to_path_buf(),
            source,
        })?;

    let marker = file.read_u8().map_err(|source| LevelError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    if marker != RecordType::U32 as u8 {
        return Err(LevelError::InvalidObjectArrayMarker {
            entry: "light_links",
            expected: RecordType::U32,
            actual: marker,
        });
    }

    let total = file
        .read_u32::<LittleEndian>()
        .map_err(|source| LevelError::Read {
            path: path.to_path_buf(),
            source,
        })?;

    let mut links = Vec::with_capacity(usize::try_from(total).unwrap_or(0));
    for _ in 0..total {
        let from_object = file
            .read_i32::<LittleEndian>()
            .map_err(|source| LevelError::Read {
                path: path.to_path_buf(),
                source,
            })?;
        let to_light = file
            .read_i32::<LittleEndian>()
            .map_err(|source| LevelError::Read {
                path: path.to_path_buf(),
                source,
            })?;
        links.push(LightLink {
            from_object,
            to_light,
        });
    }

    Ok(links)
}

/// Reads all object data from the level file.
///
/// Combines type IDs, state IDs, lvars, and vars into complete object structures.
fn read_objects(
    directory: &SpeDirectory,
    file: &mut File,
    path: &Path,
    object_count: Option<u32>,
    object_type_names: &[String],
    object_state_names: &[Vec<String>],
) -> Result<Vec<LoadedObject>, LevelError> {
    let Some(total_u32) = object_count else {
        return Ok(Vec::new());
    };
    let total = usize::try_from(total_u32).map_err(|_| LevelError::TileCountOverflow {
        width: total_u32,
        height: 1,
    })?;

    let type_ids = read_object_u16_array(directory, file, path, "type", RecordType::U16, total)?;
    let state_ids = read_object_u16_array(directory, file, path, "state", RecordType::U16, total)?;
    let lvars = read_object_lvars(directory, file, path, total)?;
    let vars = read_object_vars(directory, file, path, total)?;

    let mut objects = Vec::with_capacity(total);
    for idx in 0..total {
        objects.push(LoadedObject {
            type_id: type_ids[idx],
            state_id: state_ids[idx],
            type_name: object_type_names.get(type_ids[idx] as usize).cloned(),
            state_name: object_state_names
                .get(type_ids[idx] as usize)
                .and_then(|states| states.get(state_ids[idx] as usize))
                .cloned(),
            lvars: lvars[idx].clone(),
            vars: vars[idx],
        });
    }

    Ok(objects)
}

/// Reads object type and state name descriptions from the level file.
///
/// # Returns
///
/// A tuple containing:
/// - Vector of object type names
/// - Vector of vectors where each inner vector contains state names for that type
fn read_object_descriptions(
    directory: &SpeDirectory,
    file: &mut File,
    path: &Path,
) -> Result<(Vec<String>, Vec<Vec<String>>), LevelError> {
    let Some(descriptions) = directory.find_by_name("object_descripitions") else {
        return Ok((Vec::new(), Vec::new()));
    };

    file.seek(SeekFrom::Start(u64::from(descriptions.offset)))
        .map_err(|source| LevelError::Read {
            path: path.to_path_buf(),
            source,
        })?;
    let total = file
        .read_u16::<LittleEndian>()
        .map_err(|source| LevelError::Read {
            path: path.to_path_buf(),
            source,
        })? as usize;

    let mut type_names = Vec::new();
    if let Some(entry) = directory.find_by_name("describe_names") {
        file.seek(SeekFrom::Start(u64::from(entry.offset)))
            .map_err(|source| LevelError::Read {
                path: path.to_path_buf(),
                source,
            })?;
        type_names.reserve(total);
        for _ in 0..total {
            type_names.push(read_cursor_len_prefixed_string(
                file,
                path,
                "describe_names",
            )?);
        }
    }

    let mut state_names = Vec::new();
    if let Some(entry) = directory.find_by_name("describe_states") {
        file.seek(SeekFrom::Start(u64::from(entry.offset)))
            .map_err(|source| LevelError::Read {
                path: path.to_path_buf(),
                source,
            })?;
        state_names.reserve(total);

        for _ in 0..total {
            let states_total =
                file.read_u16::<LittleEndian>()
                    .map_err(|source| LevelError::Read {
                        path: path.to_path_buf(),
                        source,
                    })? as usize;
            let mut states = Vec::with_capacity(states_total);
            for _ in 0..states_total {
                states.push(read_cursor_len_prefixed_string(
                    file,
                    path,
                    "describe_states",
                )?);
            }
            state_names.push(states);
        }
    }

    Ok((type_names, state_names))
}

/// Reads a length-prefixed string from the current file position.
///
/// The string format is: 1 byte length, followed by that many bytes of data.
/// Trailing null bytes are removed.
fn read_cursor_len_prefixed_string(
    file: &mut File,
    path: &Path,
    entry_name: &'static str,
) -> Result<String, LevelError> {
    let len = file.read_u8().map_err(|source| LevelError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let mut bytes = vec![0_u8; usize::from(len)];
    file.read_exact(&mut bytes)
        .map_err(|source| LevelError::Read {
            path: path.to_path_buf(),
            source,
        })?;

    if bytes.last().copied() == Some(0) {
        bytes.pop();
    }

    String::from_utf8(bytes).map_err(|_| LevelError::InvalidString { entry: entry_name })
}

/// Reads an array of u16 values for objects from the level file.
///
/// # Arguments
///
/// * `entry_name` - Name of the directory entry to read
/// * `expected_marker` - Expected record type marker
/// * `total` - Number of values to read
fn read_object_u16_array(
    directory: &SpeDirectory,
    file: &mut File,
    path: &Path,
    entry_name: &'static str,
    expected_marker: RecordType,
    total: usize,
) -> Result<Vec<u16>, LevelError> {
    let Some(entry) = directory.find_by_name(entry_name) else {
        return Ok(vec![0_u16; total]);
    };

    file.seek(SeekFrom::Start(u64::from(entry.offset)))
        .map_err(|source| LevelError::Read {
            path: path.to_path_buf(),
            source,
        })?;

    let marker = file.read_u8().map_err(|source| LevelError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    if marker != expected_marker as u8 {
        return Err(LevelError::InvalidObjectArrayMarker {
            entry: entry_name,
            expected: expected_marker,
            actual: marker,
        });
    }

    let mut out = vec![0_u16; total];
    for item in &mut out {
        *item = file
            .read_u16::<LittleEndian>()
            .map_err(|source| LevelError::Read {
                path: path.to_path_buf(),
                source,
            })?;
    }
    Ok(out)
}

/// Reads level-specific variables (lvars) for all objects.
///
/// Each object can have a variable number of lvars stored sequentially.
fn read_object_lvars(
    directory: &SpeDirectory,
    file: &mut File,
    path: &Path,
    total: usize,
) -> Result<Vec<Vec<i32>>, LevelError> {
    let Some(entry) = directory.find_by_name("lvars") else {
        return Ok(vec![Vec::new(); total]);
    };

    file.seek(SeekFrom::Start(u64::from(entry.offset)))
        .map_err(|source| LevelError::Read {
            path: path.to_path_buf(),
            source,
        })?;

    let mut all = Vec::with_capacity(total);
    for _ in 0..total {
        let count = file
            .read_u16::<LittleEndian>()
            .map_err(|source| LevelError::Read {
                path: path.to_path_buf(),
                source,
            })?;
        let mut vars = Vec::with_capacity(usize::from(count));

        for _ in 0..count {
            let marker = file.read_u8().map_err(|source| LevelError::Read {
                path: path.to_path_buf(),
                source,
            })?;
            if marker != RecordType::U32 as u8 {
                return Err(LevelError::InvalidObjectArrayMarker {
                    entry: "lvars",
                    expected: RecordType::U32,
                    actual: marker,
                });
            }

            let value = file
                .read_i32::<LittleEndian>()
                .map_err(|source| LevelError::Read {
                    path: path.to_path_buf(),
                    source,
                })?;
            vars.push(value);
        }

        all.push(vars);
    }

    Ok(all)
}

/// Reads standard object variables for all objects.
///
/// Iterates through all variable types defined in `OBJECT_VAR_SPECS` and reads
/// their values for each object, handling different storage sizes (8/16/32-bit).
fn read_object_vars(
    directory: &SpeDirectory,
    file: &mut File,
    path: &Path,
    total: usize,
) -> Result<Vec<[i32; TOTAL_OBJECT_VARS]>, LevelError> {
    let mut objects = vec![[0_i32; TOTAL_OBJECT_VARS]; total];

    for (var_index, (entry_name, expected_marker)) in OBJECT_VAR_SPECS.iter().enumerate() {
        let Some(entry) = directory.find_by_name(entry_name) else {
            continue;
        };

        file.seek(SeekFrom::Start(u64::from(entry.offset)))
            .map_err(|source| LevelError::Read {
                path: path.to_path_buf(),
                source,
            })?;

        let marker = file.read_u8().map_err(|source| LevelError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        if marker != *expected_marker as u8 {
            return Err(LevelError::InvalidObjectArrayMarker {
                entry: entry_name,
                expected: *expected_marker,
                actual: marker,
            });
        }

        for object in &mut objects {
            let value = match expected_marker {
                RecordType::U8 => i32::from(file.read_u8().map_err(|source| LevelError::Read {
                    path: path.to_path_buf(),
                    source,
                })?),
                RecordType::U16 => {
                    i32::from(file.read_u16::<LittleEndian>().map_err(|source| {
                        LevelError::Read {
                            path: path.to_path_buf(),
                            source,
                        }
                    })?)
                }
                RecordType::U32 => {
                    file.read_i32::<LittleEndian>()
                        .map_err(|source| LevelError::Read {
                            path: path.to_path_buf(),
                            source,
                        })?
                }
            };
            object[var_index] = value;
        }
    }

    Ok(objects)
}

/// Reads a tilemap (foreground or background) from the level file.
///
/// # Arguments
///
/// * `entry_name` - Name of the directory entry ("fgmap" or "bgmap")
///
/// # Returns
///
/// A tuple containing:
/// - Map width in tiles
/// - Map height in tiles
/// - Vector of tile indices (row-major order)
fn read_map(
    directory: &SpeDirectory,
    file: &mut File,
    path: &Path,
    entry_name: &'static str,
) -> Result<(u32, u32, Vec<u16>), LevelError> {
    let entry = directory
        .find_by_name(entry_name)
        .ok_or(LevelError::MissingEntry { entry: entry_name })?;

    file.seek(SeekFrom::Start(u64::from(entry.offset)))
        .map_err(|source| LevelError::Read {
            path: path.to_path_buf(),
            source,
        })?;

    let width = file
        .read_u32::<LittleEndian>()
        .map_err(|source| LevelError::Read {
            path: path.to_path_buf(),
            source,
        })?;
    let height = file
        .read_u32::<LittleEndian>()
        .map_err(|source| LevelError::Read {
            path: path.to_path_buf(),
            source,
        })?;

    let total = width
        .checked_mul(height)
        .ok_or(LevelError::TileCountOverflow { width, height })?;
    let mut tiles = vec![
        0_u16;
        usize::try_from(total)
            .map_err(|_| LevelError::TileCountOverflow { width, height })?
    ];

    for tile in &mut tiles {
        *tile = file
            .read_u16::<LittleEndian>()
            .map_err(|source| LevelError::Read {
                path: path.to_path_buf(),
                source,
            })?;
    }

    Ok((width, height, tiles))
}

/// Reads the background parallax scroll rate from the level file.
///
/// # Returns
///
/// A tuple containing (xmul, xdiv, ymul, ydiv) for parallax calculations.
/// Defaults to (1, 8, 1, 8) if not present or invalid.
fn read_bg_scroll_rate(
    directory: &SpeDirectory,
    file: &mut File,
    path: &Path,
) -> Result<(u32, u32, u32, u32), LevelError> {
    let Some(entry) = directory.find_by_name("bg_scroll_rate") else {
        return Ok((1, 8, 1, 8));
    };

    file.seek(SeekFrom::Start(u64::from(entry.offset)))
        .map_err(|source| LevelError::Read {
            path: path.to_path_buf(),
            source,
        })?;

    let marker = file.read_u8().map_err(|source| LevelError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    if marker != RecordType::U32 as u8 {
        return Ok((1, 8, 1, 8));
    }

    let bg_xmul = file
        .read_u32::<LittleEndian>()
        .map_err(|source| LevelError::Read {
            path: path.to_path_buf(),
            source,
        })?;
    let bg_xdiv = file
        .read_u32::<LittleEndian>()
        .map_err(|source| LevelError::Read {
            path: path.to_path_buf(),
            source,
        })?;
    let bg_ymul = file
        .read_u32::<LittleEndian>()
        .map_err(|source| LevelError::Read {
            path: path.to_path_buf(),
            source,
        })?;
    let bg_ydiv = file
        .read_u32::<LittleEndian>()
        .map_err(|source| LevelError::Read {
            path: path.to_path_buf(),
            source,
        })?;

    Ok((bg_xmul, bg_xdiv, bg_ymul, bg_ydiv))
}

/// Reads the total count of objects in the level.
///
/// # Returns
///
/// The object count if the "object_list" entry exists, or `None` otherwise.
fn read_object_count(
    directory: &SpeDirectory,
    file: &mut File,
    path: &Path,
) -> Result<Option<u32>, LevelError> {
    let Some(entry) = directory.find_by_name("object_list") else {
        return Ok(None);
    };

    file.seek(SeekFrom::Start(u64::from(entry.offset)))
        .map_err(|source| LevelError::Read {
            path: path.to_path_buf(),
            source,
        })?;

    let count = file
        .read_u32::<LittleEndian>()
        .map_err(|source| LevelError::Read {
            path: path.to_path_buf(),
            source,
        })?;

    Ok(Some(count))
}

/// Reads a length-prefixed string from a specific file offset.
///
/// Similar to `read_cursor_len_prefixed_string`, but seeks to the offset first.
///
/// # Arguments
///
/// * `offset` - Byte offset in the file to seek to
/// * `entry_name` - Name of the entry (for error reporting)
fn read_len_prefixed_string(
    file: &mut File,
    path: &Path,
    offset: u64,
    entry_name: &'static str,
) -> Result<String, LevelError> {
    file.seek(SeekFrom::Start(offset))
        .map_err(|source| LevelError::Read {
            path: path.to_path_buf(),
            source,
        })?;

    let len = file.read_u8().map_err(|source| LevelError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let mut bytes = vec![0_u8; usize::from(len)];
    file.read_exact(&mut bytes)
        .map_err(|source| LevelError::Read {
            path: path.to_path_buf(),
            source,
        })?;

    if bytes.last().copied() == Some(0) {
        bytes.pop();
    }

    String::from_utf8(bytes).map_err(|_| LevelError::InvalidString { entry: entry_name })
}
