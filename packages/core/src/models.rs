use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub group: String,
    pub logo: Option<String>,
    pub url: String,
    pub tvg_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ParsedPlaylist {
    pub channels: Vec<Channel>,
    pub groups: Vec<String>,
}
