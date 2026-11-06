//! Test support for the controls the Facet skin restyles: draws one control in a headless
//! window and reports what was painted for it.
//!
//! A unit test can resolve the surface a control should have, but only the scene shows whether
//! `render` painted it, and whether a flat theme still paints what it always has. The probe
//! swaps the headless renderer for a recorder, draws the control with the pointer away from
//! it, over it or pressing it, and reads the control's own fill, border and shadows back out
//! of the recorded scene.
//!
//! Nothing here is compiled outside tests, and nothing is public: the items are visible to the
//! Moon modules only.

use std::{
    borrow::Cow,
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

use gpui::{
    AnyElement, AnyWindowHandle, AppContext as _, AtlasKey, AtlasTile, Background, Bounds,
    BoxShadow, Context, DevicePixels, HeadlessAppContext, Hsla, IntoElement, Modifiers,
    MouseButton, MouseDownEvent, MouseMoveEvent, NoopTextSystem, ParentElement as _, Pixels,
    PlatformAtlas, PlatformHeadlessRenderer, PlatformInput, Quad, Render, ScaledPixels, Scene,
    Shadow, Size, Styled as _, Window, div, point, px, size,
};

use crate::moon::{MoonFacet, MoonTheme, MoonThemeConfig};

/// Distance from the window's top-left corner to the probed control, in pixels: room for the
/// control's shadows and glow, so that none of them is clipped by the window.
const MARGIN: f32 = 60.0;

/// How far a recorded edge may sit from the laid-out one and still be the same edge, in device
/// pixels. Painting snaps bounds to whole device pixels; layout does not.
const SNAP_TOLERANCE: f32 = 1.0;

/// Frames drawn after the window opens and after each pointer event, so that state a frame
/// hands to the next one, such as a hover or a press, has reached the scene that is read.
const SETTLE_FRAMES: usize = 3;

/// Where the pointer is while the probed control is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::moon) enum Pointer {
    /// Outside the control: its resting look.
    Away,
    /// Over the middle of the control, with no button held.
    Over,
    /// Over the middle of the control, with the left button held down.
    Pressed,
}

/// What a control painted for itself, as opposed to what its children painted.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::moon) struct Painted {
    /// The control's fill. `None` when it painted no fill, or a fully transparent one.
    pub(in crate::moon) fill: Option<Background>,
    /// The colour of the control's border. `None` when it painted no border.
    pub(in crate::moon) border: Option<Hsla>,
    /// The control's own shadows in reference pixels: the ones inside it first, then the ones
    /// outside it, each group in the order it was painted. This is the order a
    /// [`MoonFacet`](crate::moon::MoonFacet) lists its shadow stack in.
    pub(in crate::moon) shadows: Vec<BoxShadow>,
}

impl Painted {
    /// What a control paints for a faceted surface: its fill, its edge as the border and its
    /// whole shadow stack.
    pub(in crate::moon) fn facet(surface: MoonFacet) -> Self {
        Self {
            fill: Some(surface.fill),
            border: Some(surface.edge),
            shadows: surface.shadows,
        }
    }

    /// What a control paints for a flat fill and border with no shadow. A fully transparent
    /// colour is read back as nothing painted.
    pub(in crate::moon) fn flat(fill: Hsla, border: Hsla) -> Self {
        Self {
            fill: (!fill.is_transparent()).then(|| Background::from(fill)),
            border: (!border.is_transparent()).then_some(border),
            shadows: Vec::new(),
        }
    }
}

/// Draw the control `build` returns under the theme `config` and report what it painted.
///
/// The control is the only child of a window-sized row and keeps its own size. Each call opens
/// its own headless application, so calls do not share a theme or a pointer.
///
/// Args:
///     config: The theme to install before the control is drawn.
///     pointer: Where the pointer is while the control is drawn.
///     build: Builds the control; called once per frame.
///
/// Returns:
///     The fill, border colour and shadows of the control's own box.
///
/// Panics:
///     When the headless window cannot be opened or drawn, or the control was never laid out.
pub(in crate::moon) fn painted<E: IntoElement>(
    config: MoonThemeConfig,
    pointer: Pointer,
    build: impl Fn() -> E + 'static,
) -> Painted {
    let scene = Rc::new(RefCell::new(RecordedScene::default()));
    let recorded = scene.clone();
    let mut cx = HeadlessAppContext::with_platform(
        Arc::new(NoopTextSystem::new()),
        Arc::new(()),
        move || Some(Box::new(SceneRecorder(recorded.clone())) as Box<dyn PlatformHeadlessRenderer>),
    );
    cx.update(crate::init);
    cx.update(|cx| MoonTheme::install_config(config, cx));

    let bounds = Rc::new(Cell::new(Bounds::default()));
    let window: AnyWindowHandle = cx
        .open_window(size(px(480.0), px(240.0)), {
            let bounds = bounds.clone();
            move |_, cx| {
                cx.new(|_| ProbeHost {
                    build: Box::new(move || build().into_any_element()),
                    bounds,
                })
            }
        })
        .expect("headless window must open")
        .into();
    settle(&mut cx, window);

    let control = bounds.get();
    assert!(
        control.size.width > px(0.0) && control.size.height > px(0.0),
        "the probed control was never laid out: {control:?}"
    );
    let middle = control.center();
    if pointer != Pointer::Away {
        send(
            &mut cx,
            window,
            PlatformInput::MouseMove(MouseMoveEvent {
                position: middle,
                pressed_button: None,
                modifiers: Modifiers::default(),
            }),
        );
    }
    if pointer == Pointer::Pressed {
        send(
            &mut cx,
            window,
            PlatformInput::MouseDown(MouseDownEvent {
                button: MouseButton::Left,
                position: middle,
                modifiers: Modifiers::default(),
                click_count: 1,
                first_mouse: false,
            }),
        );
    }

    let scale = cx
        .update_window(window, |_, window, _| window.scale_factor())
        .expect("headless window must report its scale");
    // The recorder only fires on a render pass, and its "image" is an error by design.
    let _ = cx.capture_screenshot(window);
    scene.borrow().painted_for(control, scale)
}

/// Dispatch `event` to the window and let the frames that follow it settle.
fn send(cx: &mut HeadlessAppContext, window: AnyWindowHandle, event: PlatformInput) {
    cx.update_window(window, |_, window, cx| {
        window.dispatch_event(event, cx);
    })
    .expect("headless window must take input");
    settle(cx, window);
}

/// Draw [`SETTLE_FRAMES`] frames.
fn settle(cx: &mut HeadlessAppContext, window: AnyWindowHandle) {
    for _ in 0..SETTLE_FRAMES {
        cx.update_window(window, |_, window, cx| {
            window.refresh();
            window.draw(cx).clear();
        })
        .expect("headless frame must draw");
        cx.run_until_parked();
    }
}

/// The root view of a probe window: one control, start-aligned so the row does not stretch it,
/// and the bounds it was laid out at.
struct ProbeHost {
    build: Box<dyn Fn() -> AnyElement>,
    bounds: Rc<Cell<Bounds<Pixels>>>,
}

impl Render for ProbeHost {
    /// Render the control [`MARGIN`] in from the window's top-left corner and record its bounds.
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let sink = self.bounds.clone();
        div()
            .size_full()
            .flex()
            .flex_row()
            .items_start()
            .justify_start()
            .p(px(MARGIN))
            .on_children_prepainted(move |bounds, _, _| {
                if let Some(control) = bounds.first() {
                    sink.set(*control);
                }
            })
            .child((self.build)())
    }
}

/// The quads and shadows of the last rendered scene.
#[derive(Default)]
struct RecordedScene {
    quads: Vec<Quad>,
    shadows: Vec<Shadow>,
}

impl RecordedScene {
    /// Pick out what was painted for the box laid out at `control`.
    ///
    /// A fill and a border are separate quads over the same box. A child that fills its parent
    /// exactly would be read as the parent; none of the probed controls has one.
    ///
    /// Args:
    ///     control: The control's laid-out bounds.
    ///     scale: The window's scale factor, which the recorded primitives are multiplied by.
    ///
    /// Returns:
    ///     The fill, border colour and shadows painted over exactly that box.
    fn painted_for(&self, control: Bounds<Pixels>, scale: f32) -> Painted {
        let target = control.scale(scale);
        let own_quads = || {
            self.quads
                .iter()
                .filter(|quad| same_box(quad.bounds, target))
        };
        let fill = own_quads()
            .map(|quad| quad.background)
            .find(|background| !background.is_transparent());
        let border = own_quads()
            .find(|quad| {
                let widths = quad.border_widths;
                widths.top.0 + widths.right.0 + widths.bottom.0 + widths.left.0 > 0.0
                    && !quad.border_color.is_transparent()
            })
            .map(|quad| quad.border_color);

        let own_shadows = |inset: bool| {
            self.shadows
                .iter()
                .filter(move |shadow| {
                    (shadow.inset != 0) == inset && same_box(shadow.element_bounds, target)
                })
                .map(move |shadow| box_shadow_of(shadow, scale))
        };
        Painted {
            fill,
            border,
            shadows: own_shadows(true).chain(own_shadows(false)).collect(),
        }
    }
}

/// Return whether two recorded boxes are the same box, to within [`SNAP_TOLERANCE`].
fn same_box(a: Bounds<ScaledPixels>, b: Bounds<ScaledPixels>) -> bool {
    let near = |x: ScaledPixels, y: ScaledPixels| (x.0 - y.0).abs() <= SNAP_TOLERANCE;
    near(a.origin.x, b.origin.x)
        && near(a.origin.y, b.origin.y)
        && near(a.size.width, b.size.width)
        && near(a.size.height, b.size.height)
}

/// Turn a recorded shadow back into the `BoxShadow` it was painted from, in reference pixels.
///
/// A recorded shadow keeps its own rectangle and its element's: the offset is how far the one
/// was moved from the other, and the spread is how far it was grown, or for a shadow inside
/// its element how far its hole was shrunk.
fn box_shadow_of(shadow: &Shadow, scale: f32) -> BoxShadow {
    let grown = (shadow.bounds.size.width.0 - shadow.element_bounds.size.width.0) / 2.0;
    let spread = if shadow.inset != 0 { -grown } else { grown };
    let middle = |bounds: Bounds<ScaledPixels>| bounds.center();
    let moved = middle(shadow.bounds) - middle(shadow.element_bounds);
    BoxShadow {
        color: shadow.color,
        offset: point(px(moved.x.0 / scale), px(moved.y.0 / scale)),
        blur_radius: px(shadow.blur_radius.0 / scale),
        spread_radius: px(spread / scale),
        inset: shadow.inset != 0,
    }
}

/// Records the primitives a headless window submits, without rasterizing them.
struct SceneRecorder(Rc<RefCell<RecordedScene>>);

impl PlatformHeadlessRenderer for SceneRecorder {
    /// Record the scene; image output is intentionally unsupported.
    fn render_scene_to_image(
        &mut self,
        scene: &Scene,
        size: Size<DevicePixels>,
    ) -> anyhow::Result<image::RgbaImage> {
        self.render_scene(scene, size)?;
        anyhow::bail!("the scene recorder does not rasterize images")
    }

    /// Replace the preceding frame, so a probe reads the final submission only.
    fn render_scene(&mut self, scene: &Scene, _size: Size<DevicePixels>) -> anyhow::Result<()> {
        self.0.replace(RecordedScene {
            quads: scene.quads.clone(),
            shadows: scene.shadows.clone(),
        });
        Ok(())
    }

    /// Fills and shadows do not depend on glyph or icon textures.
    fn sprite_atlas(&self) -> Arc<dyn PlatformAtlas> {
        Arc::new(NoSpriteAtlas)
    }
}

/// Omits sprites from the scene: the probe reads quads and shadows only.
struct NoSpriteAtlas;

impl PlatformAtlas for NoSpriteAtlas {
    /// Decline every sprite.
    fn get_or_insert_with<'a>(
        &self,
        _key: &AtlasKey,
        _build: &mut dyn FnMut() -> anyhow::Result<Option<(Size<DevicePixels>, Cow<'a, [u8]>)>>,
    ) -> anyhow::Result<Option<AtlasTile>> {
        Ok(None)
    }

    /// Nothing was stored, so there is nothing to remove.
    fn remove(&self, _key: &AtlasKey) {}
}
