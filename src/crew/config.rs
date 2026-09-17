// src/crew/config.rs
#[derive( Clone, Debug, Default)]
pub struct CrewLinkConfig
{
    pub source_node_id: u32,
    pub destination_node_ids: Vec< u32>,
}
