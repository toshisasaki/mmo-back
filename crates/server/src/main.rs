use bevy_app::{App, ScheduleRunnerPlugin, Startup, Update};
use bevy_ecs::prelude::*;
use bevy_time::TimePlugin;
use std::time::Duration;
use tokio::runtime::Runtime;

mod net;
mod ws;

fn main() {
    // 1. Initialize Tokio Runtime for async networking
    let rt = Runtime::new().unwrap();

    // 2. Initialize Bevy App for simulation
    // Use ScheduleRunnerPlugin for headless fixed-tick loop
    App::new()
        .add_plugins(TimePlugin)
        .add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            1.0 / 30.0,
        ))) // 30 Hz
        .add_plugins(net::NetworkPlugin)
        .add_systems(Startup, setup_world)
        .add_systems(Update, tick)
        .insert_resource(AsyncRuntime(rt))
        .run();
}

#[derive(Resource)]
pub struct AsyncRuntime(pub Runtime);

fn setup_world(mut commands: Commands) {
    println!("Server starting up...");
    // Future: Spawn initial entities or resources
}

fn tick() {
    // This runs every 1/30th of a second
    // Logic: Process queued inputs -> Run Physics/GameLogic -> Broadcast Snapshot
}
