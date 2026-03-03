use dioxus::prelude::*;

use crate::services::playlist_service;
use iptv_core::ParsedPlaylist;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

#[component]
pub fn TopBar(
    server_url: String,
    username: String,
    password: String,
    profile_name: String,
    status_text: String,
    helper_message: String,
    on_server_change: EventHandler<String>,
    on_username_change: EventHandler<String>,
    on_password_change: EventHandler<String>,
    on_load_url: EventHandler<()>,
    on_load_demo: EventHandler<()>,
    on_logout: EventHandler<()>,
    on_file_loaded: EventHandler<Result<ParsedPlaylist, String>>,
) -> Element {
    rsx! {
        header { class: "topbar",
            div { class: "brand",
                h1 { "FinderBit Stream" }
                p { "{helper_message}" }
            }
            div { class: "topbar-actions",
                input {
                    class: "topbar-input topbar-input-url",
                    r#type: "text",
                    placeholder: "URL da playlist ou endereco do servidor",
                    value: server_url,
                    oninput: move |event| on_server_change.call(event.value()),
                }
                input {
                    class: "topbar-input topbar-input-credential",
                    r#type: "text",
                    placeholder: "Usuario",
                    value: username,
                    oninput: move |event| on_username_change.call(event.value()),
                }
                input {
                    class: "topbar-input topbar-input-credential",
                    r#type: "password",
                    placeholder: "Senha",
                    value: password,
                    oninput: move |event| on_password_change.call(event.value()),
                }
                button {
                    class: "primary-btn",
                    onclick: move |_| on_load_url.call(()),
                    "Carregar"
                }
                label {
                    class: "ghost-btn",
                    r#for: "playlist-file",
                    "Importar arquivo .m3u"
                }
                input {
                    id: "playlist-file",
                    class: "hidden-file-input",
                    r#type: "file",
                    accept: ".m3u,.m3u8,.txt",
                    onchange: move |_| {
                        let window = match web_sys::window() {
                            Some(w) => w,
                            None => {
                                on_file_loaded.call(Err("Janela indisponivel.".to_string()));
                                return;
                            }
                        };
                        let doc = match window.document() {
                            Some(d) => d,
                            None => {
                                on_file_loaded.call(Err("Documento indisponivel.".to_string()));
                                return;
                            }
                        };
                        let input_el = match doc.get_element_by_id("playlist-file") {
                            Some(el) => el,
                            None => {
                                on_file_loaded.call(Err("Input de arquivo nao encontrado.".to_string()));
                                return;
                            }
                        };
                        let input = match input_el.dyn_into::<HtmlInputElement>() {
                            Ok(i) => i,
                            Err(_) => {
                                on_file_loaded.call(Err("Elemento nao e input.".to_string()));
                                return;
                            }
                        };
                        let Some(files) = input.files() else {
                            on_file_loaded.call(Err("Nenhum arquivo.".to_string()));
                            return;
                        };
                        let Some(file) = files.item(0) else {
                            on_file_loaded.call(Err("Nenhum arquivo selecionado.".to_string()));
                            return;
                        };

                        let on_file_loaded = on_file_loaded.clone();
                        spawn(async move {
                            match playlist_service::load_playlist_from_file_stream(file).await {
                                Ok(parsed) => on_file_loaded.call(Ok(parsed)),
                                Err(error) => on_file_loaded.call(Err(error)),
                            }
                        });
                    }
                }
                button {
                    class: "secondary-btn",
                    onclick: move |_| on_load_demo.call(()),
                    "Demo"
                }
            }
            div { class: "topbar-meta",
                div { class: "status-pill", "{status_text}" }
                div { class: "profile-chip",
                    span { class: "profile-avatar", "{profile_name.chars().next().unwrap_or('F')}" }
                    span { class: "profile-name", "{profile_name}" }
                }
                button {
                    class: "ghost-btn",
                    onclick: move |_| on_logout.call(()),
                    "Sair"
                }
            }
        }
    }
}
