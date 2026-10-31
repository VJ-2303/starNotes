# starNotes — plan

> Editable **intent** file: ideas + plans — what we *mean to do*.
> This is NOT the event log. `events.jsonl` -> `summary.md` records what
> *happened*; this file records what we *intend*. The AI reads it at
> session start and edits it directly (like `PROJECT_MAP.md`): add ideas
> and plans, check items off, move done work down to Shipped. Plans are
> never logged as events.

## Ideas
_Loose thoughts, not yet committed to._

## Active plans
_What we're working toward now. Use `- [ ]` / `- [x]` checklists._

## Next
_Queued, but not started._

## Someday / maybe

## Shipped
_Move completed plans here so the top stays about the future._

- [x] Switched global allocator to `mimalloc` to curb glibc fragmentation and drop memory footprint on Linux.
- [x] Removed Vim emulation mode:
  - Deleted `crates/document_editor/src/vim` module and tests (~4,800 lines).
  - Cleaned Vim key bindings, actions, and contexts from `crates/app/src/keymap.rs`.
  - Cleaned editor overlays, selection rendering, and status bar mode indicator.
  - Removed `editor_vim_mode` setting from `crates/settings` and `settings.json`.
- [x] Pure note-taking app refactor:
  - Pruned dead inspector/UI board code.
  - Pruned dead reminders module.
  - Pruned 15 dead archive structs (board/list/card/view/workflow/recurring task).
  - Pruned workspace links model and reference parsing to pure note targets.
  - Removed board/card/embed completion candidates and slash commands from wikilinks.
  - Updated metadata, desktop entries, and documentation.

