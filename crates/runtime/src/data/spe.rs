use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use byteorder::{LittleEndian, ReadBytesExt};
use thiserror::Error;

pub const SPEC_SIGNATURE: &[u8; 8] = b"SPEC1.0\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SpecType {
    Invalid = 0,
    ColorTable = 1,
    Palette = 2,
    Image = 4,
    ForeTile = 5,
    BackTile = 6,
    Character = 7,
    MorphPoints8 = 8,
    MorphPoints16 = 9,
    GrueObjs = 10,
    ExternSfx = 11,
    DmxMus = 12,
    PatchedMorph = 13,
    NormalFile = 14,
    Compress1File = 15,
    VectorImage = 16,
    LightList = 17,
    GrueFgMap = 18,
    GrueBgMap = 19,
    DataArray = 20,
    Character2 = 21,
    Particle = 22,
    ExternalLcache = 23,
}

#[derive(Debug, Clone)]
pub struct SpeEntry {
    pub spec_type: SpecType,
    pub name: String,
    pub flags: u8,
    pub size: u32,
    pub offset: u32,
}

#[derive(Debug, Clone)]
pub struct SpeDirectory {
    pub entries: Vec<SpeEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeParseMode {
    Strict,
    Lenient,
}

#[derive(Debug, Error)]
pub enum SpeError {
    #[error("failed to open spe file at {path}: {source}")]
    Open {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read spe file at {path}: {source}")]
    Read {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid spe signature: expected {expected:?}, got {actual:?}")]
    BadSignature { expected: [u8; 8], actual: [u8; 8] },
    #[error("invalid entry name length 0")]
    InvalidNameLength,
    #[error("entry at index {index} has invalid type {spec_type}")]
    InvalidType { index: usize, spec_type: u8 },
    #[error("entry at index {index} has non-utf8 name bytes")]
    InvalidNameEncoding { index: usize },
    #[error("entry at index {index} has impossible offset/size combination")]
    InvalidEntryBounds { index: usize },
}

impl SpecType {
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for SpecType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Invalid),
            1 => Ok(Self::ColorTable),
            2 => Ok(Self::Palette),
            4 => Ok(Self::Image),
            5 => Ok(Self::ForeTile),
            6 => Ok(Self::BackTile),
            7 => Ok(Self::Character),
            8 => Ok(Self::MorphPoints8),
            9 => Ok(Self::MorphPoints16),
            10 => Ok(Self::GrueObjs),
            11 => Ok(Self::ExternSfx),
            12 => Ok(Self::DmxMus),
            13 => Ok(Self::PatchedMorph),
            14 => Ok(Self::NormalFile),
            15 => Ok(Self::Compress1File),
            16 => Ok(Self::VectorImage),
            17 => Ok(Self::LightList),
            18 => Ok(Self::GrueFgMap),
            19 => Ok(Self::GrueBgMap),
            20 => Ok(Self::DataArray),
            21 => Ok(Self::Character2),
            22 => Ok(Self::Particle),
            23 => Ok(Self::ExternalLcache),
            _ => Err(()),
        }
    }
}

impl SpeDirectory {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, SpeError> {
        Self::open_with_mode(path, SpeParseMode::Strict)
    }

    pub fn open_lenient(path: impl AsRef<Path>) -> Result<Self, SpeError> {
        Self::open_with_mode(path, SpeParseMode::Lenient)
    }

    pub fn open_with_mode(path: impl AsRef<Path>, mode: SpeParseMode) -> Result<Self, SpeError> {
        let path_ref = path.as_ref();
        let mut file = File::open(path_ref).map_err(|source| SpeError::Open {
            path: path_ref.to_path_buf(),
            source,
        })?;

        let mut signature = [0_u8; 8];
        file.read_exact(&mut signature)
            .map_err(|source| SpeError::Read {
                path: path_ref.to_path_buf(),
                source,
            })?;

        if &signature != SPEC_SIGNATURE {
            return Err(SpeError::BadSignature {
                expected: *SPEC_SIGNATURE,
                actual: signature,
            });
        }

        let total = file
            .read_u16::<LittleEndian>()
            .map_err(|source| SpeError::Read {
                path: path_ref.to_path_buf(),
                source,
            })? as usize;

        let mut entries = Vec::with_capacity(total);
        let file_size = file
            .seek(SeekFrom::End(0))
            .map_err(|source| SpeError::Read {
                path: path_ref.to_path_buf(),
                source,
            })?;
        file.seek(SeekFrom::Start(10))
            .map_err(|source| SpeError::Read {
                path: path_ref.to_path_buf(),
                source,
            })?;

        for index in 0..total {
            let raw_type = file.read_u8().map_err(|source| SpeError::Read {
                path: path_ref.to_path_buf(),
                source,
            })?;
            let spec_type = match SpecType::try_from(raw_type) {
                Ok(value) => value,
                Err(()) => {
                    if mode == SpeParseMode::Strict {
                        return Err(SpeError::InvalidType {
                            index,
                            spec_type: raw_type,
                        });
                    }
                    SpecType::Invalid
                }
            };

            let name_len = file.read_u8().map_err(|source| SpeError::Read {
                path: path_ref.to_path_buf(),
                source,
            })?;
            if name_len == 0 {
                return Err(SpeError::InvalidNameLength);
            }

            let mut name_buf = vec![0_u8; usize::from(name_len)];
            file.read_exact(&mut name_buf)
                .map_err(|source| SpeError::Read {
                    path: path_ref.to_path_buf(),
                    source,
                })?;

            let flags = file.read_u8().map_err(|source| SpeError::Read {
                path: path_ref.to_path_buf(),
                source,
            })?;
            let size = file
                .read_u32::<LittleEndian>()
                .map_err(|source| SpeError::Read {
                    path: path_ref.to_path_buf(),
                    source,
                })?;
            let offset = file
                .read_u32::<LittleEndian>()
                .map_err(|source| SpeError::Read {
                    path: path_ref.to_path_buf(),
                    source,
                })?;

            let name = match String::from_utf8(name_buf) {
                Ok(mut text) => {
                    if text.ends_with('\0') {
                        text.pop();
                    }
                    text
                }
                Err(_) => {
                    if mode == SpeParseMode::Strict {
                        return Err(SpeError::InvalidNameEncoding { index });
                    }
                    String::new()
                }
            };

            let end = u64::from(offset) + u64::from(size);
            if end > file_size {
                return Err(SpeError::InvalidEntryBounds { index });
            }

            entries.push(SpeEntry {
                spec_type,
                name,
                flags,
                size,
                offset,
            });
        }

        Ok(Self { entries })
    }

    pub fn find_by_name(&self, name: &str) -> Option<&SpeEntry> {
        self.entries.iter().find(|entry| entry.name == name)
    }

    pub fn find_by_type(&self, spec_type: SpecType) -> Option<&SpeEntry> {
        self.entries
            .iter()
            .find(|entry| entry.spec_type == spec_type)
    }

    pub fn entries_of_type(&self, spec_type: SpecType) -> impl Iterator<Item = &SpeEntry> {
        self.entries
            .iter()
            .filter(move |entry| entry.spec_type == spec_type)
    }
}

#[cfg(test)]
mod tests {
    use super::SpecType;

    #[test]
    fn spec_type_roundtrip_known_values() {
        for value in [0_u8, 1, 2, 4, 14, 23] {
            let parsed = SpecType::try_from(value).expect("known type should parse");
            assert_eq!(parsed.as_u8(), value);
        }
    }

    #[test]
    fn spec_type_rejects_unknown_values() {
        assert!(SpecType::try_from(3).is_err());
        assert!(SpecType::try_from(255).is_err());
    }
}
