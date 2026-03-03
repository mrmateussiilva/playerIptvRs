use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub enum ListMode {
    All,
    Favorites,
    Recent,
}

#[derive(Clone, PartialEq)]
pub struct ChannelListItem {
    pub id: String,
    pub name: String,
    pub group: String,
    pub logo: Option<String>,
    pub is_favorite: bool,
}

#[component]
pub fn ChannelList(
    channels: Vec<ChannelListItem>,
    channels_total: usize,
    display_limit: usize,
    on_load_more: EventHandler<()>,
    selected_channel_id: Option<String>,
    search_query: String,
    mode: ListMode,
    on_search_change: EventHandler<String>,
    on_mode_change: EventHandler<ListMode>,
    on_select_channel: EventHandler<String>,
    on_toggle_favorite: EventHandler<String>,
) -> Element {
    let is_empty = channels.is_empty();
    let has_more = channels_total > display_limit;
    let selected_id = selected_channel_id.clone();
    let all_mode_handler = on_mode_change.clone();
    let favorite_mode_handler = on_mode_change.clone();
    let recent_mode_handler = on_mode_change.clone();

    rsx! {
        h2 { "Catalogo" }
        p { class: "panel-subtitle", "Titulos organizados por filtros, favoritos e historico recente." }
        div { class: "list-toolbar",
            button {
                class: if mode == ListMode::All { "tab-btn active" } else { "tab-btn" },
                onclick: move |_| all_mode_handler.call(ListMode::All),
                "Descobrir"
            }
            button {
                class: if mode == ListMode::Favorites { "tab-btn active" } else { "tab-btn" },
                onclick: move |_| favorite_mode_handler.call(ListMode::Favorites),
                "Minha lista"
            }
            button {
                class: if mode == ListMode::Recent { "tab-btn active" } else { "tab-btn" },
                onclick: move |_| recent_mode_handler.call(ListMode::Recent),
                "Continuar"
            }
        }
        input {
            class: "search-input",
            r#type: "search",
            placeholder: "Buscar titulo ou canal",
            value: search_query,
            oninput: move |event| on_search_change.call(event.value()),
        }
        if is_empty {
            div { class: "empty-state", "Nenhum titulo encontrado para este filtro." }
        } else {
            div { class: "channels-rail",
                {channels.iter().map(|channel| {
                    let channel_id = channel.id.clone();
                    let select_id = channel.id.clone();
                    let favorite_id = channel.id.clone();
                    let channel_name = channel.name.clone();
                    let channel_group = channel.group.clone();
                    let channel_logo = channel.logo.clone();
                    let is_selected = selected_id.as_deref() == Some(channel_id.as_str());
                    let select_handler = on_select_channel.clone();
                    let favorite_handler = on_toggle_favorite.clone();

                    rsx! {
                        article { key: "{channel_id}", class: "title-card",
                            button {
                                class: if channel.is_favorite {
                                    "favorite-btn active floating"
                                } else {
                                    "favorite-btn floating"
                                },
                                onclick: move |_| favorite_handler.call(favorite_id.clone()),
                                if channel.is_favorite { "★" } else { "☆" }
                            }
                            button {
                                class: if is_selected { "channel-btn active" } else { "channel-btn" },
                                onclick: move |_| select_handler.call(select_id.clone()),
                                div { class: "title-art",
                                    if let Some(logo) = channel_logo.clone() {
                                        img {
                                            class: "channel-logo",
                                            src: logo,
                                            alt: "Logo do canal",
                                        }
                                    } else {
                                        div { class: "channel-logo-placeholder", "PLAY" }
                                    }
                                    span { class: "channel-pill", "{channel_group}" }
                                }
                                div { class: "channel-card-body",
                                    div { class: "channel-name", "{channel_name}" }
                                    div { class: "channel-meta", "Assistir agora" }
                                }
                            }
                        }
                    }
                })}
                if has_more {
                    div { class: "load-more-wrap",
                        button {
                            class: "primary-btn load-more-btn",
                            onclick: move |_| on_load_more.call(()),
                            "Mostrar mais ({channels.len()} de {channels_total} exibidos)"
                        }
                    }
                }
            }
        }
        div { class: "recent-note", "Continuar exibe os ultimos 10 titulos reproduzidos." }
    }
}
