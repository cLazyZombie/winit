use std::collections::BTreeSet;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Transition {
    Resumed,
    Suspended,
}

#[derive(Default)]
pub(crate) struct SceneLifecycle {
    active: BTreeSet<String>,
}

impl SceneLifecycle {
    // UIKit이 중복 알림을 보내거나 여러 scene이 겹쳐 활성화되어도 앱 전환은 한 번만 보낸다.
    pub(crate) fn update(&mut self, id: String, active: bool) -> Option<Transition> {
        let was_active = !self.active.is_empty();
        if active {
            self.active.insert(id);
        } else {
            self.active.remove(&id);
        }
        match (was_active, !self.active.is_empty()) {
            (false, true) => Some(Transition::Resumed),
            (true, false) => Some(Transition::Suspended),
            _ => None,
        }
    }
}
