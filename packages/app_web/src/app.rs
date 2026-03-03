use dioxus::prelude::*;
use dioxus::hooks::use_signal;
use dioxus_signals::WritableExt;
use iptv_core::{parse_playlist, Channel, ParsedPlaylist};

use crate::components::{
    ChannelList, ChannelListItem, ListMode, LoginScreen, Player, SidebarGroups, TopBar,
};
use crate::services::playlist_service;

const VIDEO_ID: &str = "iptv-video";
const APP_STYLES: &str = include_str!("../assets/main.css");
const DEFAULT_LOGIN_USER: &str = "mateus";
const DEFAULT_LOGIN_PASSWORD: &str = "3010";

#[derive(Clone, PartialEq)]
enum PlayerState {
    Idle,
    Loading,
    Playing,
    Error(String),
}

#[component]
pub fn App() -> Element {
    let restored = playlist_service::restore_snapshot();
    let restored_playlist = restored.playlist.clone();
    let restored_favorites = restored.favorites.clone();
    let restored_recents = restored.recents.clone();
    let restored_connection = restored.connection.clone();
    let restored_session = restored.session.clone();
    let has_restored_playlist = restored_playlist.is_some();

    let mut playlist = use_signal(move || restored_playlist.clone().unwrap_or_default());
    let mut favorites = use_signal(move || restored_favorites.clone());
    let mut recents = use_signal(move || restored_recents.clone());
    let mut server_url = use_signal(move || restored_connection.server_url.clone());
    let mut username = use_signal(move || restored_connection.username.clone());
    let mut password = use_signal(move || restored_connection.password.clone());
    let mut selected_group = use_signal(|| "Todos".to_string());
    let mut search_query = use_signal(String::new);
    let mut list_mode = use_signal(|| ListMode::All);
    let mut current_channel = use_signal(|| None::<Channel>);
    let mut player_state = use_signal(|| PlayerState::Idle);
    let mut login_profile = use_signal(move || restored_session.profile_name.clone());
    let mut login_access_key = use_signal(String::new);
    let mut is_logged_in = use_signal(move || restored_session.is_logged_in);
    let mut ui_message = use_signal(|| {
        if has_restored_playlist {
            "Playlist restaurada do navegador.".to_string()
        } else {
            "Entre com o usuario mateus para acessar sua playlist.".to_string()
        }
    });

    let channels = filtered_channels(
        &playlist(),
        &favorites(),
        &recents(),
        &selected_group(),
        &search_query(),
        &list_mode(),
    );

    let status_text = format!(
        "carregado {} canais / {} grupos",
        playlist().channels.len(),
        playlist().groups.len()
    );
    let featured = current_channel()
        .clone()
        .or_else(|| playlist().channels.first().cloned());
    let featured_for_play = featured.clone();
    let featured_for_favorite = featured.clone();
    let has_featured = featured.is_some();

    if !is_logged_in() {
        return rsx! {
            div { class: "app-shell",
                style { "{APP_STYLES}" }
                LoginScreen {
                    profile_name: login_profile(),
                    access_key: login_access_key(),
                    helper_message: ui_message(),
                    on_profile_change: move |value| login_profile.set(value),
                    on_access_key_change: move |value| login_access_key.set(value),
                    on_login: move |_| {
                        let profile_name = login_profile().trim().to_string();
                        let access_key = login_access_key().trim().to_string();

                        if profile_name.is_empty() || access_key.is_empty() {
                            ui_message.set("Informe um perfil e uma senha para entrar.".to_string());
                            return;
                        }

                        if profile_name != DEFAULT_LOGIN_USER || access_key != DEFAULT_LOGIN_PASSWORD {
                            ui_message.set("Credenciais invalidas. Use mateus / 3010.".to_string());
                            return;
                        }

                        is_logged_in.set(true);
                        login_access_key.set(String::new());
                        ui_message.set(format!("Sessao iniciada para {profile_name}."));
                        let _ = playlist_service::save_viewer_session(
                            &playlist_service::ViewerSession {
                                profile_name,
                                is_logged_in: true,
                            },
                        );
                    }
                }
            }
        };
    }

    rsx! {
        div { class: "app-shell",
            style { "{APP_STYLES}" }
            div { class: "app-frame",
                TopBar {
                    server_url: server_url(),
                    username: username(),
                    password: password(),
                    profile_name: login_profile(),
                    status_text,
                    helper_message: ui_message(),
                    on_server_change: move |value: String| {
                        server_url.set(value.clone());
                        let _ = playlist_service::save_connection_settings(
                            &playlist_service::ConnectionSettings {
                                server_url: value,
                                username: username(),
                                password: password(),
                            },
                        );
                    },
                    on_username_change: move |value: String| {
                        username.set(value.clone());
                        let _ = playlist_service::save_connection_settings(
                            &playlist_service::ConnectionSettings {
                                server_url: server_url(),
                                username: value,
                                password: password(),
                            },
                        );
                    },
                    on_password_change: move |value: String| {
                        password.set(value.clone());
                        let _ = playlist_service::save_connection_settings(
                            &playlist_service::ConnectionSettings {
                                server_url: server_url(),
                                username: username(),
                                password: value,
                            },
                        );
                    },
                    on_load_url: move |_| {
                        let request_url = match playlist_service::build_playlist_url(
                            &server_url(),
                            &username(),
                            &password(),
                        ) {
                            Ok(url) => url,
                            Err(error) => {
                                ui_message.set(error);
                                return;
                            }
                        };

                        if request_url.is_empty() {
                            ui_message.set("Informe uma URL de playlist.".to_string());
                            return;
                        }

                        player_state.set(PlayerState::Idle);
                        current_channel.set(None);
                        ui_message.set("Carregando playlist do servidor...".to_string());

                        let mut ui_message = ui_message;
                        let mut playlist = playlist;
                        let mut selected_group = selected_group;
                        let mut search_query = search_query;

                        spawn(async move {
                            match playlist_service::fetch_playlist_text(&request_url).await {
                                Ok(content) => match parse_playlist(&content) {
                                    Ok(parsed) => {
                                        if let Err(error) = playlist_service::save_playlist(&parsed) {
                                            ui_message.set(format!(
                                                "Playlist carregada, mas falhou ao persistir: {error}"
                                            ));
                                        } else {
                                            ui_message.set(format!(
                                                "Playlist remota carregada com {} canais.",
                                                parsed.channels.len()
                                            ));
                                        }
                                        playlist.set(parsed);
                                        selected_group.set("Todos".to_string());
                                        search_query.set(String::new());
                                    }
                                    Err(error) => ui_message.set(error),
                                },
                                Err(error) => ui_message.set(error),
                            }
                        });
                    },
                    on_load_demo: move |_| {
                        match parse_playlist(playlist_service::demo_playlist()) {
                            Ok(parsed) => {
                                if let Err(error) = playlist_service::save_playlist(&parsed) {
                                    ui_message.set(format!(
                                        "Playlist demo carregada, mas nao foi salva: {error}"
                                    ));
                                } else {
                                    ui_message.set(format!(
                                        "Playlist demo carregada com {} canais.",
                                        parsed.channels.len()
                                    ));
                                }
                                playlist.set(parsed);
                                selected_group.set("Todos".to_string());
                                search_query.set(String::new());
                            }
                            Err(error) => ui_message.set(error),
                        }
                    },
                    on_logout: move |_| {
                        is_logged_in.set(false);
                        ui_message.set("Sessao encerrada.".to_string());
                        let _ = playlist_service::save_viewer_session(
                            &playlist_service::ViewerSession {
                                profile_name: login_profile(),
                                is_logged_in: false,
                            },
                        );
                    },
                    on_file_loaded: move |result: Result<String, String>| match result {
                        Ok(content) => match parse_playlist(&content) {
                            Ok(parsed) => {
                                if let Err(error) = playlist_service::save_playlist(&parsed) {
                                    ui_message.set(format!(
                                        "Playlist do arquivo carregada, mas nao foi salva: {error}"
                                    ));
                                } else {
                                    ui_message.set(format!(
                                        "Playlist do arquivo carregada com {} canais.",
                                        parsed.channels.len()
                                    ));
                                }
                                playlist.set(parsed);
                                selected_group.set("Todos".to_string());
                                search_query.set(String::new());
                            }
                            Err(error) => ui_message.set(error),
                        },
                        Err(error) => ui_message.set(error),
                    },
                }
                section { class: "hero-showcase",
                    div { class: "hero-copy",
                        p { class: "hero-kicker", "Em destaque" }
                        h2 {
                            if let Some(channel) = featured.as_ref() {
                                "{channel.name}"
                            } else {
                                "Conecte sua playlist"
                            }
                        }
                        p { class: "hero-description",
                            if let Some(channel) = featured.as_ref() {
                                "Categoria: "
                                strong { "{channel.group}" }
                                "  •  Use os cards abaixo para montar sua sessao de filmes, canais ou eventos ao vivo."
                            } else {
                                "Adicione a URL do servidor, importe um arquivo M3U ou use a playlist demo para popular o catalogo."
                            }
                        }
                        div { class: "hero-actions",
                            button {
                                class: "primary-btn",
                                disabled: !has_featured,
                                onclick: move |_| {
                                    if let Some(channel) = featured_for_play.clone() {
                                        current_channel.set(Some(channel.clone()));
                                        player_state.set(PlayerState::Loading);
                                        ui_message.set(format!("Abrindo {}...", channel.name));

                                        if let Err(error) = playlist_service::play_channel(VIDEO_ID, &channel.url) {
                                            player_state.set(PlayerState::Error(error.clone()));
                                            ui_message.set(error);
                                            return;
                                        }

                                        let updated = push_recent(&recents(), &channel.id);
                                        if let Err(error) = playlist_service::save_recents(&updated) {
                                            ui_message.set(format!(
                                                "Canal tocando, mas falhou ao salvar recentes: {error}"
                                            ));
                                        }
                                        recents.set(updated);
                                    }
                                },
                                "Assistir agora"
                            }
                            button {
                                class: "ghost-btn",
                                disabled: !has_featured,
                                onclick: move |_| {
                                    if let Some(channel) = featured_for_favorite.as_ref() {
                                        let updated = toggle_favorite(&favorites(), &channel.id);
                                        if playlist_service::save_favorites(&updated).is_ok() {
                                            favorites.set(updated);
                                        }
                                    }
                                },
                                "Adicionar a minha lista"
                            }
                        }
                    }
                    div { class: "hero-metrics",
                        div { class: "metric-card",
                            span { class: "metric-label", "Perfil" }
                            strong { "{login_profile()}" }
                        }
                        div { class: "metric-card",
                            span { class: "metric-label", "Favoritos" }
                            strong { "{favorites().len()}" }
                        }
                        div { class: "metric-card",
                            span { class: "metric-label", "Recentes" }
                            strong { "{recents().len()}" }
                        }
                    }
                }
                div { class: "content-grid",
                    SidebarGroups {
                        groups: playlist().groups.clone(),
                        selected_group: selected_group(),
                        on_select: move |group| selected_group.set(group),
                    }
                    section { class: "catalog-panel",
                        div { class: "catalog-layout",
                            section { class: "panel",
                                ChannelList {
                                    channels,
                                    selected_channel_id: current_channel()
                                        .as_ref()
                                        .map(|channel| channel.id.clone()),
                                    search_query: search_query(),
                                    mode: list_mode(),
                                    on_search_change: move |value| search_query.set(value),
                                    on_mode_change: move |mode| list_mode.set(mode),
                                    on_select_channel: move |channel_id| {
                                        let selected = playlist()
                                            .channels
                                            .iter()
                                            .find(|channel| channel.id == channel_id)
                                            .cloned();

                                        let Some(channel) = selected else {
                                            ui_message.set("Titulo nao encontrado.".to_string());
                                            return;
                                        };

                                        current_channel.set(Some(channel.clone()));
                                        player_state.set(PlayerState::Loading);
                                        ui_message.set(format!("Abrindo {}...", channel.name));

                                        if let Err(error) = playlist_service::play_channel(VIDEO_ID, &channel.url) {
                                            player_state.set(PlayerState::Error(error.clone()));
                                            ui_message.set(error);
                                            return;
                                        }

                                        let updated = push_recent(&recents(), &channel.id);
                                        if let Err(error) = playlist_service::save_recents(&updated) {
                                            ui_message.set(format!(
                                                "Canal tocando, mas falhou ao salvar recentes: {error}"
                                            ));
                                        }
                                        recents.set(updated);
                                    },
                                    on_toggle_favorite: move |channel_id: String| {
                                        let updated = toggle_favorite(&favorites(), &channel_id);
                                        match playlist_service::save_favorites(&updated) {
                                            Ok(()) => favorites.set(updated),
                                            Err(error) => ui_message.set(error),
                                        }
                                    },
                                }
                            }
                            section { class: "panel player-panel",
                                Player {
                                    video_id: VIDEO_ID.to_string(),
                                    channel_name: current_channel()
                                        .as_ref()
                                        .map(|channel| channel.name.clone()),
                                    state_label: player_label(&player_state()),
                                    is_error: matches!(player_state(), PlayerState::Error(_)),
                                    on_playing: move |_| {
                                        player_state.set(PlayerState::Playing);
                                        if let Some(channel) = current_channel().as_ref() {
                                            ui_message.set(format!("Tocando {}.", channel.name));
                                        }
                                    },
                                    on_error: move |_| {
                                        let error = playlist_service::last_player_error(VIDEO_ID);
                                        let message = if error.is_empty() {
                                            "Falha ao reproduzir o canal.".to_string()
                                        } else {
                                            error
                                        };
                                        player_state.set(PlayerState::Error(message.clone()));
                                        ui_message.set(message);
                                    },
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn filtered_channels(
    playlist: &ParsedPlaylist,
    favorites: &[String],
    recents: &[String],
    selected_group: &str,
    search_query: &str,
    mode: &ListMode,
) -> Vec<ChannelListItem> {
    let filter_group = selected_group != "Todos";
    let search = search_query.trim().to_lowercase();

    let base: Vec<Channel> = match mode {
        ListMode::All => playlist.channels.clone(),
        ListMode::Favorites => playlist
            .channels
            .iter()
            .filter(|channel| favorites.contains(&channel.id))
            .cloned()
            .collect(),
        ListMode::Recent => recents
            .iter()
            .filter_map(|id| playlist.channels.iter().find(|channel| &channel.id == id))
            .cloned()
            .collect(),
    };

    base.into_iter()
        .filter(|channel| !filter_group || channel.group == selected_group)
        .filter(|channel| search.is_empty() || channel.name.to_lowercase().contains(&search))
        .map(|channel| ChannelListItem {
            id: channel.id.clone(),
            name: channel.name,
            group: channel.group,
            logo: channel.logo,
            is_favorite: favorites.contains(&channel.id),
        })
        .collect()
}

fn toggle_favorite(current: &[String], channel_id: &str) -> Vec<String> {
    let mut next = current.to_vec();
    if let Some(index) = next.iter().position(|id| id == channel_id) {
        next.remove(index);
    } else {
        next.push(channel_id.to_string());
    }
    next
}

fn push_recent(current: &[String], channel_id: &str) -> Vec<String> {
    let mut next = current
        .iter()
        .filter(|id| id.as_str() != channel_id)
        .cloned()
        .collect::<Vec<_>>();

    next.insert(0, channel_id.to_string());
    next.truncate(10);
    next
}

fn player_label(state: &PlayerState) -> String {
    match state {
        PlayerState::Idle => "Selecione um titulo para iniciar.".to_string(),
        PlayerState::Loading => "Carregando stream...".to_string(),
        PlayerState::Playing => "Reproduzindo agora".to_string(),
        PlayerState::Error(message) => format!("Erro: {message}"),
    }
}
