//! Parser incremental M3U: processa linha a linha e emite um `ChannelItem`
//! quando #EXTINF + URL estão completos. Não acumula tudo na memória.
//!
//! Performance: uma única passagem, sem alocar a playlist inteira.

use std::collections::HashMap;

use crate::model::{ChannelItem, ExtInfMeta};

/// Parser stateful: a cada linha retorna `Some(ChannelItem)` quando
/// formar um item completo (#EXTINF seguido de linha de URL).
#[derive(Debug, Default)]
pub struct M3uIncrementalParser {
    saw_extm3u: bool,
    pending_meta: Option<ExtInfMeta>,
    pending_name: Option<String>,
    items_emitted: u64,
}

impl M3uIncrementalParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Número de itens emitidos até agora (para progresso).
    #[inline]
    pub fn items_emitted(&self) -> u64 {
        self.items_emitted
    }

    /// Processa uma linha. Retorna `Some(ChannelItem)` quando a linha
    /// é a URL do canal (linha seguinte ao #EXTINF).
    pub fn feed_line(&mut self, line: &str) -> Option<ChannelItem> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }

        if line == "#EXTM3U" {
            self.saw_extm3u = true;
            return None;
        }

        if let Some(rest) = line.strip_prefix("#EXTINF:") {
            let (meta, name) = split_extinf(rest);
            let attrs = parse_attributes(meta);
            self.pending_meta = Some(ExtInfMeta {
                tvg_id: attrs.get("tvg-id").cloned(),
                tvg_name: attrs.get("tvg-name").cloned(),
                tvg_logo: attrs.get("tvg-logo").cloned(),
                group_title: attrs.get("group-title").cloned(),
            });
            self.pending_name = Some(name.to_string());
            return None;
        }

        if line.starts_with('#') {
            return None;
        }

        let url = line;
        let (meta, name) = match (self.pending_meta.take(), self.pending_name.take()) {
            (Some(m), Some(n)) => (m, n),
            _ => return None,
        };

        let group = meta
            .group_title
            .unwrap_or_else(|| "Sem grupo".to_string());
        let id = channel_id(&name, url);

        self.items_emitted += 1;

        Some(ChannelItem {
            id,
            name,
            group,
            logo: meta.tvg_logo,
            url: url.to_string(),
            tvg_id: meta.tvg_id,
            tvg_name: meta.tvg_name,
        })
    }

    /// Indica se já viu o cabeçalho #EXTM3U (útil para validação opcional).
    #[inline]
    pub fn saw_header(&self) -> bool {
        self.saw_extm3u
    }
}

fn split_extinf(rest: &str) -> (&str, &str) {
    match rest.rsplit_once(',') {
        Some((meta, name)) => (meta.trim(), name.trim()),
        None => (rest.trim(), "Canal sem nome"),
    }
}

fn parse_attributes(meta: &str) -> HashMap<String, String> {
    let mut attrs = HashMap::new();
    let mut chars = meta.chars().peekable();

    while chars.peek().is_some() {
        while chars
            .peek()
            .map(|c| c.is_whitespace() || *c == '-' || *c == '0' || *c == '1' || *c == ':' || *c == ',')
            == Some(true)
        {
            chars.next();
        }

        let mut key = String::new();
        while let Some(&c) = chars.peek() {
            if c == '=' || c.is_whitespace() || c == ',' {
                break;
            }
            key.push(c);
            chars.next();
        }

        while chars.peek() == Some(&'=') {
            chars.next();
        }
        while chars.peek().map(|c| c.is_whitespace()) == Some(true) {
            chars.next();
        }

        if key.is_empty() {
            break;
        }

        let value = match chars.peek() {
            Some('"') => {
                chars.next();
                let mut v = String::new();
                for c in chars.by_ref() {
                    if c == '"' {
                        break;
                    }
                    v.push(c);
                }
                v
            }
            Some(_) => {
                let mut v = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_whitespace() || c == ',' {
                        break;
                    }
                    v.push(c);
                    chars.next();
                }
                v
            }
            None => String::new(),
        };

        if !value.is_empty() {
            attrs.insert(key, value);
        }
    }

    attrs
}

fn channel_id(name: &str, url: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    name.hash(&mut hasher);
    url.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_item_after_extinf_and_url() {
        let mut p = M3uIncrementalParser::new();
        assert!(p.feed_line("#EXTM3U").is_none());
        assert!(p.feed_line(r#"#EXTINF:-1 tvg-id="a" group-title="G",Canal A"#).is_none());
        let item = p.feed_line("https://example.com/a.m3u8").expect("one item");
        assert_eq!(item.name, "Canal A");
        assert_eq!(item.group, "G");
        assert_eq!(item.url, "https://example.com/a.m3u8");
        assert_eq!(item.tvg_id.as_deref(), Some("a"));
    }
}
