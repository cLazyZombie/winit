#[path = "../src/platform_impl/ios/scene_lifecycle.rs"]
mod scene_lifecycle;

use scene_lifecycle::{SceneLifecycle, Transition};

#[test]
fn test_scene_lifecycle_when_first_scene_activates_then_resumes_once() {
    let mut state = SceneLifecycle::default();
    assert_eq!(state.update("main".into(), true), Some(Transition::Resumed));
    assert_eq!(state.update("main".into(), true), None);
    assert_eq!(state.update("main".into(), false), Some(Transition::Suspended));
    assert_eq!(state.update("main".into(), false), None);
    assert_eq!(state.update("main".into(), true), Some(Transition::Resumed));
}

#[test]
fn test_scene_lifecycle_when_scenes_overlap_then_only_last_deactivation_suspends() {
    let mut state = SceneLifecycle::default();
    assert_eq!(state.update("first".into(), true), Some(Transition::Resumed));
    assert_eq!(state.update("second".into(), true), None);
    assert_eq!(state.update("first".into(), false), None);
    assert_eq!(state.update("second".into(), false), Some(Transition::Suspended));
}

#[test]
fn test_scene_lifecycle_when_unknown_scene_disconnects_then_active_scene_is_preserved() {
    let mut state = SceneLifecycle::default();
    assert_eq!(state.update("unknown".into(), false), None);
    assert_eq!(state.update("main".into(), true), Some(Transition::Resumed));
    assert_eq!(state.update("unknown".into(), false), None);
    assert_eq!(state.update("main".into(), false), Some(Transition::Suspended));
}
