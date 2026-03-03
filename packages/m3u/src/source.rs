//! Fontes de playlist M3U: arquivo local (std::fs + BufReader) e URL remota
//! (reqwest bytes_stream). Emitem itens e progresso via mpsc para não bloquear a UI.

use std::io::{BufRead, BufReader};
use std::path::Path;

use tokio::sync::mpsc;

use crate::model::ChannelItem;
use crate::parser::M3uIncrementalParser;

/// Progresso da importação: bytes lidos e itens já parseados.
#[derive(Debug, Clone)]
pub struct Progress {
    pub bytes_read: u64,
    pub items_processed: u64,
}

/// Headers e auth para requisição remota.
#[derive(Debug, Clone, Default)]
pub struct RequestOptions {
    pub user_agent: Option<String>,
    pub basic_auth: Option<(String, String)>,
    pub bearer_token: Option<String>,
    pub headers: Vec<(String, String)>,
}

/// Carrega M3U de arquivo local em streaming: lê com BufReader por linhas,
/// parseia incrementalmente e envia cada item pelo canal. Não carrega o arquivo
/// inteiro na RAM.
///
/// Retorna (receiver de itens, receiver de progresso). O sender é fechado
/// quando a leitura termina. Progresso é enviado a cada item emitido.
pub fn load_m3u_from_file(
    path: impl AsRef<Path>,
) -> Result<
    (
        mpsc::Receiver<ChannelItem>,
        mpsc::Receiver<Progress>,
    ),
    std::io::Error,
> {
    let path = path.as_ref().to_path_buf();

    let (tx, rx) = mpsc::channel(256);
    let (tx_progress, rx_progress) = mpsc::channel(32);

    std::thread::spawn(move || {
        let file = match std::fs::File::open(&path) {
            Ok(f) => f,
            Err(e) => {
                drop(tx);
                drop(tx_progress);
                return;
            }
        };

        let mut reader = BufReader::new(file);
        let mut parser = M3uIncrementalParser::new();
        let mut line = String::new();
        let mut bytes_read: u64 = 0;

        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(n) => {
                    bytes_read += n as u64;
                    if let Some(item) = parser.feed_line(&line) {
                        if tx.blocking_send(item).is_err() {
                            break;
                        }
                        let _ = tx_progress.blocking_send(Progress {
                            bytes_read,
                            items_processed: parser.items_emitted(),
                        });
                    }
                }
                Err(_) => break,
            }
        }

        drop(tx);
        drop(tx_progress);
    });

    Ok((rx, rx_progress))
}

/// Carrega M3U de URL em streaming: usa reqwest::get().bytes_stream(),
/// converte bytes em linhas incrementalmente e emite itens pelo canal.
/// Suporta Basic Auth, Bearer token e headers customizados.
///
/// Retorna (receiver de itens, receiver de progresso). A tarefa de rede
/// roda em background; a UI consome pelos receivers sem bloquear.
pub async fn load_m3u_from_url(
    url: &str,
    options: RequestOptions,
) -> Result<
    (
        mpsc::Receiver<ChannelItem>,
        mpsc::Receiver<Progress>,
    ),
    Box<dyn std::error::Error + Send + Sync>,
> {
    let url = url.to_string();

    let (tx, rx) = mpsc::channel(256);
    let (tx_progress, rx_progress) = mpsc::channel(32);

    let client = reqwest::Client::new();
    let mut req = client.get(&url);

    if let Some(ua) = options.user_agent {
        req = req.header("User-Agent", ua);
    }
    if let Some((user, pass)) = options.basic_auth {
        req = req.basic_auth(user, Some(pass));
    }
    if let Some(token) = options.bearer_token {
        req = req.bearer_auth(token);
    }
    for (k, v) in options.headers {
        req = req.header(k, v);
    }

    let stream = req.send().await?.error_for_status()?.bytes_stream();

    tokio::spawn(async move {
        use futures_util::StreamExt;

        let mut parser = M3uIncrementalParser::new();
        let mut line_buf = Vec::<u8>::new();
        let mut bytes_read: u64 = 0;

        let mut stream = stream;

        while let Some(chunk_result) = stream.next().await {
            let chunk = match chunk_result {
                Ok(c) => c,
                Err(_) => break,
            };

            bytes_read += chunk.len() as u64;

            for byte in chunk.iter().copied() {
                if byte == b'\n' {
                    if !line_buf.is_empty() {
                        let line = String::from_utf8_lossy(&line_buf);
                        if let Some(item) = parser.feed_line(&line) {
                            if tx.send(item).await.is_err() {
                                return;
                            }
                            let _ = tx_progress
                                .send(Progress {
                                    bytes_read,
                                    items_processed: parser.items_emitted(),
                                })
                                .await;
                        }
                        line_buf.clear();
                    }
                } else if byte != b'\r' {
                    line_buf.push(byte);
                }
            }
        }

        if !line_buf.is_empty() {
            let line = String::from_utf8_lossy(&line_buf);
            if let Some(item) = parser.feed_line(&line) {
                let _ = tx.send(item).await;
                let _ = tx_progress
                    .send(Progress {
                        bytes_read,
                        items_processed: parser.items_emitted(),
                    })
                    .await;
            }
        }

        drop(tx);
        drop(tx_progress);
    });

    Ok((rx, rx_progress))
}
