use dioxus::prelude::*;

#[component]
pub fn SidebarGroups(
    groups: Vec<String>,
    selected_group: String,
    on_select: EventHandler<String>,
) -> Element {
    let group_on_select = on_select.clone();
    let group_buttons = groups
        .into_iter()
        .map(|group| {
            let class_name = if selected_group == group {
                "group-btn active"
            } else {
                "group-btn"
            };
            let button_key = group.clone();
            let group_for_click = group.clone();
            let click_handler = group_on_select.clone();

            rsx! {
                button {
                    key: "{button_key}",
                    class: class_name,
                    onclick: move |_| click_handler.call(group_for_click.clone()),
                    "{group}"
                }
            }
        })
        .collect::<Vec<_>>();

    rsx! {
        aside { class: "panel",
            h2 { "Grupos" }
            p { class: "panel-subtitle", "Filtre rapidamente os canais por categoria." }
            div { class: "groups-list",
                button {
                    class: if selected_group == "Todos" { "group-btn active" } else { "group-btn" },
                    onclick: move |_| on_select.call("Todos".to_string()),
                    "Todos"
                }
                {group_buttons}
            }
        }
    }
}
