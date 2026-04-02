use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use byteorder::{LittleEndian, ReadBytesExt};
use thiserror::Error;

use crate::data::spe::{SpeDirectory, SpeError};

const RC_8: u8 = 0;
const RC_16: u8 = 1;
const RC_32: u8 = 2;
const TOTAL_OBJECT_VARS: usize = 28;

const OBJECT_VAR_SPECS: [(&str, u8); TOTAL_OBJECT_VARS] = [
    ("fade_dir", RC_8),
    ("frame_dir", RC_8),
    ("direction", RC_8),
    ("gravity_on", RC_8),
    ("fade_count", RC_8),
    ("fade_max", RC_8),
    ("active", RC_8),
    ("flags", RC_8),
    ("aitype", RC_8),
    ("xvel", RC_32),
    ("fxvel", RC_8),
    ("yvel", RC_32),
    ("fyvel", RC_8),
    ("xacel", RC_32),
    ("fxacel", RC_8),
    ("yacel", RC_32),
    ("fyacel", RC_8),
    ("x", RC_32),
    ("fx", RC_8),
    ("y", RC_32),
    ("fy", RC_8),
    ("hp", RC_16),
    ("mp", RC_16),
    ("fmp", RC_16),
    ("cur_frame", RC_16),
    ("aistate", RC_16),
    ("aistate_time", RC_16),
    ("targetable", RC_8),
];

pub const VAR_X: usize = 17;
pub const VAR_Y: usize = 19;
pub const VAR_HP: usize = 21;

#[derive(Debug, Clone)]
pub struct LevelData {
    pub name: String,
    pub first_name: Option<String>,
    pub fg_width: u32,
    pub fg_height: u32,
    pub fg_tiles: Vec<u16>,
    pub bg_width: u32,
    pub bg_height: u32,
    pub bg_tiles: Vec<u16>,
    pub bg_xmul: u32,
    pub bg_xdiv: u32,
    pub bg_ymul: u32,
    pub bg_ydiv: u32,
    pub object_count: Option<u32>,
    pub objects: Vec<LoadedObject>,
    pub min_light_level: Option<u32>,
    pub lights: Vec<LoadedLight>,
    pub object_links: Vec<ObjectLink>,
    pub light_links: Vec<LightLink>,
}

#[derive(Debug, Clone)]
pub struct LoadedObject {
    pub type_id: u16,
    pub state_id: u16,
    pub lvars: Vec<i32>,
    pub vars: [i32; TOTAL_OBJECT_VARS],
}

#[derive(Debug, Clone)]
pub struct LoadedLight {
    pub light_type: u8,
    pub x: i32,
    pub y: i32,
    pub xshift: i32,
    pub yshift: i32,
    pub inner_radius: i32,
    pub outer_radius: i32,
}

#[derive(Debug, Clone)]
pub struct ObjectLink {
    pub from_object: i32,
    pub to_object: i32,
}

#[derive(Debug, Clone)]
pub struct LightLink {
    pub from_object: i32,
    pub to_light: i32,
}

impl LoadedObject {
    pub fn var(&self, index: usize) -> Option<i32> {
        self.vars.get(index).copied()
    }
}

#[derive(Debug, Error)]
pub enum LevelError {
    #[error("failed to open level file at {path}: {source}")]
    Open {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read level file at {path}: {source}")]
    Read {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse level spe directory: {source}")]
    Spe {
        #[from]
        source: SpeError,
    },
    #[error("missing mandatory entry: {entry}")]
    MissingEntry { entry: &'static str },
    #[error("map tile count overflow for dimensions {width}x{height}")]
    TileCountOverflow { width: u32, height: u32 },
    #[error("invalid utf-8 string in entry {entry}")]
    InvalidString { entry: &'static str },
    #[error("invalid object array marker for {entry}: expected {expected}, got {actual}")]
    InvalidObjectArrayMarker {
        entry: &'static str,
        expected: u8,
        actual: u8,
    },
}

impl LevelData {
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
        let objects = read_objects(&directory, &mut file, path_ref, object_count)?;
        let (min_light_level, lights) = read_lights(&directory, &mut file, path_ref)?;
        let object_links = read_object_links(&directory, &mut file, path_ref)?;
        let light_links = read_light_links(&directory, &mut file, path_ref)?;

        Ok(Self {
            name: path_ref.display().to_string(),
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
            min_light_level,
            lights,
            object_links,
            light_links,
        })
    }
}

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
    if marker != RC_32 {
        return Err(LevelError::InvalidObjectArrayMarker {
            entry: "object_links",
            expected: RC_32,
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
    if marker != RC_32 {
        return Err(LevelError::InvalidObjectArrayMarker {
            entry: "light_links",
            expected: RC_32,
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

fn read_objects(
    directory: &SpeDirectory,
    file: &mut File,
    path: &Path,
    object_count: Option<u32>,
) -> Result<Vec<LoadedObject>, LevelError> {
    let Some(total_u32) = object_count else {
        return Ok(Vec::new());
    };
    let total = usize::try_from(total_u32).map_err(|_| LevelError::TileCountOverflow {
        width: total_u32,
        height: 1,
    })?;

    let type_ids = read_object_u16_array(directory, file, path, "type", RC_16, total)?;
    let state_ids = read_object_u16_array(directory, file, path, "state", RC_16, total)?;
    let lvars = read_object_lvars(directory, file, path, total)?;
    let vars = read_object_vars(directory, file, path, total)?;

    let mut objects = Vec::with_capacity(total);
    for idx in 0..total {
        objects.push(LoadedObject {
            type_id: type_ids[idx],
            state_id: state_ids[idx],
            lvars: lvars[idx].clone(),
            vars: vars[idx],
        });
    }

    Ok(objects)
}

fn read_object_u16_array(
    directory: &SpeDirectory,
    file: &mut File,
    path: &Path,
    entry_name: &'static str,
    expected_marker: u8,
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
    if marker != expected_marker {
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
            if marker != RC_32 {
                return Err(LevelError::InvalidObjectArrayMarker {
                    entry: "lvars",
                    expected: RC_32,
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
        if marker != *expected_marker {
            return Err(LevelError::InvalidObjectArrayMarker {
                entry: entry_name,
                expected: *expected_marker,
                actual: marker,
            });
        }

        for object in &mut objects {
            let value = match marker {
                RC_8 => i32::from(file.read_u8().map_err(|source| LevelError::Read {
                    path: path.to_path_buf(),
                    source,
                })?),
                RC_16 => i32::from(file.read_u16::<LittleEndian>().map_err(|source| {
                    LevelError::Read {
                        path: path.to_path_buf(),
                        source,
                    }
                })?),
                RC_32 => file
                    .read_i32::<LittleEndian>()
                    .map_err(|source| LevelError::Read {
                        path: path.to_path_buf(),
                        source,
                    })?,
                _ => 0,
            };
            object[var_index] = value;
        }
    }

    Ok(objects)
}

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
    if marker != RC_32 {
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
