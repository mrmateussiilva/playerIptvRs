use dioxus::prelude::*;

#[component]
pub fn Player(
    video_id: String,
    channel_name: Option<String>,
    state_label: String,
    is_error: bool,
    on_playing: EventHandler<()>,
    on_error: EventHandler<()>,
) -> Element {
    let title = channel_name.unwrap_or_else(|| "Nenhum canal selecionado".to_string());
    let on_can_play = on_playing.clone();

    rsx! {
        div { class: "player-card",
            h2 { "Player" }
            p { class: "player-title", "{title}" }
            div {
                class: if is_error { "player-status error" } else { "player-status" },
                "{state_label}"
            }
            div { class: "video-shell",
                video {
                    id: "{video_id}",
                    controls: true,
                    autoplay: true,
                    playsinline: true,
                    preload: "none",
                    oncanplay: move |_| on_can_play.call(()),
                    onplaying: move |_| on_playing.call(()),
                    onerror: move |_| on_error.call(()),
                }
            }
        }
    }
}
