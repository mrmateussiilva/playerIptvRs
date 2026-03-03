use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use crate::models::{Channel, ParsedPlaylist};

pub fn parse_playlist(input: &str) -> Result<ParsedPlaylist, String> {
    let mut channels = Vec::new();
    let mut groups = Vec::new();
    let mut seen_groups = HashSet::new();
    let mut pending = None;
    let mut saw_header = false;

    for raw_line in input.lines() {
        let line = raw_line.trim();

        if line.is_empty() {
            continue;
        }

        if line == "#EXTM3U" {
            saw_header = true;
            continue;
        }

        if let Some(rest) = line.strip_prefix("#EXTINF:") {
            let (meta, name) = split_extinf(rest);
            let attrs = parse_attributes(meta);
            pending = Some(PendingChannel {
                name: name.to_string(),
                group: attrs
                    .get("group-title")
                    .cloned()
                    .unwrap_or_else(|| "Sem grupo".to_string()),
                logo: attrs.get("tvg-logo").cloned(),
                tvg_id: attrs.get("tvg-id").cloned(),
            });
            continue;
        }

        if line.starts_with('#') {
            continue;
        }

        if let Some(info) = pending.take() {
            let channel = Channel {
                id: channel_id(&info.name, line),
                name: info.name,
                group: info.group,
                logo: info.logo,
                url: line.to_string(),
                tvg_id: info.tvg_id,
            };

            if seen_groups.insert(channel.group.clone()) {
                groups.push(channel.group.clone());
            }

            channels.push(channel);
        }
    }

    if !saw_header {
        return Err("Playlist M3U invalida: cabecalho #EXTM3U ausente.".to_string());
    }

    if channels.is_empty() {
        return Err("Nenhum canal encontrado na playlist.".to_string());
    }

    Ok(ParsedPlaylist { channels, groups })
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
    use super::parse_playlist;

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
