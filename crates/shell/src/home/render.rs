use super::*;

impl AppShell {
    pub(crate) fn render_home(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let active_project = self.workspace.active_project_id.and_then(|id| {
            self.workspace
                .projects
                .iter()
                .find(|project| project.id == id)
        });
        let active_project_id = active_project.map(|project| project.id);

        v_flex()
            .id("workspace-home")
            .size_full()
            .overflow_y_scrollbar()
            .bg(cx.theme().background)
            .child(
                v_flex()
                    .w_full()
                    .max_w(px(1080.))
                    .mx_auto()
                    .p_6()
                    .gap_6()
                    .child(
                        h_flex()
                            .items_end()
                            .justify_between()
                            .gap_4()
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_2xl()
                                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                                            .child("Home"),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child("The work that needs your attention, without the noise."),
                                    ),
                            )
                            .child(
                                Button::new("home-new-note")
                                    .icon(IconName::Plus)
                                    .label(match active_project {
                                        Some(project) => format!("Note in {}", project.name),
                                        None => "New note".to_string(),
                                    })
                                    .primary()
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.create_note(active_project_id, window, cx);
                                    })),
                            ),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .items_start()
                            .gap_6()
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap_3()
                                    .child(section_title("Pinned", "Keep close", cx))
                                    .child(self.render_home_items(
                                        "home-pinned",
                                        &self.home.data.pinned,
                                        "Pin notes from their item menu.",
                                        cx,
                                    )),
                            )
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap_3()
                                    .child(section_title("Recent", "Last opened", cx))
                                    .child(self.render_home_items(
                                        "home-recent",
                                        &self.home.data.recent,
                                        "Open a note and it will appear here.",
                                        cx,
                                    )),
                            ),
                    ),
            )
    }

    pub(crate) fn render_home_items(
        &self,
        id: &'static str,
        items: &[WorkspaceHomeItem],
        empty_copy: &'static str,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        if items.is_empty() {
            return div()
                .id(id)
                .p_3()
                .rounded(cx.theme().radius)
                .bg(cx.theme().secondary.opacity(0.32))
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(empty_copy)
                .into_any_element();
        }
        v_flex()
            .id(id)
            .gap_1()
            .children(items.iter().cloned().enumerate().map(|(index, item)| {
                let icon = match item.kind {
                    WorkspaceItemKind::Note => IconName::BookOpen,
                };
                h_flex()
                    .id((id, index))
                    .w_full()
                    .min_w_0()
                    .gap_2()
                    .px_2()
                    .py_2()
                    .rounded(cx.theme().radius)
                    .hover(|this| this.bg(cx.theme().secondary_hover.opacity(0.7)))
                    .child(
                        Icon::new(icon)
                            .xsmall()
                            .text_color(cx.theme().muted_foreground),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_sm()
                            .text_ellipsis()
                            .overflow_hidden()
                            .child(item.title.clone()),
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.open_home_item(item.clone(), window, cx);
                    }))
            }))
            .into_any_element()
    }
}
