use std::path::Path;

#[derive(Debug, Clone)]
pub struct LevelData {
    pub name: String,
    pub fg_width: u32,
    pub fg_height: u32,
    pub bg_width: u32,
    pub bg_height: u32,
}

impl LevelData {
    pub fn open(_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        anyhow::bail!("not implemented: level loader bootstrap pending")
    }
}
