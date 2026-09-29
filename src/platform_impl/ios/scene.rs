use std::cell::RefCell;
use std::rc::Rc;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{MainThreadMarker, Message};
use objc2_foundation::{ns_string, NSBundle, NSNotificationCenter, NSObjectProtocol};
use objc2_ui_kit::{
    UIApplication, UICoordinateSpace, UIScene, UISceneActivationState, UIWindow, UIWindowScene,
};

use super::app_state::{self, EventWrapper};
use super::notification_center::create_observer;
use super::scene_lifecycle::{SceneLifecycle, Transition};
use crate::event::Event;

pub(crate) fn enabled() -> bool {
    NSBundle::mainBundle()
        .objectForInfoDictionaryKey(ns_string!("UIApplicationSceneManifest"))
        .is_some()
}

pub(crate) fn attach(window: &UIWindow, mtm: MainThreadMarker) {
    if !enabled() {
        return;
    }
    // Resumed는 scene 활성화 뒤 전달된다. 여러 scene 중 선택도 안정된 session ID 순서를 따른다.
    let mut scenes: Vec<_> = UIApplication::sharedApplication(mtm)
        .connectedScenes()
        .iter()
        .filter_map(|scene| scene.downcast_ref::<UIWindowScene>().map(Message::retain))
        .filter(|scene| scene.activationState() == UISceneActivationState::ForegroundActive)
        .collect();
    scenes.sort_by_key(|scene| scene.session().persistentIdentifier().to_string());
    let scene =
        scenes.first().expect("An active UIWindowScene is required before creating a window");
    window.setWindowScene(Some(scene));
    // effectiveGeometry는 iOS 26부터 제공되므로 이전 지원 버전의 API를 유지한다.
    #[allow(deprecated)]
    window.setFrame(scene.coordinateSpace().bounds());
}

pub(crate) fn observe(
    center: &NSNotificationCenter,
    mtm: MainThreadMarker,
) -> Vec<Retained<ProtocolObject<dyn NSObjectProtocol>>> {
    if !enabled() {
        return Vec::new();
    }
    let state = Rc::new(RefCell::new(SceneLifecycle::default()));
    [
        (ns_string!("UISceneDidActivateNotification"), true),
        (ns_string!("UISceneWillDeactivateNotification"), false),
        (ns_string!("UISceneDidDisconnectNotification"), false),
    ]
    .into_iter()
    .map(|(name, active)| {
        let state = Rc::clone(&state);
        create_observer(center, name, move |notification| {
            let Some(object) = notification.object() else { return };
            let Some(scene) = object.downcast_ref::<UIScene>() else { return };
            let transition = state
                .borrow_mut()
                .update(scene.session().persistentIdentifier().to_string(), active);
            let event = match transition {
                Some(Transition::Resumed) => Event::Resumed,
                Some(Transition::Suspended) => Event::Suspended,
                None => return,
            };
            tracing::info!(?event, "UIKit scene lifecycle transition");
            app_state::handle_nonuser_event(mtm, EventWrapper::StaticEvent(event));
        })
    })
    .collect()
}
