//! Player IPTV desktop: listas M3U em streaming, progresso e lista paginada.
//! Requer runtime tokio (dioxus-desktop com tokio_runtime).

fn main() {
    dioxus_desktop::launch(app_desktop::App);
}
