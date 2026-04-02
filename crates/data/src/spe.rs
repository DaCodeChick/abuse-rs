use std::path::Path;

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
    pub spec_type: u8,
    pub name: String,
    pub size: u32,
    pub offset: u32,
}

#[derive(Debug, Clone)]
pub struct SpeDirectory {
    pub entries: Vec<SpeEntry>,
}

impl SpeDirectory {
    pub fn open(_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        anyhow::bail!("not implemented: .spe parser bootstrap pending")
    }
}
