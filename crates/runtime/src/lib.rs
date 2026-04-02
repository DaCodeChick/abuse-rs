use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;

pub mod data;
pub mod viewer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityMode {
    Strict,
    Lenient,
}

pub struct AbuseRuntimePlugins;

impl PluginGroup for AbuseRuntimePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add(BootstrapPlugin)
    }
}

pub struct BootstrapPlugin;

impl Plugin for BootstrapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, bootstrap_log_system);
    }
}

fn bootstrap_log_system() {
    info!("abuse-rs runtime bootstrap initialized");
}
