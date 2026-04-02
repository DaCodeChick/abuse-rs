//! Legacy viewer asset loading and decoding helpers.
//!
//! This module handles palette and indexed image decoding from Abuse SPE archives,
//! then converts them into Bevy `Image` textures for viewer rendering.

use std::collections::HashMap;
use std::{fs::File, io::Read, io::Seek, io::SeekFrom, path::Path};

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use byteorder::{LittleEndian, ReadBytesExt};

use crate::data::spe::{SpeDirectory, SpecType};
use crate::viewer::constants::{BG_TILE_SPE_FILES, FG_TILE_SPE_FILES, OBJECT_SPE_FILES};

/// In-memory FG/BG tile texture lookup tables with discovered tile dimensions.
#[derive(Debug, Clone)]
pub struct LegacyTileSet {
    /// Foreground tiles by tile id.
    pub fg_tiles: HashMap<u16, Handle<Image>>,
    /// Background tiles by tile id.
    pub bg_tiles: HashMap<u16, Handle<Image>>,
    /// Foreground tile dimensions.
    pub fg_tile_size: Vec2,
    /// Background tile dimensions.
    pub bg_tile_size: Vec2,
}

/// Lookup map for object sprites keyed by `(archive_path, entry_name)`.
#[derive(Debug, Clone)]
pub struct ObjectSpriteLibrary {
    /// Texture handles indexed by lowercase key pair.
    pub sprites: HashMap<(String, String), Handle<Image>>,
}

impl ObjectSpriteLibrary {
    /// Returns a sprite handle for a given archive path and entry name.
    pub fn get(&self, spe_path: &str, entry_name: &str) -> Option<Handle<Image>> {
        self.sprites
            .get(&(
                spe_path.to_ascii_lowercase(),
                entry_name.to_ascii_lowercase(),
            ))
            .cloned()
    }
}

/// Loads foreground/background tile textures from known legacy archives.
pub fn load_legacy_tile_set(
    level_path: &Path,
    images: &mut Assets<Image>,
    fallback_tile_size: f32,
) -> Result<LegacyTileSet, String> {
    let data_root = derive_data_root(level_path)
        .ok_or_else(|| format!("could not derive data root from {}", level_path.display()))?;

    let palette = read_palette(&data_root.join("art/back/backgrnd.spe"))?;

    let mut fg_tiles = HashMap::new();
    let mut bg_tiles = HashMap::new();
    let mut fg_tile_size = Vec2::new(fallback_tile_size, fallback_tile_size);
    let mut bg_tile_size = Vec2::new(fallback_tile_size, fallback_tile_size);

    for rel_path in FG_TILE_SPE_FILES {
        let path = data_root.join(rel_path);
        if !path.exists() {
            continue;
        }
        let loaded = read_tile_images_from_spe(&path, SpecType::ForeTile, &palette)?;
        for (tile_id, rgba, width, height) in loaded {
            fg_tile_size = Vec2::new(width as f32, height as f32);
            let texture = image_from_rgba(width, height, rgba);
            fg_tiles.insert(tile_id, images.add(texture));
        }
    }

    for rel_path in BG_TILE_SPE_FILES {
        let path = data_root.join(rel_path);
        if !path.exists() {
            continue;
        }
        let loaded = read_tile_images_from_spe(&path, SpecType::BackTile, &palette)?;
        for (tile_id, rgba, width, height) in loaded {
            bg_tile_size = Vec2::new(width as f32, height as f32);
            let texture = image_from_rgba(width, height, rgba);
            bg_tiles.insert(tile_id, images.add(texture));
        }
    }

    if fg_tiles.is_empty() && bg_tiles.is_empty() {
        return Err("no foreground/background tile textures found".to_string());
    }

    Ok(LegacyTileSet {
        fg_tiles,
        bg_tiles,
        fg_tile_size,
        bg_tile_size,
    })
}

/// Loads mapped object sprite textures from known object archives.
pub fn load_object_sprite_library(
    level_path: &Path,
    images: &mut Assets<Image>,
) -> Result<ObjectSpriteLibrary, String> {
    let data_root = derive_data_root(level_path)
        .ok_or_else(|| format!("could not derive data root from {}", level_path.display()))?;
    let fallback_palette = read_palette(&data_root.join("art/back/backgrnd.spe"))?;

    let mut sprites = HashMap::new();
    for rel in OBJECT_SPE_FILES {
        let path = data_root.join(rel);
        if !path.exists() {
            continue;
        }

        let directory = SpeDirectory::open_lenient(&path).map_err(|err| err.to_string())?;
        let palette = read_palette(&path).unwrap_or_else(|_| fallback_palette.clone());
        let mut file = File::open(&path).map_err(|err| err.to_string())?;

        for entry in directory.entries.iter().filter(|entry| {
            matches!(
                entry.spec_type,
                SpecType::Image | SpecType::Character | SpecType::Character2
            )
        }) {
            let (rgba, width, height) = read_image_entry(&mut file, entry.offset, &palette)?;
            let handle = images.add(image_from_rgba(width, height, rgba));
            sprites.insert(
                (rel.to_ascii_lowercase(), entry.name.to_ascii_lowercase()),
                handle,
            );
        }
    }

    Ok(ObjectSpriteLibrary { sprites })
}

/// Derives the legacy data root (`.../data`) from a level path.
pub fn derive_data_root(level_path: &Path) -> Option<std::path::PathBuf> {
    level_path
        .parent()
        .and_then(|p| p.parent())
        .map(std::path::PathBuf::from)
}

/// Reads and normalizes a palette entry from an SPE archive.
pub fn read_palette(path: &Path) -> Result<Vec<[u8; 3]>, String> {
    let directory = SpeDirectory::open_lenient(path).map_err(|err| err.to_string())?;
    let palette_entry = directory
        .entries
        .iter()
        .find(|e| e.spec_type == SpecType::Palette && e.name == "palette")
        .or_else(|| {
            directory
                .entries
                .iter()
                .find(|e| e.spec_type == SpecType::Palette)
        })
        .ok_or_else(|| format!("no palette entry in {}", path.display()))?;

    let mut file = File::open(path).map_err(|err| err.to_string())?;
    file.seek(SeekFrom::Start(u64::from(palette_entry.offset)))
        .map_err(|err| err.to_string())?;

    let count = file
        .read_u16::<LittleEndian>()
        .map_err(|err| err.to_string())? as usize;

    let mut colors = Vec::with_capacity(count);
    let mut max_component = 0_u8;
    for _ in 0..count {
        let r = file.read_u8().map_err(|err| err.to_string())?;
        let g = file.read_u8().map_err(|err| err.to_string())?;
        let b = file.read_u8().map_err(|err| err.to_string())?;
        max_component = max_component.max(r).max(g).max(b);
        colors.push([r, g, b]);
    }

    if max_component <= 63 {
        for color in &mut colors {
            color[0] = color[0].saturating_mul(4);
            color[1] = color[1].saturating_mul(4);
            color[2] = color[2].saturating_mul(4);
        }
    }

    Ok(colors)
}

/// A tile image with its ID, RGBA data, width, and height.
type TileImage = (u16, Vec<u8>, u32, u32);

/// Reads all tile images of a specific type from an SPE archive.
pub fn read_tile_images_from_spe(
    path: &Path,
    tile_type: SpecType,
    palette: &[[u8; 3]],
) -> Result<Vec<TileImage>, String> {
    let directory = SpeDirectory::open_lenient(path).map_err(|err| err.to_string())?;
    let mut file = File::open(path).map_err(|err| err.to_string())?;

    let mut out = Vec::new();
    for entry in directory
        .entries
        .iter()
        .filter(|entry| entry.spec_type == tile_type)
    {
        let Ok(tile_id) = entry.name.parse::<u16>() else {
            continue;
        };

        let (rgba, width, height) = read_image_entry(&mut file, entry.offset, palette)?;
        out.push((tile_id, rgba, width, height));
    }

    Ok(out)
}

/// Reads one indexed legacy image payload and converts it to RGBA bytes.
pub fn read_image_entry(
    file: &mut File,
    offset: u32,
    palette: &[[u8; 3]],
) -> Result<(Vec<u8>, u32, u32), String> {
    file.seek(SeekFrom::Start(u64::from(offset)))
        .map_err(|err| err.to_string())?;

    let width = file
        .read_u16::<LittleEndian>()
        .map_err(|err| err.to_string())? as u32;
    let height = file
        .read_u16::<LittleEndian>()
        .map_err(|err| err.to_string())? as u32;
    let pixel_count = usize::try_from(width.saturating_mul(height))
        .map_err(|_| format!("image too large: {}x{}", width, height))?;

    let mut indexed = vec![0_u8; pixel_count];
    file.read_exact(&mut indexed)
        .map_err(|err| err.to_string())?;

    let mut rgba = vec![0_u8; pixel_count * 4];
    for (i, idx) in indexed.into_iter().enumerate() {
        let color = palette.get(idx as usize).copied().unwrap_or([0, 0, 0]);
        let base = i * 4;
        rgba[base] = color[0];
        rgba[base + 1] = color[1];
        rgba[base + 2] = color[2];
        rgba[base + 3] = if idx == 0 { 0 } else { 255 };
    }

    Ok((rgba, width, height))
}

/// Creates a Bevy image from raw RGBA bytes.
pub fn image_from_rgba(width: u32, height: u32, rgba: Vec<u8>) -> Image {
    Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
}

/// Creates a soft radial glow texture used for light overlays.
pub fn make_radial_glow_texture(size: u32) -> Image {
    let mut rgba = vec![0_u8; (size * size * 4) as usize];
    let c = size as f32 * 0.5;
    let max_r = c - 1.0;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - c;
            let dy = y as f32 - c;
            let dist = (dx * dx + dy * dy).sqrt();
            let t = (1.0 - dist / max_r).clamp(0.0, 1.0);
            let a = (t * t * 255.0) as u8;
            let i = ((y * size + x) * 4) as usize;
            rgba[i] = 255;
            rgba[i + 1] = 255;
            rgba[i + 2] = 255;
            rgba[i + 3] = a;
        }
    }

    image_from_rgba(size, size, rgba)
}
