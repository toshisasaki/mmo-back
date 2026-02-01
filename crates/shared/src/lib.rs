use glam::Vec2;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientCommand {
    Move { dir: Vec2 },
    CastSpell { id: u32 },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerEvent {
    Snapshot {
        tick: u64,
        // We'll add entity states later
    },
}
