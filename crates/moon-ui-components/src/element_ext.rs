use gpui::{
    AnyElement, App, Bounds, IntoElement, ParentElement, Pixels, Styled as _, Window, canvas,
};

use crate::{Sizable, Size};

#[derive(Default)]
struct ChildElementOptions {
    ix: usize,
    size: Size,
}

#[allow(patterns_in_fns_without_body)]
pub trait ChildElement: Sizable + IntoElement {
    fn with_ix(mut self, ix: usize) -> Self;
}

/// A type-erased element that can accept a [`AnyChildElementOptions`] before being rendered.
pub struct AnyChildElement(Box<dyn FnOnce(ChildElementOptions) -> AnyElement>);

impl AnyChildElement {
    pub fn new(element: impl ChildElement + 'static) -> Self {
        Self(Box::new(|options| {
            element
                .with_ix(options.ix)
                .with_size(options.size)
                .into_any_element()
        }))
    }

    pub fn into_any(self, ix: usize, size: Size) -> AnyElement {
        (self.0)(ChildElementOptions { ix, size })
    }
}

/// A trait to extend [`gpui::Element`] with additional functionality.
pub trait ElementExt: ParentElement + Sized {
    /// Add a prepaint callback to the element.
    ///
    /// The first argument is a bounds rect in pixels, measured at prepaint time.
    ///
    /// IMPORTANT — it is not necessarily the host's own box. The measurement comes from an
    /// absolutely positioned [`gpui::canvas`] added as a child AT THE POINT THIS IS CALLED, so
    /// builder order decides where it sits among the host's children. `size_full` gives it the
    /// host's SIZE, but its ORIGIN is the static position layout assigns an absolute child with
    /// auto insets:
    ///
    /// - in a FLEX host: the content-box origin, so the rect does match the host;
    /// - in a BLOCK host (gpui's default): advanced past the in-flow siblings that PRECEDE the
    ///   canvas, on the Y axis only — X is never advanced. Call this before adding children and
    ///   the origin is the host's top; call it after, and it is the bottom edge of what came
    ///   before.
    ///
    /// Consumers therefore differ, and correcting this would not be a local cleanup. `Popover`
    /// captures after its trigger and depends on the resulting bottom-edge origin (see
    /// `Popover::resolved_corner` and the menu offset in `moon::dropdown`, whose geometry tests
    /// are the tripwire). `DockArea` captures before its content and gets the top. Callers that
    /// ignore the rect entirely, like `Root`'s sheet sizing, are unaffected either way.
    ///
    /// See also [`gpui::canvas`].
    fn on_prepaint<F>(self, f: F) -> Self
    where
        F: FnOnce(Bounds<Pixels>, &mut Window, &mut App) + 'static,
    {
        self.child(
            canvas(
                move |bounds, window, cx| f(bounds, window, cx),
                |_, _, _, _| {},
            )
            .absolute()
            .size_full(),
        )
    }
}

impl<T: ParentElement> ElementExt for T {}
