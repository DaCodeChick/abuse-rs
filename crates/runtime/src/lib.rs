//! Runtime support for the Abuse (1997) port to Rust + Bevy.
//!
//! This crate provides:
//! - Legacy data format parsers (`.spe`, level sections, Lisp scripts)
//! - Viewer runtime modules (asset loading, camera, HUD, object rendering, audio)
//! - Bevy plugin groups for bootstrapping the runtime environment
//!
//! The runtime follows a compatibility-first approach: it loads and interprets
//! original Abuse data formats before introducing new systems.

use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;

pub mod data;
pub mod viewer;

/// Compatibility mode for legacy data parsing.
///
/// - `Strict`: Reject malformed or unexpected data.
/// - `Lenient`: Attempt to recover from quirks and legacy edge cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityMode {
    Strict,
    Lenient,
}

/// Plugin group providing core runtime functionality.
pub struct AbuseRuntimePlugins;

impl PluginGroup for AbuseRuntimePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add(BootstrapPlugin)
    }
}

/// Internal plugin for runtime bootstrap logging.
pub struct BootstrapPlugin;

impl Plugin for BootstrapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, bootstrap_log_system);
    }
}

fn bootstrap_log_system() {
    info!("abuse-rs runtime bootstrap initialized");
}
