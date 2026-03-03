//! App Dioxus desktop: importação M3U (arquivo/URL com auth), progresso, lista paginada, busca.

use dioxus::prelude::*;
use iptv_m3u::{load_m3u_from_file, load_m3u_from_url, ChannelItem, Progress, RequestOptions};
use std::time::Duration;

const PAGE_SIZE: usize = 200;
const SEARCH_DEBOUNCE_MS: u64 = 300;

pub fn App(cx: Scope) -> Element {
    let channels = use_signal(cx, Vec::<ChannelItem>::new);
    let progress = use_signal(cx, Option::<Progress>::None);
    let page = use_signal(cx, || 0usize);
    let search_query = use_signal(cx, String::new);
    let search_debounced = use_signal(cx, String::new);
    let url_input = use_signal(cx, String::new);
    let basic_user = use_signal(cx, String::new);
    let basic_pass = use_signal(cx, String::new);
    let bearer = use_signal(cx, String::new);
    let status = use_signal(cx, || "Importe uma lista (arquivo ou URL).".to_string());

    // Debounce search: ao digitar, atualiza search_debounced após 300ms (não bloqueia UI)
    use_effect(cx, || {
        let q = search_query().clone();
        let sd = search_debounced.clone();
        spawn(async move {
            tokio::time::sleep(Duration::from_millis(SEARCH_DEBOUNCE_MS)).await;
            sd.set(q);
        });
    });

    let list = channels();
    let search = search_debounced().to_lowercase();
    let filtered: Vec<ChannelItem> = if search.is_empty() {
        list.clone()
    } else {
        list.iter()
            .filter(|c| c.name.to_lowercase().contains(&search))
            .cloned()
            .collect()
    };
    let total_filtered = filtered.len();
    let total_pages = (total_filtered.max(1) + PAGE_SIZE - 1) / PAGE_SIZE;
    let current_page = page().min(total_pages.saturating_sub(1));
    let page_start = current_page * PAGE_SIZE;
    let page_items: Vec<ChannelItem> = filtered
        .into_iter()
        .skip(page_start)
        .take(PAGE_SIZE)
        .collect();

    cx.render(rsx! {
        main { class: "app",
            h1 { "Player IPTV (listas grandes)" }
            p { "{status}" }

            div { class: "progress",
                if let Some(p) = progress() {
                    rsx! {
                        span { "Bytes: {p.bytes_read} | Itens: {p.items_processed}" }
                    }
                } else {
                    rsx! { span { "" } }
                }
            }

            div { class: "import",
                h2 { "Importar lista" }
                div {
                    button {
                        onclick: move |_| {
                            status.set("Abrindo seletor de arquivo...".to_string());
                            let channels = channels.to_owned();
                            let progress = progress.to_owned();
                            let status = status.to_owned();
                            spawn(async move {
                                let path = rfd::FileDialog::new()
                                    .add_filter("M3U", &["m3u", "m3u8", "txt"])
                                    .pick_file();
                                if let Some(path) = path {
                                    status.set("Carregando arquivo em streaming...".to_string());
                                    progress.set(Some(Progress { bytes_read: 0, items_processed: 0 }));
                                    channels.set(Vec::new());
                                    page.set(0);
                                    match load_m3u_from_file(&path) {
                                        Ok((mut rx, mut rx_progress)) => {
                                            let mut list = Vec::new();
                                            let mut batch = Vec::with_capacity(50);
                                            while let Some(item) = rx.recv().await {
                                                batch.push(item);
                                                if batch.len() >= 50 {
                                                    list.extend(batch.drain(..));
                                                    channels.set(list.clone());
                                                }
                                            }
                                            list.extend(batch);
                                            channels.set(list.clone());
                                            while let Some(p) = rx_progress.recv().await {
                                                progress.set(Some(p));
                                            }
                                            status.set(format!("Arquivo carregado: {} itens.", list.len()));
                                        }
                                        Err(e) => status.set(format!("Erro: {e}")),
                                    }
                                } else {
                                    status.set("Nenhum arquivo selecionado.".to_string());
                                }
                            });
                        },
                        "Importar arquivo local"
                    }
                }
                div { class: "url-import",
                    input {
                        placeholder: "URL da playlist (ex: http://servidor/get.php?user=...)",
                        value: "{url_input()}",
                        oninput: move |e| url_input.set(e.value.clone()),
                    }
                    input {
                        placeholder: "Basic Auth: usuário",
                        value: "{basic_user()}",
                        oninput: move |e| basic_user.set(e.value.clone()),
                    }
                    input {
                        placeholder: "Basic Auth: senha",
                        r#type: "password",
                        value: "{basic_pass()}",
                        oninput: move |e| basic_pass.set(e.value.clone()),
                    }
                    input {
                        placeholder: "Bearer token (opcional)",
                        value: "{bearer()}",
                        oninput: move |e| bearer.set(e.value.clone()),
                    }
                    button {
                        onclick: move |_| {
                            let url = url_input().trim().to_string();
                            if url.is_empty() {
                                status.set("Informe a URL.".to_string());
                                return;
                            }
                            status.set("Carregando URL em streaming...".to_string());
                            let channels = channels.to_owned();
                            let progress = progress.to_owned();
                            let status = status.to_owned();
                            let page = page.to_owned();
                            let user = basic_user().clone();
                            let pass = basic_pass().clone();
                            let token = bearer().clone();
                            spawn(async move {
                                progress.set(Some(Progress { bytes_read: 0, items_processed: 0 }));
                                channels.set(Vec::new());
                                page.set(0);
                                let mut opts = RequestOptions::default();
                                if !user.is_empty() || !pass.is_empty() {
                                    opts.basic_auth = Some((user, pass));
                                }
                                if !token.is_empty() {
                                    opts.bearer_token = Some(token);
                                }
                                match load_m3u_from_url(&url, opts).await {
                                    Ok((mut rx, mut rx_progress)) => {
                                        let mut list = Vec::new();
                                        let mut batch = Vec::with_capacity(50);
                                        while let Some(item) = rx.recv().await {
                                            batch.push(item);
                                            if batch.len() >= 50 {
                                                list.extend(batch.drain(..));
                                                channels.set(list.clone());
                                            }
                                        }
                                        list.extend(batch);
                                        channels.set(list.clone());
                                        while let Some(p) = rx_progress.recv().await {
                                            progress.set(Some(p));
                                        }
                                        status.set(format!("URL carregada: {} itens.", list.len()));
                                    }
                                    Err(e) => status.set(format!("Erro: {e}")),
                                }
                            });
                        },
                        "Carregar URL"
                    }
                }
            }

            div { class: "search",
                input {
                    placeholder: "Buscar por título (debounce 300ms)",
                    value: "{search_query()}",
                    oninput: move |e| search_query.set(e.value.clone()),
                }
            }

            div { class: "list-header",
                span { "Página {current_page + 1} de {total_pages} | Exibindo {page_items.len()} de {total_filtered} (total: {list.len()})" }
            }
            div { class: "pagination",
                button {
                    disabled: current_page == 0,
                    onclick: move |_| page.set(page().saturating_sub(1)),
                    "Anterior"
                }
                button {
                    disabled: current_page >= total_pages.saturating_sub(1),
                    onclick: move |_| page.set((page() + 1).min(total_pages.saturating_sub(1))),
                    "Próximo"
                }
            }
            ul { class: "channel-list",
                {page_items.iter().map(|c| rsx! {
                    li { key: "{c.id}",
                        span { class: "name", "{c.name}" }
                        span { class: "group", "{c.group_display()}" }
                    }
                })}
            }
        }
    })
}
