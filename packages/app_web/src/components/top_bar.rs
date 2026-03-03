use dioxus::prelude::*;

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
    on_file_loaded: EventHandler<Result<String, String>>,
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
