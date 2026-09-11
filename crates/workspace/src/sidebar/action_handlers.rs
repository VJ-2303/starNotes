use gpui_kit::component::{
    ActiveTheme, Icon, IconName, WindowExt,
    button::{Button, ButtonVariant, ButtonVariants as _},
    dialog::DialogButtonProps,
    notification::Notification,
};
use gpui_kit::{ClipboardItem, Context, Styled, Window};
use storage::workspace::trash;

use super::{SidebarView, action::*};

struct TrashUndoNotification;

impl SidebarView {
    pub(super) fn on_copy_note_internal_link_action(
        &mut self,
        action: &CopyNoteInternalLinkAction,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let title = self
            .projects
            .iter()
            .flat_map(|project| project.notes.iter())
            .chain(self.standalone_notes.iter())
            .find(|note| note.id == action.0)
            .map(|note| note.title.as_ref())
            .unwrap_or("Note");
        let item = storage::workspace::links::WorkspaceItemRef {
            kind: storage::workspace::links::WorkspaceItemKind::Note,
            id: i64::from(action.0),
        };
        let catalog = self.reference_catalog();
        cx.write_to_clipboard(ClipboardItem::new_string(
            catalog
                .format_item_link(item, None)
                .unwrap_or_else(|| storage::workspace::links::stable_workspace_link(item, title)),
        ));
    }

    fn reference_catalog(&self) -> storage::workspace::links::WorkspaceReferenceCatalog {
        let mut items = Vec::new();
        for project in &self.projects {
            for note in &project.notes {
                items.push(storage::workspace::links::WorkspaceCatalogEntry {
                    item: storage::workspace::links::WorkspaceItemRef {
                        kind: storage::workspace::links::WorkspaceItemKind::Note,
                        id: i64::from(note.id),
                    },
                    title: note.title.to_string(),
                    project_id: Some(i64::from(project.id)),
                    project_name: Some(project.name.to_string()),
                    board_id: None,
                    board_title: None,
                    list_id: None,
                    list_title: None,
                });
            }
        }
        for note in &self.standalone_notes {
            items.push(storage::workspace::links::WorkspaceCatalogEntry {
                item: storage::workspace::links::WorkspaceItemRef {
                    kind: storage::workspace::links::WorkspaceItemKind::Note,
                    id: i64::from(note.id),
                },
                title: note.title.to_string(),
                project_id: None,
                project_name: None,
                board_id: None,
                board_title: None,
                list_id: None,
                list_title: None,
            });
        }
        storage::workspace::links::WorkspaceReferenceCatalog {
            items,
            ..Default::default()
        }
    }

    pub(super) fn on_move_note_action(
        &mut self,
        action: &MoveNoteAction,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_note(cx, action.note_id, action.project_id);
    }

    pub(super) fn on_delete_note_action(
        &mut self,
        action: &DeleteNoteAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let title = self
            .projects
            .iter()
            .flat_map(|project| project.notes.iter())
            .chain(self.standalone_notes.iter())
            .find(|note| note.id == action.0)
            .map(|note| note.title.clone())
            .unwrap_or_else(|| "Note".into());
        self.delete_note(action.0, title, window, cx);
    }

    pub(super) fn on_edit_note_action(
        &mut self,
        action: &EditNoteAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.start_renaming_note(action, window, cx);
    }

    pub(super) fn on_rename_project_action(
        &mut self,
        action: &RenameProjectAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.start_renaming_project(action, window, cx);
    }

    pub(super) fn on_delete_project_action(
        &mut self,
        action: &DeleteProjectAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = self.projects.iter().find(|project| project.id == action.0) else {
            return;
        };
        let title = project.name.clone();
        let child_count = project.notes.len();
        let view = cx.entity();
        let project_id = action.0;
        window.open_alert_dialog(cx, move |alert, _, cx| {
            alert
                .icon(Icon::new(IconName::TriangleAlert).text_color(cx.theme().danger))
                .title(format!("Move project ‘{title}’ to Trash"))
                .description(format!(
                    "This hides the project and its {child_count} item(s) until you restore it."
                ))
                .button_props(
                    DialogButtonProps::default()
                        .ok_variant(ButtonVariant::Danger)
                        .ok_text("Move to Trash")
                        .cancel_text("Cancel")
                        .show_cancel(true),
                )
                .on_ok({
                    let view = view.clone();
                    move |_, _, cx| {
                        view.update(cx, |this, cx| this.delete_project(cx, project_id));
                        true
                    }
                })
        });
    }

    pub(super) fn push_trash_undo(
        &self,
        kind: trash::TrashItemKind,
        id: u32,
        title: gpui_kit::SharedString,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let sidebar = cx.entity();
        window.push_notification(
            Notification::info(format!("Moved {title} to Trash"))
                .id::<TrashUndoNotification>()
                .action(move |_, _, cx| {
                    let sidebar = sidebar.clone();
                    Button::new("undo-move-to-trash")
                        .label("Undo")
                        .primary()
                        .on_click(cx.listener(move |notification, _, window, cx| {
                            sidebar.update(cx, |this, cx| this.restore_trashed(kind, id, cx));
                            notification.dismiss(window, cx);
                        }))
                })
                .autohide(true),
            cx,
        );
    }

    pub(super) fn on_move_project_up_action(
        &mut self,
        action: &MoveProjectUpAction,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_project_up(cx, action.0);
    }

    pub(super) fn on_toggle_note_pinned_action(
        &mut self,
        action: &ToggleNotePinnedAction,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_note_pinned(action.note_id, action.pinned, cx);
    }

    pub(super) fn on_move_project_down_action(
        &mut self,
        action: &MoveProjectDownAction,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_project_down(cx, action.0);
    }
}
