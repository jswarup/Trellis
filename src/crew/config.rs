// src/crew/config.rs
use crate::silo::buff::Buff;

#[derive(Clone)]
pub struct CrewLinkConfig {
    pub source_node_id: u32,
    pub destination_node_ids: Buff<u32>,
}
