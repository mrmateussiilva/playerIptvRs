use gloo_net::http::Request;
use iptv_core::storage::PlaylistStorage;
use iptv_core::{M3uStreamParser, ParsedPlaylist};
use js_sys::{Reflect, Uint8Array};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{ReadableStream, ReadableStreamDefaultReader, TextDecoder};
use wasm_bindgen::JsCast;

const PLAYLIST_KEY: &str = "finderbit.playlist";
const FAVORITES_KEY: &str = "finderbit.favorites";
const RECENTS_KEY: &str = "finderbit.recents";
const CONNECTION_KEY: &str = "finderbit.connection";
const SESSION_KEY: &str = "finderbit.session";

#[derive(Clone, Default)]
pub struct AppSnapshot {
    pub playlist: Option<ParsedPlaylist>,
    pub favorites: Vec<String>,
    pub recents: Vec<String>,
    pub connection: ConnectionSettings,
    pub session: ViewerSession,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct ConnectionSettings {
    pub server_url: String,
    pub username: String,
    pub password: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct ViewerSession {
    pub profile_name: String,
    pub is_logged_in: bool,
}

#[derive(Default)]
pub struct WebStorage;

#[derive(Serialize, Deserialize)]
struct StoredVec {
    items: Vec<String>,
}

impl PlaylistStorage for WebStorage {
    fn save_playlist(&self, playlist: &ParsedPlaylist) -> Result<(), String> {
        set_json(PLAYLIST_KEY, playlist)
    }

    fn load_playlist(&self) -> Result<Option<ParsedPlaylist>, String> {
        get_json(PLAYLIST_KEY)
    }

    fn save_favorites(&self, favorites: &[String]) -> Result<(), String> {
        set_json(
            FAVORITES_KEY,
            &StoredVec {
                items: favorites.to_vec(),
            },
        )
    }

    fn load_favorites(&self) -> Result<Vec<String>, String> {
        Ok(get_json::<StoredVec>(FAVORITES_KEY)?
            .map(|stored| stored.items)
            .unwrap_or_default())
    }

    fn save_recents(&self, recents: &[String]) -> Result<(), String> {
        set_json(
            RECENTS_KEY,
            &StoredVec {
                items: recents.to_vec(),
            },
        )
    }

    fn load_recents(&self) -> Result<Vec<String>, String> {
        Ok(get_json::<StoredVec>(RECENTS_KEY)?
            .map(|stored| stored.items)
            .unwrap_or_default())
    }
}

pub async fn fetch_playlist_text(url: &str) -> Result<String, String> {
    let response = Request::get(url)
        .send()
        .await
        .map_err(|error| format!("Falha ao buscar playlist: {error}"))?;

    if !response.ok() {
        return Err(format!(
            "Falha ao buscar playlist: status HTTP {}",
            response.status()
        ));
    }

    response
        .text()
        .await
        .map_err(|error| format!("Falha ao ler resposta da playlist: {error}"))
}

/// Lê um ReadableStream e parseia o conteúdo como M3U em chunks (sem carregar tudo na memória).
pub async fn parse_readable_stream_into_playlist(
    body: ReadableStream,
) -> Result<ParsedPlaylist, String> {
    let reader = ReadableStreamDefaultReader::new(&body)
        .map_err(|_| "Falha ao obter leitor do stream.".to_string())?;

    let decoder = TextDecoder::new().map_err(|_| "Falha ao criar TextDecoder.")?;

    let mut parser = M3uStreamParser::new();
    let mut line_buffer = String::new();

    loop {
        let read_promise = reader.read();
        let read_result = JsFuture::from(read_promise)
            .await
            .map_err(|e| format!("Falha ao ler chunk do stream: {e:?}"))?;

        let done = Reflect::get(&read_result, &JsValue::from_str("done"))
            .ok()
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        if done {
            break;
        }

        let value_js = Reflect::get(&read_result, &JsValue::from_str("value"))
            .map_err(|_| "Chunk sem campo 'value'.")?;

        if value_js.is_undefined() || value_js.is_null() {
            continue;
        }

        let chunk = Uint8Array::new(&value_js);
        let decoded = decoder
            .decode_with_buffer_source(&chunk)
            .map_err(|_| "Falha ao decodificar chunk (UTF-8).")?;

        line_buffer.push_str(&decoded);

        while let Some(newline_pos) = line_buffer.find('\n') {
            let line = line_buffer[..newline_pos].trim_end_matches('\r');
            if !line.is_empty() {
                parser.feed_line(line);
            }
            line_buffer.replace_range(..newline_pos + 1, "");
        }
    }

    if !line_buffer.trim().is_empty() {
        parser.feed_line(line_buffer.trim_end_matches('\r'));
    }

    parser.finish()
}

/// Carrega a playlist por URL lendo o corpo em stream (chunks), sem carregar
/// todo o conteúdo na memória. Suporta playlists muito grandes (ex.: centenas de MB).
pub async fn fetch_playlist_stream(url: &str) -> Result<ParsedPlaylist, String> {
    let response = Request::get(url)
        .send()
        .await
        .map_err(|e| format!("Falha ao buscar playlist: {e}"))?;

    if !response.ok() {
        return Err(format!(
            "Falha ao buscar playlist: status HTTP {}",
            response.status()
        ));
    }

    let body = response
        .body()
        .ok_or_else(|| "Resposta sem corpo (body null).".to_string())?;

    parse_readable_stream_into_playlist(body).await
}

/// Carrega a playlist a partir de um arquivo local lendo em stream (file.stream()),
/// sem carregar o arquivo inteiro na memória.
pub async fn load_playlist_from_file_stream(file: web_sys::File) -> Result<ParsedPlaylist, String> {
    let file_js: &JsValue = file.as_ref();
    let stream_method = Reflect::get(file_js, &JsValue::from_str("stream"))
        .map_err(|_| "Arquivo sem método stream().")?;
    let stream_fn = stream_method
        .dyn_ref::<js_sys::Function>()
        .ok_or_else(|| "stream não é uma função.".to_string())?;
    let stream_value = stream_fn
        .call0(file_js)
        .map_err(|e| format!("Falha ao chamar file.stream(): {e:?}"))?;
    let stream: ReadableStream = stream_value
        .dyn_into()
        .map_err(|_| "file.stream() não retornou ReadableStream.")?;

    parse_readable_stream_into_playlist(stream).await
}

pub fn build_playlist_url(server_url: &str, username: &str, password: &str) -> Result<String, String> {
    let server_url = server_url.trim();
    if server_url.is_empty() {
        return Err("Informe a URL da playlist ou o endereco do servidor.".to_string());
    }

    let username = username.trim();
    let password = password.trim();

    if username.is_empty() && password.is_empty() {
        return Ok(server_url.to_string());
    }

    if username.is_empty() || password.is_empty() {
        return Err("Preencha usuario e senha para acessar o servidor protegido.".to_string());
    }

    let normalized = server_url.trim_end_matches('/');
    let expects_query = normalized.contains('?')
        || normalized.ends_with(".m3u")
        || normalized.ends_with(".m3u8")
        || normalized.ends_with("get.php");

    if expects_query {
        let mut playlist_url = normalized.to_string();
        let mut query_parts = Vec::new();

        if !normalized.contains("username=") {
            query_parts.push(format!("username={username}"));
        }

        if !normalized.contains("password=") {
            query_parts.push(format!("password={password}"));
        }

        if !normalized.contains("type=") {
            query_parts.push("type=m3u_plus".to_string());
        }

        if !normalized.contains("output=") {
            query_parts.push("output=ts".to_string());
        }

        if !query_parts.is_empty() {
            let separator = if playlist_url.ends_with('?') || playlist_url.ends_with('&') {
                ""
            } else if playlist_url.contains('?') {
                "&"
            } else {
                "?"
            };
            playlist_url.push_str(separator);
            playlist_url.push_str(&query_parts.join("&"));
        }

        return Ok(playlist_url);
    }

    Ok(format!(
        "{normalized}/get.php?username={username}&password={password}&type=m3u_plus&output=ts"
    ))
}

pub fn restore_snapshot() -> AppSnapshot {
    let storage = WebStorage;
    AppSnapshot {
        playlist: storage.load_playlist().ok().flatten(),
        favorites: storage.load_favorites().unwrap_or_default(),
        recents: storage.load_recents().unwrap_or_default(),
        connection: load_connection_settings().unwrap_or_default(),
        session: load_viewer_session().unwrap_or_default(),
    }
}

pub fn save_playlist(playlist: &ParsedPlaylist) -> Result<(), String> {
    WebStorage.save_playlist(playlist)
}

pub fn save_favorites(favorites: &[String]) -> Result<(), String> {
    WebStorage.save_favorites(favorites)
}

pub fn save_recents(recents: &[String]) -> Result<(), String> {
    WebStorage.save_recents(recents)
}

pub fn save_connection_settings(settings: &ConnectionSettings) -> Result<(), String> {
    set_json(CONNECTION_KEY, settings)
}

pub fn load_connection_settings() -> Result<ConnectionSettings, String> {
    Ok(get_json(CONNECTION_KEY)?.unwrap_or_default())
}

pub fn save_viewer_session(session: &ViewerSession) -> Result<(), String> {
    set_json(SESSION_KEY, session)
}

pub fn load_viewer_session() -> Result<ViewerSession, String> {
    Ok(get_json(SESSION_KEY)?.unwrap_or_default())
}

pub fn demo_playlist() -> &'static str {
    r#"#EXTM3U
#EXTINF:-1 tvg-id="mux-demo" group-title="Demo" tvg-logo="https://mux.com/favicon.ico",Mux Demo
https://test-streams.mux.dev/x36xhzz/x36xhzz.m3u8
#EXTINF:-1 tvg-id="apple-demo" group-title="Demo" tvg-logo="https://www.apple.com/favicon.ico",Apple BipBop
https://devstreaming-cdn.apple.com/videos/streaming/examples/img_bipbop_adv_example_ts/master.m3u8
"#
}

pub fn play_channel(video_id: &str, url: &str) -> Result<(), String> {
    js_play(video_id, url).map_err(js_error)
}

pub fn last_player_error(video_id: &str) -> String {
    js_last_error(video_id)
}

fn storage() -> Result<web_sys::Storage, String> {
    web_sys::window()
        .ok_or_else(|| "Window indisponivel.".to_string())?
        .local_storage()
        .map_err(|_| "Falha ao acessar localStorage.".to_string())?
        .ok_or_else(|| "localStorage indisponivel.".to_string())
}

fn set_json<T: Serialize>(key: &str, value: &T) -> Result<(), String> {
    let payload = serde_json::to_string(value).map_err(|error| error.to_string())?;
    storage()?
        .set_item(key, &payload)
        .map_err(|_| format!("Falha ao salvar chave {key} no localStorage."))
}

fn get_json<T: for<'de> Deserialize<'de>>(key: &str) -> Result<Option<T>, String> {
    let Some(raw) = storage()?
        .get_item(key)
        .map_err(|_| format!("Falha ao ler chave {key} do localStorage."))?
    else {
        return Ok(None);
    };

    serde_json::from_str(&raw)
        .map(Some)
        .map_err(|error| format!("Falha ao desserializar chave {key}: {error}"))
}

fn js_error(error: JsValue) -> String {
    error
        .as_string()
        .unwrap_or_else(|| "Erro desconhecido do player.".to_string())
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "IptvPlayer"], catch, js_name = play)]
    fn js_play(video_id: &str, url: &str) -> Result<(), JsValue>;

    #[wasm_bindgen(js_namespace = ["window", "IptvPlayer"], js_name = lastError)]
    fn js_last_error(video_id: &str) -> String;
}
