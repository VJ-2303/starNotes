use gpui_kit::{App, AsKeystroke as _, Global, KeyBinding, SharedString};
use settings::{SaveSettingsDocument, ShortcutReference};

use command_palette::{
    CloseCommandPaletteAction, CommandPaletteAction, OpenWorkspaceSearchAction,
    SelectNextCommandPaletteItem, SelectPrevCommandPaletteItem, SwitchThemeAction,
};
use document_editor::action::{
    ApplyMarkdownFormat, EmmetCancelWrap, EmmetSubmitWrap, ExpandEmmet, FormatDocument,
    MarkdownFormat, MoveLineDown, MoveLineUp, OutlineClose, OutlineLeft, OutlineNext, OutlineOpen,
    OutlinePrevious, OutlineRight, SaveDocumentFile, SaveDocumentFileAs, ToggleDocumentOutline,
    ToggleDocumentPreview, ToggleFocusMode, ToggleTask, ToggleTypewriterScrolling, ToggleZenMode,
};
use shell::{CycleNextTab, CyclePrevTab, OpenSettingsAction, ToggleSidebarAction};

struct ShortcutRegistry(Vec<ShortcutReference>);

impl Global for ShortcutRegistry {}

pub fn shortcuts(cx: &App) -> &[ShortcutReference] {
    &cx.global::<ShortcutRegistry>().0
}

pub fn init(cx: &mut App) {
    let bindings = default_bindings();

    let shortcuts = bindings
        .iter()
        .map(|binding| ShortcutReference {
            action: shortcut_action_name(binding),
            context: binding
                .predicate()
                .map(|predicate| predicate.to_string().into())
                .unwrap_or_else(|| "Global".into()),
            keystrokes: binding
                .keystrokes()
                .iter()
                .map(|stroke| stroke.as_keystroke().clone())
                .collect(),
        })
        .collect();

    cx.set_global(ShortcutRegistry(shortcuts));
    cx.bind_keys(bindings);
}

fn default_bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("ctrl-tab", CycleNextTab, Some("AppShell")),
        KeyBinding::new("ctrl-shift-tab", CyclePrevTab, Some("AppShell")),
        KeyBinding::new("ctrl-p", CommandPaletteAction, Some("AppShell")),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-,", OpenSettingsAction, Some("AppShell")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-,", OpenSettingsAction, Some("AppShell")),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-f", OpenWorkspaceSearchAction, Some("AppShell")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-f", OpenWorkspaceSearchAction, Some("AppShell")),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-alt-t", SwitchThemeAction, Some("AppShell")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-alt-t", SwitchThemeAction, Some("AppShell")),
        KeyBinding::new("escape", CloseCommandPaletteAction, Some("AppShell")),
        KeyBinding::new("escape", CloseCommandPaletteAction, Some("CommandPalette")),
        KeyBinding::new("up", SelectPrevCommandPaletteItem, Some("CommandPalette")),
        KeyBinding::new("down", SelectNextCommandPaletteItem, Some("CommandPalette")),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-b", ToggleSidebarAction, Some("AppShell")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-b", ToggleSidebarAction, Some("AppShell")),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-alt-e", ExpandEmmet, Some("MarkdownSource")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-alt-e", ExpandEmmet, Some("MarkdownSource")),
        #[cfg(target_os = "macos")]
        KeyBinding::new(
            "cmd-b",
            ApplyMarkdownFormat(MarkdownFormat::Bold),
            Some("MarkdownSource"),
        ),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new(
            "ctrl-b",
            ApplyMarkdownFormat(MarkdownFormat::Bold),
            Some("MarkdownSource"),
        ),
        #[cfg(target_os = "macos")]
        KeyBinding::new(
            "cmd-i",
            ApplyMarkdownFormat(MarkdownFormat::Italic),
            Some("MarkdownSource"),
        ),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new(
            "ctrl-i",
            ApplyMarkdownFormat(MarkdownFormat::Italic),
            Some("MarkdownSource"),
        ),
        #[cfg(target_os = "macos")]
        KeyBinding::new(
            "cmd-k",
            ApplyMarkdownFormat(MarkdownFormat::Link),
            Some("MarkdownSource"),
        ),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new(
            "ctrl-k",
            ApplyMarkdownFormat(MarkdownFormat::Link),
            Some("MarkdownSource"),
        ),
        #[cfg(target_os = "macos")]
        KeyBinding::new(
            "cmd-e",
            ApplyMarkdownFormat(MarkdownFormat::InlineCode),
            Some("MarkdownSource"),
        ),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new(
            "ctrl-e",
            ApplyMarkdownFormat(MarkdownFormat::InlineCode),
            Some("MarkdownSource"),
        ),
        #[cfg(target_os = "macos")]
        KeyBinding::new(
            "cmd-alt-1",
            ApplyMarkdownFormat(MarkdownFormat::HeadingOne),
            Some("MarkdownSource"),
        ),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new(
            "ctrl-alt-1",
            ApplyMarkdownFormat(MarkdownFormat::HeadingOne),
            Some("MarkdownSource"),
        ),
        #[cfg(target_os = "macos")]
        KeyBinding::new(
            "cmd-alt-2",
            ApplyMarkdownFormat(MarkdownFormat::HeadingTwo),
            Some("MarkdownSource"),
        ),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new(
            "ctrl-alt-2",
            ApplyMarkdownFormat(MarkdownFormat::HeadingTwo),
            Some("MarkdownSource"),
        ),
        #[cfg(target_os = "macos")]
        KeyBinding::new(
            "cmd-alt-3",
            ApplyMarkdownFormat(MarkdownFormat::HeadingThree),
            Some("MarkdownSource"),
        ),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new(
            "ctrl-alt-3",
            ApplyMarkdownFormat(MarkdownFormat::HeadingThree),
            Some("MarkdownSource"),
        ),
        #[cfg(target_os = "macos")]
        KeyBinding::new(
            "cmd-alt-4",
            ApplyMarkdownFormat(MarkdownFormat::HeadingFour),
            Some("MarkdownSource"),
        ),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new(
            "ctrl-alt-4",
            ApplyMarkdownFormat(MarkdownFormat::HeadingFour),
            Some("MarkdownSource"),
        ),
        #[cfg(target_os = "macos")]
        KeyBinding::new(
            "cmd-alt-5",
            ApplyMarkdownFormat(MarkdownFormat::HeadingFive),
            Some("MarkdownSource"),
        ),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new(
            "ctrl-alt-5",
            ApplyMarkdownFormat(MarkdownFormat::HeadingFive),
            Some("MarkdownSource"),
        ),
        #[cfg(target_os = "macos")]
        KeyBinding::new(
            "cmd-alt-6",
            ApplyMarkdownFormat(MarkdownFormat::HeadingSix),
            Some("MarkdownSource"),
        ),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new(
            "ctrl-alt-6",
            ApplyMarkdownFormat(MarkdownFormat::HeadingSix),
            Some("MarkdownSource"),
        ),
        #[cfg(target_os = "macos")]
        KeyBinding::new(
            "cmd-shift-7",
            ApplyMarkdownFormat(MarkdownFormat::OrderedList),
            Some("MarkdownSource"),
        ),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new(
            "ctrl-shift-7",
            ApplyMarkdownFormat(MarkdownFormat::OrderedList),
            Some("MarkdownSource"),
        ),
        #[cfg(target_os = "macos")]
        KeyBinding::new(
            "cmd-shift-8",
            ApplyMarkdownFormat(MarkdownFormat::BulletList),
            Some("MarkdownSource"),
        ),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new(
            "ctrl-shift-8",
            ApplyMarkdownFormat(MarkdownFormat::BulletList),
            Some("MarkdownSource"),
        ),
        #[cfg(target_os = "macos")]
        KeyBinding::new(
            "cmd-shift-.",
            ApplyMarkdownFormat(MarkdownFormat::Quote),
            Some("MarkdownSource"),
        ),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new(
            "ctrl-shift-.",
            ApplyMarkdownFormat(MarkdownFormat::Quote),
            Some("MarkdownSource"),
        ),
        #[cfg(target_os = "macos")]
        KeyBinding::new(
            "cmd-alt-c",
            ApplyMarkdownFormat(MarkdownFormat::CodeBlock),
            Some("MarkdownSource"),
        ),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new(
            "ctrl-alt-c",
            ApplyMarkdownFormat(MarkdownFormat::CodeBlock),
            Some("MarkdownSource"),
        ),
        #[cfg(target_os = "macos")]
        KeyBinding::new(
            "cmd-shift-x",
            ApplyMarkdownFormat(MarkdownFormat::Strikethrough),
            Some("MarkdownSource"),
        ),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new(
            "ctrl-shift-x",
            ApplyMarkdownFormat(MarkdownFormat::Strikethrough),
            Some("MarkdownSource"),
        ),
        #[cfg(target_os = "macos")]
        KeyBinding::new(
            "cmd-shift-h",
            ApplyMarkdownFormat(MarkdownFormat::Highlight),
            Some("MarkdownSource"),
        ),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new(
            "ctrl-shift-h",
            ApplyMarkdownFormat(MarkdownFormat::Highlight),
            Some("MarkdownSource"),
        ),
        #[cfg(target_os = "macos")]
        KeyBinding::new(
            "cmd-shift-9",
            ApplyMarkdownFormat(MarkdownFormat::Task),
            Some("MarkdownSource"),
        ),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new(
            "ctrl-shift-9",
            ApplyMarkdownFormat(MarkdownFormat::Task),
            Some("MarkdownSource"),
        ),
        #[cfg(target_os = "macos")]
        KeyBinding::new(
            "cmd-alt-f",
            ApplyMarkdownFormat(MarkdownFormat::Footnote),
            Some("MarkdownSource"),
        ),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new(
            "ctrl-alt-f",
            ApplyMarkdownFormat(MarkdownFormat::Footnote),
            Some("MarkdownSource"),
        ),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-space", ToggleTask, Some("MarkdownSource")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-space", ToggleTask, Some("MarkdownSource")),
        KeyBinding::new("alt-up", MoveLineUp, Some("DocumentEditor")),
        KeyBinding::new("alt-down", MoveLineDown, Some("DocumentEditor")),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-s", SaveDocumentFile, Some("DocumentEditor")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-s", SaveDocumentFile, Some("DocumentEditor")),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-s", SaveDocumentFileAs, Some("DocumentEditor")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-s", SaveDocumentFileAs, Some("DocumentEditor")),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-s", SaveSettingsDocument, Some("SettingsDocument")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-s", SaveSettingsDocument, Some("SettingsDocument")),
        KeyBinding::new("alt-shift-f", FormatDocument, Some("DocumentEditor")),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-alt-m", ToggleFocusMode, Some("DocumentEditor")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-alt-m", ToggleFocusMode, Some("DocumentEditor")),
        #[cfg(target_os = "macos")]
        KeyBinding::new(
            "cmd-alt-w",
            ToggleTypewriterScrolling,
            Some("DocumentEditor"),
        ),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new(
            "ctrl-alt-w",
            ToggleTypewriterScrolling,
            Some("DocumentEditor"),
        ),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-alt-z", ToggleZenMode, Some("DocumentEditor")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-alt-z", ToggleZenMode, Some("DocumentEditor")),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-v", ToggleDocumentPreview, Some("DocumentEditor")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new(
            "ctrl-shift-v",
            ToggleDocumentPreview,
            Some("DocumentEditor"),
        ),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-v", ToggleDocumentPreview, Some("TextView")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-v", ToggleDocumentPreview, Some("TextView")),
        KeyBinding::new("enter", EmmetSubmitWrap, Some("EmmetInput")),
        KeyBinding::new("escape", EmmetCancelWrap, Some("EmmetInput")),
        KeyBinding::new(
            "ctrl-shift-o",
            ToggleDocumentOutline,
            Some("DocumentEditor"),
        ),
        KeyBinding::new("up", OutlinePrevious, Some("DocumentOutline")),
        KeyBinding::new("down", OutlineNext, Some("DocumentOutline")),
        KeyBinding::new("left", OutlineLeft, Some("DocumentOutline")),
        KeyBinding::new("right", OutlineRight, Some("DocumentOutline")),
        KeyBinding::new("enter", OutlineOpen, Some("DocumentOutline")),
        KeyBinding::new("escape", OutlineClose, Some("DocumentOutline")),
    ]
}

fn shortcut_action_name(binding: &KeyBinding) -> SharedString {
    if let Some(action) = binding
        .action()
        .as_any()
        .downcast_ref::<ApplyMarkdownFormat>()
    {
        return humanize_identifier(match action.0 {
            MarkdownFormat::HeadingOne => "HeadingOne",
            MarkdownFormat::HeadingTwo => "HeadingTwo",
            MarkdownFormat::HeadingThree => "HeadingThree",
            MarkdownFormat::HeadingFour => "HeadingFour",
            MarkdownFormat::HeadingFive => "HeadingFive",
            MarkdownFormat::HeadingSix => "HeadingSix",
            MarkdownFormat::Bold => "Bold",
            MarkdownFormat::Italic => "Italic",
            MarkdownFormat::InlineCode => "InlineCode",
            MarkdownFormat::Link => "Link",
            MarkdownFormat::Task => "Task",
            MarkdownFormat::Footnote => "Footnote",
            MarkdownFormat::Strikethrough => "Strikethrough",
            MarkdownFormat::Highlight => "Highlight",
            MarkdownFormat::BulletList => "BulletList",
            MarkdownFormat::OrderedList => "OrderedList",
            MarkdownFormat::Quote => "Quote",
            MarkdownFormat::CodeBlock => "CodeBlock",
        });
    }

    let name = binding
        .action()
        .name()
        .rsplit("::")
        .next()
        .unwrap_or(binding.action().name())
        .strip_suffix("Action")
        .unwrap_or_else(|| {
            binding
                .action()
                .name()
                .rsplit("::")
                .next()
                .unwrap_or(binding.action().name())
        });

    humanize_identifier(name)
}

pub(crate) fn humanize_identifier(value: &str) -> SharedString {
    let mut label = String::with_capacity(value.len() + 4);
    let mut previous_is_lowercase = false;

    for character in value.chars() {
        if character == '_' || character == '-' {
            if !label.ends_with(' ') {
                label.push(' ');
            }
            previous_is_lowercase = false;
            continue;
        }

        if character.is_uppercase() && previous_is_lowercase {
            label.push(' ');
        }
        label.push(character);
        previous_is_lowercase = character.is_lowercase();
    }

    label.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_shortcut_does_not_shadow_markdown_link() {
        let bindings = default_bindings();
        let theme = bindings
            .iter()
            .find(|binding| binding.action().as_any().is::<SwitchThemeAction>())
            .expect("theme binding should be registered");
        let link = bindings
            .iter()
            .find(|binding| {
                binding
                    .action()
                    .as_any()
                    .downcast_ref::<ApplyMarkdownFormat>()
                    .is_some_and(|action| action.0 == MarkdownFormat::Link)
            })
            .expect("markdown link binding should be registered");

        assert!(!theme.keystrokes().starts_with(link.keystrokes()));
    }

    #[test]
    fn smart_markdown_shortcuts_are_registered() {
        let bindings = default_bindings();
        let formats = [
            MarkdownFormat::HeadingFour,
            MarkdownFormat::HeadingFive,
            MarkdownFormat::HeadingSix,
            MarkdownFormat::Task,
            MarkdownFormat::Footnote,
            MarkdownFormat::Strikethrough,
            MarkdownFormat::Highlight,
        ];

        for format in formats {
            assert!(bindings.iter().any(|binding| {
                binding
                    .action()
                    .as_any()
                    .downcast_ref::<ApplyMarkdownFormat>()
                    .is_some_and(|action| action.0 == format)
            }));
        }
        assert!(
            bindings
                .iter()
                .any(|binding| binding.action().as_any().is::<MoveLineUp>())
        );
        assert!(
            bindings
                .iter()
                .any(|binding| binding.action().as_any().is::<MoveLineDown>())
        );
        assert!(
            bindings
                .iter()
                .any(|binding| binding.action().as_any().is::<ToggleTask>())
        );
        let focus_mode = bindings
            .iter()
            .find(|binding| binding.action().as_any().is::<ToggleFocusMode>())
            .expect("focus mode binding should be registered");
        let typewriter_scrolling = bindings
            .iter()
            .find(|binding| binding.action().as_any().is::<ToggleTypewriterScrolling>())
            .expect("typewriter scrolling binding should be registered");
        assert_eq!(
            focus_mode.keystrokes(),
            KeyBinding::new(
                if cfg!(target_os = "macos") {
                    "cmd-alt-m"
                } else {
                    "ctrl-alt-m"
                },
                ToggleFocusMode,
                Some("DocumentEditor"),
            )
            .keystrokes()
        );
        assert_eq!(
            typewriter_scrolling.keystrokes(),
            KeyBinding::new(
                if cfg!(target_os = "macos") {
                    "cmd-alt-w"
                } else {
                    "ctrl-alt-w"
                },
                ToggleTypewriterScrolling,
                Some("DocumentEditor"),
            )
            .keystrokes()
        );
        let zen_mode = bindings
            .iter()
            .find(|binding| binding.action().as_any().is::<ToggleZenMode>())
            .expect("zen mode binding should be registered");
        assert_eq!(
            zen_mode.keystrokes(),
            KeyBinding::new(
                if cfg!(target_os = "macos") {
                    "cmd-alt-z"
                } else {
                    "ctrl-alt-z"
                },
                ToggleZenMode,
                Some("DocumentEditor"),
            )
            .keystrokes()
        );

        let toggle_task = bindings
            .iter()
            .find(|binding| binding.action().as_any().is::<ToggleTask>())
            .expect("task toggle binding should be registered");
        let tray_shortcut = KeyBinding::new(
            "ctrl-alt-space",
            ToggleTask,
            Some("MarkdownSource"),
        );
        assert!(
            !toggle_task
                .keystrokes()
                .starts_with(tray_shortcut.keystrokes())
        );

        let expected_shortcut = KeyBinding::new(
            "ctrl-shift-space",
            ToggleTask,
            Some("MarkdownSource"),
        );
        assert_eq!(toggle_task.keystrokes(), expected_shortcut.keystrokes());
    }
}
