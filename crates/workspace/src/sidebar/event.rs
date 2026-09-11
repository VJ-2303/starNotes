use gpui_kit::SharedString;

#[derive(Clone)]
pub enum SidebarEvent {
    OpenHome,
    OpenTrash,
    OpenThemeSwitcher,
    ImportFile,
    WidthChanged,
    WorkspaceChanged,
    OpenNote {
        note_id: u32,
        project_id: Option<u32>,
        title: SharedString,
    },
    ActivateProject {
        project_id: u32,
    },
    NoteRenamed {
        note_id: u32,
        title: SharedString,
    },
    NotePathChanged {
        note_id: u32,
        file_path: Option<String>,
    },
    NoteDeleted {
        note_id: u32,
    },
    ProjectRenamed {
        project_id: u32,
        name: SharedString,
    },
    ProjectDeleted {
        project_id: u32,
    },
    ProjectsReordered,
}
