//! Scene construction from loaded level data.
//!
//! This module handles spawning tiles, objects, and lights as Bevy entities from
//! parsed level data and loaded assets.

use bevy::prelude::*;

use crate::data::level::{LevelData, ObjectVar};

use super::assets::{LegacyTileSet, ObjectSpriteLibrary};
use super::object_render::{object_render_adjustment, resolve_object_sprite};

/// Spawns foreground tiles for the level.
pub fn spawn_fg_tiles(
    commands: &mut Commands,
    level: &LevelData,
    tile_set: Option<&LegacyTileSet>,
    fg_tile_size: Vec2,
    fg_world_w: f32,
    fg_world_h: f32,
) {
    for row in 0..level.fg_height as usize {
        for col in 0..level.fg_width as usize {
            let idx = row * level.fg_width as usize + col;
            let tile = level.fg_tiles[idx];
            let tile_id = tile & 0x3fff;
            if tile_id == 0 {
                continue;
            }

            let x = col as f32 * fg_tile_size.x - fg_world_w * 0.5 + fg_tile_size.x * 0.5;
            let y = fg_world_h * 0.5 - row as f32 * fg_tile_size.y - fg_tile_size.y * 0.5;

            if let Some(texture) = tile_set.and_then(|set| set.fg_tiles.get(&tile_id)) {
                commands.spawn((
                    Sprite::from_image(texture.clone()),
                    Transform::from_xyz(x, y, 1.0),
                ));
            } else {
                commands.spawn((
                    Sprite::from_color(tile_color(tile_id, true), fg_tile_size),
                    Transform::from_xyz(x, y, 1.0),
                ));
            }
        }
    }
}

/// Spawns background tiles for the level.
pub fn spawn_bg_tiles(
    commands: &mut Commands,
    level: &LevelData,
    tile_set: Option<&LegacyTileSet>,
    bg_tile_size: Vec2,
    bg_world_w: f32,
    bg_world_h: f32,
) {
    for row in 0..level.bg_height as usize {
        for col in 0..level.bg_width as usize {
            let idx = row * level.bg_width as usize + col;
            let tile = level.bg_tiles[idx];
            if tile == 0 {
                continue;
            }

            let x = col as f32 * bg_tile_size.x - bg_world_w * 0.5 + bg_tile_size.x * 0.5;
            let y = bg_world_h * 0.5 - row as f32 * bg_tile_size.y - bg_tile_size.y * 0.5;

            if let Some(texture) = tile_set.and_then(|set| set.bg_tiles.get(&tile)) {
                commands.spawn((
                    Sprite {
                        image: texture.clone(),
                        color: Color::srgba(1.0, 1.0, 1.0, 0.55),
                        ..default()
                    },
                    Transform::from_xyz(x, y, 0.0),
                ));
            } else {
                commands.spawn((
                    Sprite::from_color(tile_color(tile, false), bg_tile_size),
                    Transform::from_xyz(x, y, 0.0),
                ));
            }
        }
    }
}

/// Spawns object sprites for the level.
pub fn spawn_objects(
    commands: &mut Commands,
    level: &LevelData,
    sprite_lib: Option<&ObjectSpriteLibrary>,
    images: &Assets<Image>,
    fg_world_w: f32,
    fg_world_h: f32,
) {
    let Some(sprite_lib) = sprite_lib else {
        return;
    };

    for object in &level.objects {
        if let Some((spe_rel, entry_name)) = resolve_object_sprite(object)
            && let Some(texture) = sprite_lib.get(spe_rel, &entry_name)
        {
            let x = object.var(ObjectVar::X).unwrap_or(0) as f32 - fg_world_w * 0.5;
            let mut y = fg_world_h * 0.5 - object.var(ObjectVar::Y).unwrap_or(0) as f32;
            if let Some(image) = images.get(&texture) {
                y += image.height() as f32 * 0.5;
            }
            let (dx, dy, z) = object_render_adjustment(object.type_name.as_deref());
            commands.spawn((
                Sprite::from_image(texture),
                Transform::from_xyz(x + dx, y + dy, z),
            ));
        }
    }
}

/// Spawns light glow entities for the level.
pub fn spawn_lights(
    commands: &mut Commands,
    level: &LevelData,
    light_glow: Handle<Image>,
    fg_world_w: f32,
    fg_world_h: f32,
) {
    for light in &level.lights {
        let x = light.x as f32 - fg_world_w * 0.5;
        let y = fg_world_h * 0.5 - light.y as f32;
        let radius = light.outer_radius.max(8) as f32;
        let tint = match light.light_type {
            1 => Color::srgba(0.92, 0.78, 0.44, 0.42),
            3 => Color::srgba(0.73, 0.42, 0.96, 0.45),
            _ => Color::srgba(0.86, 0.84, 0.78, 0.32),
        };

        commands.spawn((
            Sprite {
                image: light_glow.clone(),
                custom_size: Some(Vec2::splat(radius * 2.6)),
                color: tint,
                ..default()
            },
            Transform::from_xyz(x, y, 2.2),
        ));

        commands.spawn((
            Sprite {
                image: light_glow.clone(),
                custom_size: Some(Vec2::splat(radius * 1.2)),
                color: Color::srgba(1.0, 1.0, 1.0, 0.18),
                ..default()
            },
            Transform::from_xyz(x, y, 2.21),
        ));
    }
}

/// Returns a muted debug color for fallback tile rendering.
fn tile_color(tile: u16, foreground: bool) -> Color {
    let seed = u32::from(tile & 0x7fff);
    let tone = ((seed.wrapping_mul(37) % 120) as f32 + 70.0) / 255.0;
    if foreground {
        Color::srgb(tone * 0.85, tone, tone * 0.9)
    } else {
        Color::srgba(tone * 0.35, tone * 0.4, tone * 0.5, 0.35)
    }
}
