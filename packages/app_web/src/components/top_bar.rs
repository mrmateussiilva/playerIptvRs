use dioxus::prelude::*;

#[component]
pub fn TopBar(
    playlist_url: String,
    status_text: String,
    helper_message: String,
    on_url_change: EventHandler<String>,
    on_load_url: EventHandler<()>,
    on_load_demo: EventHandler<()>,
    on_file_loaded: EventHandler<Result<String, String>>,
) -> Element {
    rsx! {
        header { class: "topbar",
            div { class: "brand",
                h1 { "FinderBit IPTV MVP" }
                p { "{helper_message}" }
            }
            div { class: "topbar-actions",
                input {
                    class: "url-input",
                    r#type: "url",
                    placeholder: "URL da playlist M3U",
                    value: playlist_url,
                    oninput: move |event| on_url_change.call(event.value()),
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
                    onchange: move |event| {
                        let files = event.files();
                        let Some(file) = files.first().cloned() else {
                            on_file_loaded.call(Err("Nenhum arquivo selecionado.".to_string()));
                            return;
                        };

                        let on_file_loaded = on_file_loaded.clone();
                        spawn(async move {
                            match file.read_string().await {
                                Ok(content) => on_file_loaded.call(Ok(content)),
                                Err(error) => on_file_loaded.call(Err(format!(
                                    "Falha ao ler arquivo: {error}"
                                ))),
                            }
                        });
                    }
                }
                button {
                    class: "secondary-btn",
                    onclick: move |_| on_load_demo.call(()),
                    "Carregar playlist demo"
                }
            }
            div { class: "status-pill", "{status_text}" }
        }
    }
}
