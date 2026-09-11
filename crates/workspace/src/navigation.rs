use std::sync::Arc;

use gpui_kit::{App, Context, WeakEntity};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspaceNavigationTarget {
    Note {
        note_id: u32,
        source_offset: Option<usize>,
    },
}

pub type WorkspaceNavigationHandler =
    Arc<dyn Fn(WorkspaceNavigationTarget, &mut App) + Send + Sync>;

pub fn weak_navigation_handler<T: 'static>(
    owner: WeakEntity<T>,
    handle: impl Fn(&mut T, WorkspaceNavigationTarget, &mut Context<T>) + Send + Sync + 'static,
) -> WorkspaceNavigationHandler {
    Arc::new(move |target, cx| {
        let _ = owner.update(cx, |owner, cx| handle(owner, target, cx));
    })
}

impl WorkspaceNavigationTarget {
    pub fn note(note_id: u32) -> Self {
        Self::Note {
            note_id,
            source_offset: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::AppContext as _;

    #[derive(Default)]
    struct NavigationOwner {
        target: Option<WorkspaceNavigationTarget>,
    }

    #[gpui_kit::test]
    fn weak_handler_routes_targets_without_retaining_its_owner(cx: &mut gpui_kit::TestAppContext) {
        let owner = cx.new(|_| NavigationOwner::default());
        let weak_owner = owner.downgrade();
        let handler = weak_navigation_handler(weak_owner.clone(), |owner, target, _| {
            owner.target = Some(target);
        });
        let target = WorkspaceNavigationTarget::note(42);

        cx.update(|cx| handler(target, cx));
        assert_eq!(owner.read_with(cx, |owner, _| owner.target), Some(target));

        drop(owner);
        assert!(weak_owner.upgrade().is_none());
        cx.update(|cx| handler(WorkspaceNavigationTarget::note(7), cx));
    }
}
