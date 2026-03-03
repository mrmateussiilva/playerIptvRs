//! Modelos para itens M3U e metadados EXTINF.
//!
//! Evitamos duplicar strings: a UI pode guardar `Arc<str>` ou referências
//! se precisar compartilhar com muitas entradas.

use serde::{Deserialize, Serialize};

/// Metadados extraídos de uma linha #EXTINF:-1 attr1="v1" attr2="v2",Título
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtInfMeta {
    pub tvg_id: Option<String>,
    pub tvg_name: Option<String>,
    pub tvg_logo: Option<String>,
    pub group_title: Option<String>,
}

/// Um canal/entrada da playlist M3U pronto para a UI.
/// Campos são owned para não depender do buffer de parsing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelItem {
    pub id: String,
    pub name: String,
    pub group: String,
    pub logo: Option<String>,
    pub url: String,
    pub tvg_id: Option<String>,
    pub tvg_name: Option<String>,
}

impl ChannelItem {
    #[inline]
    pub fn group_display(&self) -> &str {
        if self.group.is_empty() {
            "Sem grupo"
        } else {
            &self.group
        }
    }
}
