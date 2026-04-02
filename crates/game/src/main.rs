use abuse_runtime::AbuseRuntimePlugins;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(AbuseRuntimePlugins)
        .run();
}
