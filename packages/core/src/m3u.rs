use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use crate::models::{Channel, ParsedPlaylist};

/// Parser stateful para M3U que processa uma linha por vez (streaming).
pub struct M3uStreamParser {
    saw_header: bool,
    pending: Option<PendingChannel>,
    channels: Vec<Channel>,
    groups: Vec<String>,
    seen_groups: HashSet<String>,
}

impl Default for M3uStreamParser {
    fn default() -> Self {
        Self {
            saw_header: false,
            pending: None,
            channels: Vec::new(),
            groups: Vec::new(),
            seen_groups: HashSet::new(),
        }
    }
}

impl M3uStreamParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Processa uma linha do M3U (linha completa, sem newline).
    pub fn feed_line(&mut self, line: &str) {
        let line = line.trim();

        if line.is_empty() {
            return;
        }

        if line == "#EXTM3U" {
            self.saw_header = true;
            return;
        }

        if let Some(rest) = line.strip_prefix("#EXTINF:") {
            let (meta, name) = split_extinf(rest);
            let attrs = parse_attributes(meta);
            self.pending = Some(PendingChannel {
                name: name.to_string(),
                group: attrs
                    .get("group-title")
                    .cloned()
                    .unwrap_or_else(|| "Sem grupo".to_string()),
                logo: attrs.get("tvg-logo").cloned(),
                tvg_id: attrs.get("tvg-id").cloned(),
            });
            return;
        }

        if line.starts_with('#') {
            return;
        }

        if let Some(info) = self.pending.take() {
            let channel = Channel {
                id: channel_id(&info.name, line),
                name: info.name,
                group: info.group.clone(),
                logo: info.logo,
                url: line.to_string(),
                tvg_id: info.tvg_id,
            };

            if self.seen_groups.insert(channel.group.clone()) {
                self.groups.push(channel.group.clone());
            }

            self.channels.push(channel);
        }
    }

    /// Finaliza o parse e retorna a playlist ou erro de validação.
    pub fn finish(self) -> Result<ParsedPlaylist, String> {
        if !self.saw_header {
            return Err("Playlist M3U invalida: cabecalho #EXTM3U ausente.".to_string());
        }

        if self.channels.is_empty() {
            return Err("Nenhum canal encontrado na playlist.".to_string());
        }

        Ok(ParsedPlaylist {
            channels: self.channels,
            groups: self.groups,
        })
    }
}

pub fn parse_playlist(input: &str) -> Result<ParsedPlaylist, String> {
    let mut parser = M3uStreamParser::new();
    for raw_line in input.lines() {
        parser.feed_line(raw_line);
    }
    parser.finish()
}

#[derive(Debug)]
struct PendingChannel {
    name: String,
    group: String,
    logo: Option<String>,
    tvg_id: Option<String>,
}

fn split_extinf(line: &str) -> (&str, &str) {
    match line.rsplit_once(',') {
        Some((meta, name)) => (meta.trim(), name.trim()),
        None => (line.trim(), "Canal sem nome"),
    }
}

fn parse_attributes(meta: &str) -> std::collections::HashMap<String, String> {
    let mut attrs = std::collections::HashMap::new();
    let mut chars = meta.chars().peekable();

    while let Some(ch) = chars.peek() {
        if ch.is_whitespace() || *ch == '-' || *ch == '0' || *ch == '1' || *ch == ':' || *ch == ','
        {
            chars.next();
            continue;
        }

        let mut key = String::new();
        while let Some(current) = chars.peek() {
            if *current == '=' || current.is_whitespace() || *current == ',' {
                break;
            }
            key.push(*current);
            chars.next();
        }

        while let Some(current) = chars.peek() {
            if *current == '=' {
                chars.next();
                break;
            }
            if current.is_whitespace() {
                chars.next();
                continue;
            }
            break;
        }

        if key.is_empty() {
            chars.next();
            continue;
        }

        let value = match chars.peek() {
            Some('"') => {
                chars.next();
                let mut quoted = String::new();
                for current in chars.by_ref() {
                    if current == '"' {
                        break;
                    }
                    quoted.push(current);
                }
                quoted
            }
            Some(_) => {
                let mut plain = String::new();
                while let Some(current) = chars.peek() {
                    if current.is_whitespace() || *current == ',' {
                        break;
                    }
                    plain.push(*current);
                    chars.next();
                }
                plain
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
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    name.hash(&mut hasher);
    url.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::{parse_playlist, M3uStreamParser};

    #[test]
    fn stream_parser_matches_parse_playlist() {
        let input = r#"#EXTM3U
#EXTINF:-1 tvg-id="news-1" tvg-logo="https://img/news.png" group-title="Noticias",Canal News
https://example.com/news.m3u8
#EXTINF:-1 group-title="Esportes",Canal Sport
https://example.com/sport.m3u8
"#;
        let from_parse = parse_playlist(input).expect("parse_playlist should succeed");
        let mut parser = M3uStreamParser::new();
        for line in input.lines() {
            parser.feed_line(line);
        }
        let from_stream = parser.finish().expect("stream parser should succeed");
        assert_eq!(from_parse.channels.len(), from_stream.channels.len());
        assert_eq!(from_parse.groups, from_stream.groups);
        assert_eq!(from_parse.channels[0].name, from_stream.channels[0].name);
    }

    #[test]
    fn parses_channels_and_groups() {
        let input = r#"#EXTM3U
#EXTINF:-1 tvg-id="news-1" tvg-logo="https://img/news.png" group-title="Noticias",Canal News
https://example.com/news.m3u8
#EXTINF:-1 group-title="Esportes",Canal Sport
https://example.com/sport.m3u8
"#;

        let parsed = parse_playlist(input).expect("playlist should parse");
        assert_eq!(parsed.channels.len(), 2);
        assert_eq!(parsed.groups, vec!["Noticias", "Esportes"]);
        assert_eq!(parsed.channels[0].name, "Canal News");
        assert_eq!(parsed.channels[0].tvg_id.as_deref(), Some("news-1"));
        assert_eq!(
            parsed.channels[0].logo.as_deref(),
            Some("https://img/news.png")
        );
    }

    #[test]
    fn ignores_comments_and_blank_lines() {
        let input = r#"#EXTM3U

#EXTINF:-1 group-title="Variedades",Canal 1
https://example.com/one.m3u8
#EXTVLCOPT:http-user-agent=Custom
# a comment

#EXTINF:-1,Canal 2
https://example.com/two.m3u8
"#;

        let parsed = parse_playlist(input).expect("playlist should parse");
        assert_eq!(parsed.channels.len(), 2);
        assert_eq!(parsed.channels[1].group, "Sem grupo");
    }
}
