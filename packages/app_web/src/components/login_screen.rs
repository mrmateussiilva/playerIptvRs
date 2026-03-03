use dioxus::prelude::*;

#[component]
pub fn LoginScreen(
    profile_name: String,
    access_key: String,
    helper_message: String,
    on_profile_change: EventHandler<String>,
    on_access_key_change: EventHandler<String>,
    on_login: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "login-shell",
            div { class: "login-backdrop" }
            section { class: "login-card",
                div { class: "login-brand",
                    p { class: "login-kicker", "FinderBit Stream" }
                    h1 { "Sua sala de filmes e canais" }
                    p { class: "login-copy", "{helper_message}" }
                }
                div { class: "login-form",
                    input {
                        class: "login-input",
                        r#type: "text",
                        placeholder: "Nome do perfil ou email",
                        value: profile_name,
                        oninput: move |event| on_profile_change.call(event.value()),
                    }
                    input {
                        class: "login-input",
                        r#type: "password",
                        placeholder: "Senha de acesso",
                        value: access_key,
                        oninput: move |event| on_access_key_change.call(event.value()),
                    }
                    button {
                        class: "login-btn",
                        onclick: move |_| on_login.call(()),
                        "Entrar"
                    }
                    p { class: "login-note", "A autenticacao e local neste MVP. Usuario: mateus  •  Senha: 3010" }
                }
            }
        }
    }
}
