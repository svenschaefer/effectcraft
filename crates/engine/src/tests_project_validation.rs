use std::sync::Arc;

use effectcraft_project::{MAX_NEXT_ID, Project};
use serde_json::json;

use crate::Session;

#[test]
fn invalid_replacement_keeps_project_history_and_editor_state() {
    let mut session = Session::default();
    session.execute("file.openDemoProject", json!({})).unwrap();
    session
        .edit("rename", None, |project, _| {
            project.settings.comment = "kept project".into();
            Ok(())
        })
        .unwrap();
    let project = session.project.clone();
    let saved = session.saved_project.clone();
    let state = serde_json::to_value(&session.state).unwrap();
    let revision = session.revision;
    let undo = session.history.undo.len();
    let invalid = Project { next_id: u64::MAX, ..Default::default() };

    assert!(session.replace_project(invalid, Some("bad.ecproj".into())).is_err());
    assert!(Arc::ptr_eq(&session.project, &project));
    assert!(Arc::ptr_eq(&session.saved_project, &saved));
    assert_eq!(serde_json::to_value(&session.state).unwrap(), state);
    assert_eq!(session.revision, revision);
    assert_eq!(session.history.undo.len(), undo);
    assert_eq!(session.path, None);
}

#[test]
fn allocation_exhaustion_rejects_the_whole_edit_without_an_undo_step() {
    let mut session = Session::default();
    Arc::make_mut(&mut session.project).next_id = MAX_NEXT_ID - 1;
    let project = session.project.clone();
    let state = serde_json::to_value(&session.state).unwrap();
    let revision = session.revision;

    let result = session.edit("exhausted allocation", None, |candidate, state| {
        candidate.alloc();
        candidate.settings.comment = "must not commit".into();
        state.snapping = false;
        Ok(())
    });

    assert!(result.is_err());
    assert!(Arc::ptr_eq(&session.project, &project));
    assert_eq!(serde_json::to_value(&session.state).unwrap(), state);
    assert_eq!(session.revision, revision);
    assert!(session.history.undo.is_empty());
}
