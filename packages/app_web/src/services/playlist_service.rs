use gloo_net::http::Request;
use iptv_core::storage::PlaylistStorage;
use iptv_core::ParsedPlaylist;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

const PLAYLIST_KEY: &str = "finderbit.playlist";
const FAVORITES_KEY: &str = "finderbit.favorites";
const RECENTS_KEY: &str = "finderbit.recents";

#[derive(Clone, Default)]
pub struct AppSnapshot {
    pub playlist: Option<ParsedPlaylist>,
    pub favorites: Vec<String>,
    pub recents: Vec<String>,
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

pub fn restore_snapshot() -> AppSnapshot {
    let storage = WebStorage;
    AppSnapshot {
        playlist: storage.load_playlist().ok().flatten(),
        favorites: storage.load_favorites().unwrap_or_default(),
        recents: storage.load_recents().unwrap_or_default(),
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
