use dioxus::prelude::*;
use iptv_core::{parse_playlist, Channel, ParsedPlaylist};

use crate::components::{ChannelList, ChannelListItem, ListMode, Player, SidebarGroups, TopBar};
use crate::services::playlist_service;

const VIDEO_ID: &str = "iptv-video";

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
    let has_restored_playlist = restored_playlist.is_some();

    let mut playlist = use_signal(move || restored_playlist.clone().unwrap_or_default());
    let mut favorites = use_signal(move || restored_favorites.clone());
    let mut recents = use_signal(move || restored_recents.clone());
    let mut playlist_url = use_signal(String::new);
    let mut selected_group = use_signal(|| "Todos".to_string());
    let mut search_query = use_signal(String::new);
    let mut list_mode = use_signal(|| ListMode::All);
    let mut current_channel = use_signal(|| None::<Channel>);
    let mut player_state = use_signal(|| PlayerState::Idle);
    let mut ui_message = use_signal(|| {
        if has_restored_playlist {
            "Playlist restaurada do navegador.".to_string()
        } else {
            "Pronto para carregar uma playlist.".to_string()
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

    rsx! {
        div { class: "app-shell",
            div { class: "app-frame",
                TopBar {
                    playlist_url: playlist_url(),
                    status_text,
                    helper_message: ui_message(),
                    on_url_change: move |value| playlist_url.set(value),
                    on_load_url: move |_| {
                        let url = playlist_url().trim().to_string();
                        if url.is_empty() {
                            ui_message.set("Informe uma URL de playlist.".to_string());
                            return;
                        }

                        player_state.set(PlayerState::Idle);
                        current_channel.set(None);
                        ui_message.set("Carregando playlist via URL...".to_string());

                        let mut ui_message = ui_message;
                        let mut playlist = playlist;
                        let mut selected_group = selected_group;
                        let mut search_query = search_query;

                        spawn(async move {
                            match playlist_service::fetch_playlist_text(&url).await {
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
                    on_file_loaded: move |result| match result {
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
                div { class: "content-grid",
                    SidebarGroups {
                        groups: playlist().groups.clone(),
                        selected_group: selected_group(),
                        on_select: move |group| selected_group.set(group),
                    }
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
                                    ui_message.set("Canal nao encontrado.".to_string());
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
                            on_toggle_favorite: move |channel_id| {
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
        PlayerState::Idle => "Selecione um canal para iniciar.".to_string(),
        PlayerState::Loading => "Loading".to_string(),
        PlayerState::Playing => "Playing".to_string(),
        PlayerState::Error(message) => format!("Error: {message}"),
    }
}
