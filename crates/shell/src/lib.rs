mod action;
mod action_handlers;
mod home;
mod render;
mod tabs;
mod workspace;

pub(crate) use action::{CloseAllTabsAction, CloseOtherTabsAction, CloseTabAction};
pub use action::{
    CycleNextTab, CyclePrevTab, ExportWorkspaceAction, ImportWorkspaceAction, OpenSettingsAction,
    ToggleSidebarAction,
};
use gpui_kit::component::{
    ActiveTheme, IconName, Root, Sizable as _, TitleBar, WindowExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{
        Escape as InputEscape, InputEvent, InputState, MoveDown as InputMoveDown,
        MoveUp as InputMoveUp,
    },
    menu::ContextMenuExt as _,
    notification::Notification,
    tab::{Tab, TabBar},
    v_flex,
};
use gpui_kit::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    MouseButton, ParentElement, PathPromptOptions, Pixels, Render, ScrollHandle, SharedString,
    Styled, Task, Window, div, prelude::FluentBuilder as _, px,
};
use std::{collections::HashMap, rc::Rc, sync::Arc};
use storage::workspace::WorkspaceTitleTarget;

use ::workspace::{SidebarEvent, SidebarView};
use command_palette::{CommandPaletteEvent, CommandPaletteView};
use document_editor::{
    DEFAULT_NOTE, DocumentEditorEvent, DocumentEditorView, DocumentKind, SaveState,
    unique_note_path,
};
use settings::{
    AppSettings, SettingsDocumentEvent, SettingsDocumentSaveState, SettingsDocumentView,
    SettingsIntegration, SettingsView, ShortcutReference, StoredTab, WorkspaceArchiveActions,
};
use storage::time::unix_timestamp_seconds as now_ts;
use storage::workspace::home::WorkspaceHomeState;
use storage::workspace::trash::{TrashItem, TrashItemKind};

const SIDEBAR_AUTO_COLLAPSE_WIDTH: f32 = 900.;

type ShortcutProvider = Rc<dyn Fn(&App) -> Vec<ShortcutReference>>;

#[derive(Clone)]
pub struct ShellIntegration {
    _shortcuts: ShortcutProvider,
}

impl ShellIntegration {
    pub fn new(
        shortcuts: impl Fn(&App) -> Vec<ShortcutReference> + 'static,
    ) -> Self {
        Self {
            _shortcuts: Rc::new(shortcuts),
        }
    }
}

#[cfg(test)]
fn test_shell_integration() -> ShellIntegration {
    ShellIntegration::new(|_| Vec::new())
}

struct OpenTab {
    id: u64,
    title: SharedString,
    kind: OpenTabKind,
}

enum OpenTabKind {
    Chooser,
    Trash,
    Note {
        note_id: u32,
        project_id: Option<u32>,
        view: Entity<DocumentEditorView>,
    },
    Settings {
        view: Entity<SettingsDocumentView>,
    },
}

struct PendingWorkspaceTitleSave {
    generation: u64,
    title: String,
}

#[derive(Clone)]
pub(crate) struct ProjectChoice {
    pub(crate) id: u32,
    pub(crate) name: SharedString,
}

#[derive(Clone)]
pub(crate) struct NoteChoice {
    pub(crate) id: u32,
    pub(crate) title: SharedString,
    pub(crate) project_id: Option<u32>,
    pub(crate) project_name: Option<SharedString>,
}

struct TabsState {
    open_tabs: Vec<OpenTab>,
    note_views: HashMap<u32, Entity<DocumentEditorView>>,
    active_tab_index: usize,
    next_tab_id: u64,
    tab_scroll_handle: ScrollHandle,
}

#[derive(Clone)]
enum LoadPhase {
    Initial,
    Loading {
        had_content: bool,
    },
    Ready,
    Failed {
        message: SharedString,
        had_content: bool,
    },
}

impl LoadPhase {
    fn is_loading(&self) -> bool {
        matches!(self, Self::Loading { .. })
    }

    fn has_content(&self) -> bool {
        matches!(
            self,
            Self::Ready
                | Self::Loading { had_content: true }
                | Self::Failed {
                    had_content: true,
                    ..
                }
        )
    }

    fn error(&self) -> Option<SharedString> {
        match self {
            Self::Failed { message, .. } => Some(message.clone()),
            _ => None,
        }
    }
}

pub(crate) struct WorkspaceState {
    pub(crate) projects: Vec<ProjectChoice>,
    pub(crate) notes: Vec<NoteChoice>,
    pub(crate) active_project_id: Option<u32>,
    refreshing: bool,
    refresh_pending: bool,
    pending_title_saves: HashMap<WorkspaceTitleTarget, PendingWorkspaceTitleSave>,
    title_save_lock: Arc<tokio::sync::Mutex<()>>,
}

struct HomeState {
    data: WorkspaceHomeState,
    phase: LoadPhase,
    refresh_pending: bool,
}

struct TrashState {
    items: Vec<TrashItem>,
    phase: LoadPhase,
    refresh_pending: bool,
    search_input: Entity<InputState>,
    query: String,
    kind_filter: Option<TrashItemKind>,
}

struct ExternalChangeState {
    task: Option<Task<()>>,
    revision: Option<i64>,
    note_revision: Option<i64>,
    link_revision: Option<i64>,
}

pub struct AppShell {
    pub(crate) focus_handle: FocusHandle,
    sidebar: Entity<SidebarView>,
    settings_view: Entity<SettingsView>,
    title_input: Entity<InputState>,
    command_palette: Entity<CommandPaletteView>,
    tabs: TabsState,
    pub(crate) workspace: WorkspaceState,
    suppress_title_event: bool,
    window_is_narrow: bool,
    home: HomeState,
    trash: TrashState,
    external_changes: ExternalChangeState,
    record_opened_task: Option<Task<()>>,
    workspace_archive_busy: bool,
}

impl AppShell {
    pub fn view(window: &mut Window, integration: ShellIntegration, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, integration, cx))
    }

    fn observe_document_editor(
        view: &Entity<DocumentEditorView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.subscribe_in(
            view,
            window,
            |this, _, event: &DocumentEditorEvent, window, cx| match event {
                DocumentEditorEvent::PathChanged => this.refresh_workspace(cx),
                DocumentEditorEvent::Saved(note_id) => {
                    if !this.tabs.open_tabs.iter().any(|tab| {
                        matches!(
                            &tab.kind,
                            OpenTabKind::Note {
                                note_id: open_note_id,
                                ..
                            } if *open_note_id == *note_id
                        )
                    }) {
                        this.tabs.note_views.remove(note_id);
                    }
                }
                DocumentEditorEvent::WorkspaceLinksChanged => {
                    for view in this.tabs.note_views.values() {
                        view.update(cx, |note, cx| note.refresh_note_links(cx));
                    }
                }
                DocumentEditorEvent::OpenNote {
                    note_id,
                    source_offset,
                } => {
                    if let Some(note) = this.workspace.notes.iter().find(|note| note.id == *note_id)
                    {
                        let project_id = note.project_id;
                        let title = note.title.clone();
                        this.open_note_tab(*note_id, project_id, title, window, cx);
                        if let Some(offset) = source_offset
                            && let Some(view) = this.tabs.note_views.get(note_id)
                        {
                            view.update(cx, |editor, cx| {
                                editor.navigate_to_offset(*offset, window, cx)
                            });
                        }
                    }
                }
                DocumentEditorEvent::OpenWorkspaceTarget(target) => {
                    this.open_workspace_target(*target, window, cx);
                }
            },
        )
        .detach();
    }

    fn observe_settings_document(
        view: &Entity<SettingsDocumentView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.subscribe_in(
            view,
            window,
            |this, view, event: &SettingsDocumentEvent, _, cx| {
                if matches!(event, SettingsDocumentEvent::StateChanged) {
                    cx.notify();
                }
                if matches!(event, SettingsDocumentEvent::Applied)
                    && matches!(view.read(cx).save_state(), SettingsDocumentSaveState::Saved)
                {
                    let show_sidebar = AppSettings::show_sidebar(cx);
                    this.sidebar.update(cx, |sidebar, cx| {
                        sidebar.set_width(AppSettings::sidebar_width(cx), cx);
                    });
                    this.set_sidebar_visible(show_sidebar, cx);
                    cx.notify();
                }
            },
        )
        .detach();
    }

    fn observe_command_palette(
        view: &Entity<CommandPaletteView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.subscribe_in(
            view,
            window,
            |this, _, event: &CommandPaletteEvent, window, cx| match event {
                CommandPaletteEvent::Closed => this.focus_handle.focus(window, cx),
                CommandPaletteEvent::OpenNote {
                    note_id,
                    project_id,
                    title,
                } => this.open_note_tab(*note_id, *project_id, title.clone(), window, cx),
                CommandPaletteEvent::NewNote { project_id, title } => {
                    this.create_note_with_title(*project_id, title.clone(), window, cx)
                }
                CommandPaletteEvent::OpenFile => this.import_file(window, cx),
                CommandPaletteEvent::ImportWorkspace => this.import_workspace(window, cx),
                CommandPaletteEvent::ExportWorkspace => this.export_workspace(window, cx),
                CommandPaletteEvent::NewTab => this.new_tab(window, cx),
                CommandPaletteEvent::CloseAllTabs => this.close_all_tabs(window, cx),
                CommandPaletteEvent::OpenSettings => this.open_settings(window, cx),
                CommandPaletteEvent::OpenSettingsFile => this.open_settings_document(window, cx),
                CommandPaletteEvent::OpenSearchResult(result) => {
                    let target = ::workspace::WorkspaceNavigationTarget::Note {
                        note_id: result.open_id,
                        source_offset: None,
                    };
                    this.open_workspace_target(target, window, cx);
                }
            },
        )
        .detach();
    }

    fn new(window: &mut Window, integration: ShellIntegration, cx: &mut Context<Self>) -> Self {
        let tab_session = AppSettings::tab_session(cx);
        let sidebar = SidebarView::view(window, cx);
        let mut open_tabs = Vec::with_capacity(tab_session.tabs.len().max(1));
        let mut note_views = HashMap::new();
        let mut next_tab_id = 1_u64;
        for stored_tab in tab_session.tabs {
            let (title, kind) = match stored_tab {
                StoredTab::Chooser => (SharedString::from("Home"), OpenTabKind::Chooser),
                StoredTab::Trash => (SharedString::from("Trash"), OpenTabKind::Trash),
                StoredTab::Note {
                    note_id,
                    project_id,
                    title,
                } => {
                    let view = DocumentEditorView::view(note_id, window, cx);
                    Self::observe_document_editor(&view, window, cx);
                    note_views.insert(note_id, view.clone());
                    (
                        SharedString::from(title),
                        OpenTabKind::Note {
                            note_id,
                            project_id,
                            view,
                        },
                    )
                }
            };
            open_tabs.push(OpenTab {
                id: next_tab_id,
                title,
                kind,
            });
            next_tab_id = next_tab_id.saturating_add(1);
        }
        if open_tabs.is_empty() {
            open_tabs.push(OpenTab {
                id: next_tab_id,
                title: "Home".into(),
                kind: OpenTabKind::Chooser,
            });
            next_tab_id = next_tab_id.saturating_add(1);
        }
        let active_tab_index = tab_session.active_tab_index.min(open_tabs.len() - 1);
        let active_title = open_tabs[active_tab_index].title.to_string();
        let tab_scroll_handle = ScrollHandle::new();
        tab_scroll_handle.scroll_to_item(active_tab_index);
        let title_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Home")
                .default_value(active_title)
        });

        let command_palette = CommandPaletteView::view(window, cx);
        Self::observe_command_palette(&command_palette, window, cx);
        let trash_search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search Trash..."));

        cx.subscribe(&title_input, |this, input, event: &InputEvent, cx| {
            if !matches!(event, InputEvent::Change) || this.suppress_title_event {
                return;
            }

            let title = input.read(cx).text().to_string();
            this.rename_active_tab(title, cx);
        })
        .detach();

        cx.subscribe(
            &trash_search_input,
            |this, input, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.trash.query = input.read(cx).text().to_string();
                    cx.notify();
                }
            },
        )
        .detach();

        cx.subscribe_in(
            &sidebar,
            window,
            |this, _, event: &SidebarEvent, window, cx| match event {
                SidebarEvent::OpenHome => this.open_home(window, cx),
                SidebarEvent::OpenTrash => this.open_trash(window, cx),
                SidebarEvent::OpenThemeSwitcher => this.open_theme_switcher(window, cx),
                SidebarEvent::ImportFile => this.import_file(window, cx),
                SidebarEvent::WidthChanged => cx.notify(),
                SidebarEvent::WorkspaceChanged => {
                    this.load_home(cx);
                    this.load_trash(cx);
                    this.refresh_workspace(cx);
                }
                SidebarEvent::OpenNote {
                    note_id,
                    project_id,
                    title,
                } => {
                    this.workspace.active_project_id = *project_id;
                    this.open_note_tab(*note_id, *project_id, title.clone(), window, cx);
                }
                SidebarEvent::ActivateProject { project_id } => {
                    this.activate_project(*project_id, window, cx);
                }
                SidebarEvent::NoteRenamed { note_id, title } => {
                    let mut renamed_active = false;
                    for (i, tab) in this.tabs.open_tabs.iter_mut().enumerate() {
                        if let OpenTabKind::Note { note_id: id, view, .. } = &tab.kind
                            && *id == *note_id
                        {
                            tab.title = title.clone();
                            renamed_active = i == this.tabs.active_tab_index;
                            let view = view.clone();
                            view.update(cx, |note, cx| {
                                note.apply_title(title, cx);
                            });
                            break;
                        }
                    }
                    if renamed_active {
                        this.sync_title_input(window, cx);
                    }
                    if let Some(note) = this.workspace.notes.iter_mut().find(|note| note.id == *note_id) {
                        note.title = title.clone();
                    }
                    this.command_palette.update(cx, |palette, cx| {
                        palette.rename_note(*note_id, title.clone(), cx)
                    });
                    this.persist_tab_session(cx);
                    cx.notify();
                }
                SidebarEvent::NotePathChanged { note_id, file_path } => {
                    if let Some(view) = this.tabs.open_tabs.iter().find_map(|tab| match &tab.kind {
                        OpenTabKind::Note {
                            note_id: open_note_id,
                            view,
                            ..
                        } if open_note_id == note_id => Some(view.clone()),
                        _ => None,
                    }) {
                        view.update(cx, |note, cx| {
                            note.apply_file_path(file_path.clone(), cx);
                        });
                    }
                }
                SidebarEvent::NoteDeleted { note_id } => {
                    if let Some(index) = this
                        .tabs.open_tabs
                        .iter()
                        .position(|tab| matches!(&tab.kind, OpenTabKind::Note { note_id: id, .. } if *id == *note_id))
                    {
                        this.close_tab(index, window, cx);
                    }
                }
                SidebarEvent::ProjectRenamed { project_id, name } => {
                    for project in &mut this.workspace.projects {
                        if project.id == *project_id {
                            project.name = name.clone();
                        }
                    }

                    for note in &mut this.workspace.notes {
                        if note.project_id == Some(*project_id) {
                            note.project_name = Some(name.clone());
                        }
                    }

                    this.command_palette.update(cx, |palette, cx| {
                        palette.rename_project(*project_id, name.clone(), cx)
                    });
                    cx.notify();
                }
                SidebarEvent::ProjectDeleted { project_id } => {
                    this.close_project_tabs(*project_id, window, cx);
                    if this.workspace.active_project_id == Some(*project_id) {
                        this.workspace.active_project_id = None;
                    }
                    this.persist_tab_session(cx);
                }
                SidebarEvent::ProjectsReordered => {
                    this.refresh_workspace(cx);
                }
            },
        )
        .detach();

        let sidebar_for_visibility = sidebar.clone();
        let shell_for_sidebar = cx.entity().downgrade();
        let settings_open_file = cx.entity().downgrade();
        let shortcuts = integration._shortcuts.clone();
        let settings_import_workspace = cx.entity().downgrade();
        let settings_export_workspace = cx.entity().downgrade();
        let settings_view = cx.new(|_| {
            SettingsView::new(
                SettingsIntegration::new(
                    move |cx| !sidebar_for_visibility.read(cx).is_collapsed(),
                    move |visible, cx| {
                        if let Some(shell) = shell_for_sidebar.upgrade() {
                            shell.update(cx, |shell, cx| {
                                shell.set_sidebar_visible(visible, cx);
                            });
                        }
                    },
                    move |cx| shortcuts(cx),
                    WorkspaceArchiveActions::new(
                        move |window, cx| {
                            if let Some(shell) = settings_import_workspace.upgrade() {
                                shell.update(cx, |shell, cx| {
                                    shell.import_workspace(window, cx);
                                });
                            }
                        },
                        move |window, cx| {
                            if let Some(shell) = settings_export_workspace.upgrade() {
                                shell.update(cx, |shell, cx| {
                                    shell.export_workspace(window, cx);
                                });
                            }
                        },
                    ),
                )
                .with_open_settings_file(move |window, cx| {
                    if let Some(shell) = settings_open_file.upgrade() {
                        shell.update(cx, |shell, cx| {
                            shell.open_settings_document(window, cx);
                        });
                    }
                }),
            )
        });

        let mut this = Self {
            focus_handle: cx.focus_handle(),
            sidebar,
            settings_view: settings_view.clone(),
            title_input,
            command_palette,
            tabs: TabsState {
                open_tabs,
                note_views,
                active_tab_index,
                next_tab_id,
                tab_scroll_handle,
            },
            workspace: WorkspaceState {
                projects: vec![],
                notes: vec![],
                active_project_id: tab_session.active_project_id,
                refreshing: false,
                refresh_pending: false,
                pending_title_saves: HashMap::new(),
                title_save_lock: Arc::new(tokio::sync::Mutex::new(())),
            },
            suppress_title_event: false,
            window_is_narrow: false,
            home: HomeState {
                data: WorkspaceHomeState::default(),
                phase: LoadPhase::Initial,
                refresh_pending: false,
            },
            trash: TrashState {
                items: Vec::new(),
                phase: LoadPhase::Initial,
                refresh_pending: false,
                search_input: trash_search_input,
                query: String::new(),
                kind_filter: None,
            },
            external_changes: ExternalChangeState {
                task: None,
                revision: None,
                note_revision: None,
                link_revision: None,
            },
            record_opened_task: None,
            workspace_archive_busy: false,
        };

        let show_sidebar = AppSettings::show_sidebar(cx);
        this.sidebar.update(cx, |sidebar, cx| {
            sidebar.set_collapsed(!show_sidebar, cx);
        });
        this.sync_sidebar_with_window_width(window.bounds().size.width, cx);
        cx.observe_window_bounds(window, |this, window, cx| {
            this.sync_sidebar_with_window_width(window.bounds().size.width, cx);
        })
        .detach();
        cx.on_app_quit(|this, cx| {
            let title_flush = this.flush_pending_workspace_title_saves(cx);
            let settings_flush = AppSettings::flush(cx);
            async move {
                tokio::join!(title_flush, settings_flush);
            }
        })
        .detach();
        this.start_external_change_watcher(window, cx);
        this.start_note_link_reindex(cx);
        this.refresh_workspace(cx);
        this.sync_sidebar_active(cx);
        this.load_home(cx);
        this.load_trash(cx);
        this
    }
}
