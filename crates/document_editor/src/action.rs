use gpui_kit::Action;
use serde::Deserialize;

gpui_kit::actions!(
    document_editor,
    [
        FormatDocument,
        SaveDocumentFile,
        SaveDocumentFileAs,
        ToggleDocumentPreview,
        MoveLineUp,
        MoveLineDown,
        ToggleTask,
        ToggleFocusMode,
        ToggleTypewriterScrolling,
        ToggleZenMode,
        ToggleZenStatusBar,
    ]
);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = document_editor, no_json)]
pub struct ApplyMarkdownFormat(pub MarkdownFormat);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = document_editor, no_json)]
pub struct ExpandEmmet;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = document_editor, no_json)]
pub struct EmmetSubmitWrap;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = document_editor, no_json)]
pub struct EmmetCancelWrap;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = document_editor, no_json)]
pub struct ToggleDocumentOutline;

gpui_kit::actions!(
    document_outline,
    [
        OutlinePrevious,
        OutlineNext,
        OutlineLeft,
        OutlineRight,
        OutlineOpen,
        OutlineClose
    ]
);

#[derive(Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum MarkdownFormat {
    HeadingOne,
    HeadingTwo,
    HeadingThree,
    HeadingFour,
    HeadingFive,
    HeadingSix,
    Bold,
    Italic,
    InlineCode,
    Link,
    Task,
    Footnote,
    Strikethrough,
    Highlight,
    BulletList,
    OrderedList,
    Quote,
    CodeBlock,
}


