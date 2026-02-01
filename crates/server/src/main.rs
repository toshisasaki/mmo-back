use bevy_app::{App, ScheduleRunnerPlugin, Update};
use bevy_ecs::prelude::*;
use bevy_time::TimePlugin;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::broadcast;
use crossbeam_channel::Receiver;
use shared::{ClientCommand, ProjectileState, PlayerState as SharedPlayerState, ServerEvent};
use glam::Vec2;

mod ws;

use ws::GamePacket;

#[derive(Resource)]
struct NetworkReceiver(Receiver<GamePacket>);

#[derive(Resource)]
struct NetworkSender(broadcast::Sender<String>);

#[derive(Component)]
struct Player {
    id: u32,
    position: Vec2,
}

#[derive(Component)]
struct Health {
    current: f32,
    max: f32,
}

#[derive(Component)]
struct Projectile {
    position: Vec2,
    velocity: Vec2,
    owner_id: u32,
    lifetime: f32,
}

fn main() {
    let rt = Runtime::new().unwrap();

    let (tx, rx) = crossbeam_channel::unbounded();
    let (broadcast_tx, _) = broadcast::channel(100);
    // Channel to signal Bevy to exit
    let (shutdown_tx, shutdown_rx) = crossbeam_channel::bounded(1);

    let b_tx = broadcast_tx.clone();
    rt.spawn(async move {
        ws::start_ws_server(tx, b_tx, shutdown_tx).await;
    });

    App::new()
        .add_plugins(TimePlugin)
        .add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            1.0 / 30.0,
        )))
        .insert_resource(NetworkReceiver(rx))
        .insert_resource(NetworkSender(broadcast_tx))
        .insert_resource(ShutdownReceiver(shutdown_rx))
        .insert_resource(AsyncRuntime(rt))
        .add_systems(Update, (handle_packets, move_projectiles, handle_collisions, broadcast_state, check_shutdown))
        .run();
}

#[derive(Resource)]
pub struct AsyncRuntime(pub Runtime);

#[derive(Resource)]
struct ShutdownReceiver(Receiver<()>);

fn check_shutdown(
    receiver: Res<ShutdownReceiver>,
    mut app_exit_events: EventWriter<bevy_app::AppExit>,
) {
    if receiver.0.try_recv().is_ok() {
        println!("Bevy: Shutdown signal received. Exiting...");
        app_exit_events.send(bevy_app::AppExit::Success);
    }
}

fn handle_packets(
    mut commands: Commands,
    receiver: Res<NetworkReceiver>,
    mut players: Query<(Entity, &mut Player)>, 
) {
    while let Ok(packet) = receiver.0.try_recv() {
        match packet {
            GamePacket::PlayerJoin { id } => {
                println!("Spawning player {}", id);
                commands.spawn((
                    Player {
                        id,
                        position: Vec2::new(400.0, 300.0), // Center
                    },
                    Health {
                        current: 100.0,
                        max: 100.0,
                    }
                ));
            }

            GamePacket::PlayerLeave { id } => {
                println!("Despawning player {}", id);
                for (entity, player) in players.iter() {
                    if player.id == id {
                        commands.entity(entity).despawn();
                    }
                }
            }
            
            GamePacket::ClientCommand { id, cmd } => {
                match cmd {
                    ClientCommand::Move { dir } => {
                        for (_, mut player) in players.iter_mut() {
                            if player.id == id {
                                let speed = 5.0; 
                                player.position += dir * speed;
                            }
                        }
                    }
                    ClientCommand::CastSpell { target } => {
                         if let Some((_, player)) = players.iter().find(|(_, p)| p.id == id) {
                             let caster_pos = player.position;
                             let dir = (target - caster_pos).normalize_or_zero();
                             
                             if dir != Vec2::ZERO {
                                 commands.spawn(Projectile {
                                     position: caster_pos + dir * 20.0, 
                                     velocity: dir * 10.0, 
                                     owner_id: id,
                                     lifetime: 60.0, 
                                 });
                             }
                         }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn move_projectiles(
    mut commands: Commands,
    mut projectiles: Query<(Entity, &mut Projectile)>,
) {
    for (entity, mut proj) in projectiles.iter_mut() {
        let velocity = proj.velocity;
        proj.position += velocity;
        proj.lifetime -= 1.0;
        if proj.lifetime <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn handle_collisions(
    mut commands: Commands,
    projectiles: Query<(Entity, &Projectile)>,
    mut players: Query<(Entity, &Player, &mut Health)>,
) {
    for (p_entity, proj) in projectiles.iter() {
        for (_pl_entity, player, mut health) in players.iter_mut() {
            if player.id != proj.owner_id {
                let dist = proj.position.distance(player.position);
                if dist < 20.0 { // Hit box size
                    health.current -= 10.0;
                    println!("Player {} hit! HP: {}", player.id, health.current);
                    commands.entity(p_entity).despawn(); // Destroy projectile
                    
                    if health.current <= 0.0 {
                         // Respawn logic? Or just reset?
                         health.current = 100.0;
                         // Ideally send "PlayerDied" event or respawn them elsewhere
                    }
                }
            }
        }
    }
}



fn broadcast_state(
    sender: Res<NetworkSender>,
    players: Query<(&Player, &Health)>,
    projectiles: Query<&Projectile>,
) {
    let player_states: Vec<SharedPlayerState> = players.iter().map(|(p, h)| SharedPlayerState {
        id: p.id,
        position: p.position,
        health: h.current,
        max_health: h.max,
    }).collect();

    let projectile_states: Vec<ProjectileState> = projectiles.iter().map(|p| ProjectileState {
        id: 0, // We didn't give projectiles UIDs in Component, maybe skip for now or generate temp?
               // Actually we need IDs if we want to interpolate them correctly. 
               // For now, let's just send them without ID or use random/entity bits?
               // Let's just use 0 for now as frontend might not match them yet.
        position: p.position,
    }).collect();

    if player_states.is_empty() && projectile_states.is_empty() {
        return;
    }

    let event = ServerEvent::Snapshot {
        tick: 0, 
        players: player_states,
        projectiles: projectile_states,
    };

    if let Ok(msg) = serde_json::to_string(&event) {
        let _ = sender.0.send(msg);
    }
}
