use objc2::rc::Retained;
use objc2::ClassType;
use objc2_app_kit::NSView;
use objc2_foundation::{MainThreadMarker, NSObjectProtocol, NSString};
use objc2_quartz_core::{CALayer, CAMetalLayer};

use crate::platform::macos::MetalSurfaceContentsGravityStatus;

pub(super) fn set_contents_top_left(
    view: &NSView,
    _mtm: MainThreadMarker,
) -> MetalSurfaceContentsGravityStatus {
    // SAFETY: The marker proves this runs on AppKit's main thread, and winit
    // keeps the view alive for the duration of the call.
    let Some(root_layer) = (unsafe { view.layer() }) else {
        return MetalSurfaceContentsGravityStatus::NotFound;
    };
    set_layer_contents_top_left(root_layer)
}

fn set_layer_contents_top_left(root_layer: Retained<CALayer>) -> MetalSurfaceContentsGravityStatus {
    set_layer_contents_top_left_with(root_layer, set_metal_layer_contents_top_left)
}

fn set_layer_contents_top_left_with(
    root_layer: Retained<CALayer>,
    mut apply: impl FnMut(&CALayer) -> MetalSurfaceContentsGravityStatus,
) -> MetalSurfaceContentsGravityStatus {
    if is_metal_layer(&root_layer) {
        return apply(&root_layer);
    }

    // SAFETY: `CALayer::sublayers` returns an optional retained array of
    // CALayer objects. The retained root stays alive while the array is read.
    let Some(sublayers) = (unsafe { root_layer.sublayers() }) else {
        return MetalSurfaceContentsGravityStatus::NotFound;
    };
    let mut found = false;
    let mut apply_failed = false;
    for layer in sublayers.iter_retained().filter(|layer| is_metal_layer(layer)) {
        found = true;
        apply_failed |= apply(&layer) != MetalSurfaceContentsGravityStatus::Applied;
    }
    if apply_failed {
        MetalSurfaceContentsGravityStatus::ApplyFailed
    } else if found {
        MetalSurfaceContentsGravityStatus::Applied
    } else {
        MetalSurfaceContentsGravityStatus::NotFound
    }
}

fn set_metal_layer_contents_top_left(layer: &CALayer) -> MetalSurfaceContentsGravityStatus {
    let gravity = NSString::from_str(upper_left_gravity_name(layer.contentsAreFlipped()));
    layer.setContentsGravity(&gravity);
    if layer.contentsGravity().as_ref() == &*gravity {
        MetalSurfaceContentsGravityStatus::Applied
    } else {
        MetalSurfaceContentsGravityStatus::ApplyFailed
    }
}

fn is_metal_layer(layer: &CALayer) -> bool {
    layer.isKindOfClass(CAMetalLayer::class())
}

fn upper_left_gravity_name(contents_are_flipped: bool) -> &'static str {
    if contents_are_flipped {
        "bottomLeft"
    } else {
        "topLeft"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_metal_layer() -> Retained<CAMetalLayer> {
        // SAFETY: The class method creates an autoreleased CAMetalLayer, and
        // the binding retains it in the returned `Retained`.
        unsafe { CAMetalLayer::layer() }
    }

    #[test]
    fn test_set_layer_contents_top_left_when_root_is_metal_then_applies_gravity() {
        let metal_layer = new_metal_layer();
        let root_layer = Retained::into_super(metal_layer.clone());

        assert_eq!(
            set_layer_contents_top_left(root_layer),
            MetalSurfaceContentsGravityStatus::Applied
        );
        assert_top_left_gravity(&metal_layer);
    }

    #[test]
    fn test_set_layer_contents_top_left_when_root_is_metal_then_does_not_change_child() {
        let root_metal_layer = new_metal_layer();
        let child_metal_layer = new_metal_layer();
        root_metal_layer.addSublayer(&child_metal_layer);
        let root_layer = Retained::into_super(root_metal_layer.clone());

        assert_eq!(
            set_layer_contents_top_left(root_layer),
            MetalSurfaceContentsGravityStatus::Applied
        );
        assert_top_left_gravity(&root_metal_layer);
        assert_not_top_left_gravity(&child_metal_layer);
    }

    #[test]
    fn test_set_layer_contents_top_left_when_one_direct_metal_sublayer_then_applies_gravity() {
        let root_layer = CALayer::new();
        root_layer.addSublayer(&CALayer::new());
        let metal_layer = new_metal_layer();
        root_layer.addSublayer(&metal_layer);

        assert_eq!(
            set_layer_contents_top_left(root_layer),
            MetalSurfaceContentsGravityStatus::Applied
        );
        assert_top_left_gravity(&metal_layer);
    }

    #[test]
    fn test_set_layer_contents_top_left_when_no_metal_layer_then_returns_not_found() {
        let root_layer = CALayer::new();
        root_layer.addSublayer(&CALayer::new());

        assert_eq!(
            set_layer_contents_top_left(root_layer),
            MetalSurfaceContentsGravityStatus::NotFound
        );
    }

    #[test]
    fn test_set_layer_contents_top_left_when_metal_layer_is_nested_then_returns_not_found() {
        let root_layer = CALayer::new();
        let child_layer = CALayer::new();
        let nested_metal_layer = new_metal_layer();
        child_layer.addSublayer(&nested_metal_layer);
        root_layer.addSublayer(&child_layer);

        assert_eq!(
            set_layer_contents_top_left(root_layer),
            MetalSurfaceContentsGravityStatus::NotFound
        );
        assert_not_top_left_gravity(&nested_metal_layer);
    }

    #[test]
    fn test_set_layer_contents_top_left_when_multiple_direct_metal_sublayers_then_applies_all() {
        let root_layer = CALayer::new();
        let first = new_metal_layer();
        let second = new_metal_layer();
        root_layer.addSublayer(&first);
        root_layer.addSublayer(&second);

        assert_eq!(
            set_layer_contents_top_left(root_layer),
            MetalSurfaceContentsGravityStatus::Applied
        );
        assert_top_left_gravity(&first);
        assert_top_left_gravity(&second);
    }

    #[test]
    fn test_set_layer_contents_top_left_when_one_layer_fails_then_attempts_all_and_returns_failure()
    {
        let root_layer = CALayer::new();
        root_layer.addSublayer(&new_metal_layer());
        root_layer.addSublayer(&new_metal_layer());
        let mut attempts = 0;

        let status = set_layer_contents_top_left_with(root_layer, |_| {
            attempts += 1;
            if attempts == 1 {
                MetalSurfaceContentsGravityStatus::ApplyFailed
            } else {
                MetalSurfaceContentsGravityStatus::Applied
            }
        });

        assert_eq!(attempts, 2);
        assert_eq!(status, MetalSurfaceContentsGravityStatus::ApplyFailed);
    }

    #[test]
    fn test_upper_left_gravity_name_when_contents_are_flipped_then_uses_bottom_left() {
        assert_eq!(upper_left_gravity_name(true), "bottomLeft");
        assert_eq!(upper_left_gravity_name(false), "topLeft");
    }

    fn assert_top_left_gravity(layer: &CALayer) {
        let expected = NSString::from_str(upper_left_gravity_name(layer.contentsAreFlipped()));
        assert_eq!(layer.contentsGravity().as_ref(), &*expected);
    }

    fn assert_not_top_left_gravity(layer: &CALayer) {
        let expected = NSString::from_str(upper_left_gravity_name(layer.contentsAreFlipped()));
        assert_ne!(layer.contentsGravity().as_ref(), &*expected);
    }
}
