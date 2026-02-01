use glam::Vec2;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientCommand {
    Join,
    Move { dir: Vec2 },
    CastSpell { target: Vec2 },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlayerState {
    pub id: u32,
    pub position: Vec2,
    pub health: f32,
    pub max_health: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectileState {
    pub id: u32,
    pub position: Vec2,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerEvent {
    PlayerJoined { id: u32, position: Vec2 },
    PlayerLeft { id: u32 },
    Snapshot {
        tick: u64,
        players: Vec<PlayerState>,
        projectiles: Vec<ProjectileState>,
    },
}
