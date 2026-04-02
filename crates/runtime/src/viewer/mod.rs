//! Viewer support modules used by the debug/playback Bevy application.
//!
//! This namespace keeps non-trivial viewer logic out of the binary entrypoint and
//! groups related concerns (camera, HUD, asset decoding, object rendering, and
//! object-driven audio).

pub mod assets;
pub mod audio;
pub mod camera;
pub mod hud;
pub mod object_render;
pub mod scene;
