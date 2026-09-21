//! Regression coverage for MoonPopover width ownership.

use super::{MoonPopover, MoonPopoverWidth, POPOVER_BORDER, POPOVER_PADDING};
use crate::moon::{MoonPalette, MoonScale, MoonTheme, MoonThemeTokens};
use gpui::{
    Context, InteractiveElement as _, IntoElement, Render, Styled as _, VisualTestContext, Window,
    div, px,
};

/// `popover.rs:MoonPopoverWidth::resolve` must apply UI and font scaling independently while
/// reserving the popup's own padding and border. Replacing either content policy with a raw outer
/// width clips fixed content as soon as the corresponding scale differs from 1.0.
#[test]
fn content_width_policies_reserve_scaled_popup_chrome() {
    for palette in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
        for (ui, font, font_delta) in [(0.5, 1.75, 0.0), (2.5, 0.75, 4.0)] {
            let mut tokens = MoonThemeTokens {
                palette,
                ..MoonThemeTokens::default()
            };
            tokens.scale = MoonScale {
                ui,
                font,
                font_delta,
                tier: Default::default(),
                zoom: 1.0,
            };
            let chrome = tokens.ui(POPOVER_PADDING) * 2.0 + POPOVER_BORDER * 2.0;

            assert_eq!(
                MoonPopoverWidth::UiContent(240.0).resolve(&tokens),
                Some(tokens.ui(240.0) + chrome)
            );
            assert_eq!(
                MoonPopoverWidth::FontContent(240.0).resolve(&tokens),
                Some(tokens.font_width(240.0) + chrome)
            );
            assert_eq!(MoonPopoverWidth::Intrinsic.resolve(&tokens), None);
        }
    }
}

/// Root view containing an always-open intrinsic popover with a fixed-width child.
struct IntrinsicPopoverHarness;

impl Render for IntrinsicPopoverHarness {
    /// Render the geometry probe.
    ///
    /// Args:
    ///     _window: Test window.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     The open intrinsic popover.
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        MoonPopover::new("intrinsic-geometry")
            .default_open(true)
            .fit_content()
            .trigger(div().w(px(20.0)).h(px(20.0)))
            .content(
                div()
                    .debug_selector(|| "intrinsic-geometry:child".to_string())
                    .w(px(73.0))
                    .h(px(20.0)),
            )
    }
}

/// `popover.rs:MoonPopover::render` must leave intrinsic width unset so the rendered border box
/// shrink-wraps its child plus component-owned chrome. Restoring an unconditional default width or
/// omitting scaled padding reddens the corresponding bounds assertion.
#[gpui::test]
fn intrinsic_popover_shrink_wraps_its_rendered_child(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    let scale = MoonScale {
        ui: 2.5,
        font: 0.25,
        font_delta: 0.0,
        tier: Default::default(),
        zoom: 1.0,
    };
    cx.update(|cx| {
        MoonTheme::global_mut(cx).scale = scale;
    });
    let window = cx.add_window(|_, _| IntrinsicPopoverHarness);
    let mut cx = VisualTestContext::from_window(window.into(), cx);

    let popup = (0..8)
        .find_map(|_| {
            cx.update(|window, _| window.refresh());
            cx.run_until_parked();
            cx.debug_bounds("intrinsic-geometry:popup")
        })
        .expect("open intrinsic popover must render");
    let child = cx
        .debug_bounds("intrinsic-geometry:child")
        .expect("intrinsic popover child must render");
    let mut tokens = MoonThemeTokens::default();
    tokens.scale = scale;
    let chrome = tokens.ui(POPOVER_PADDING) * 2.0 + POPOVER_BORDER * 2.0;

    assert_eq!(child.size.width, px(73.0));
    assert_eq!(popup.size.width, child.size.width + px(chrome));
}

// ---- Scene-order proof: managed tooltip must outrank its hosting Moon popover ----

use crate::ElementExt as _;
use crate::Root;
use crate::moon::{MoonButton, MoonThemeConfig};
use crate::tooltip::{TooltipContent, TooltipOverlay};
use gpui::{
    AnyWindowHandle, AppContext as _, AtlasKey, AtlasTile, Bounds, DevicePixels,
    HeadlessAppContext, NoopTextSystem, ParentElement as _, Pixels, PlatformAtlas,
    PlatformHeadlessRenderer, Quad, Scene, Size, size,
};
use std::{
    borrow::Cow,
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
    time::Duration,
};

/// Fresh copy of moon/select/tests.rs's module-private recorder pair: exporting it would mean
/// rewriting a test file that already carries a passing test, out of scope for a proof dispatch.
struct HostedTooltipSceneRecorder(Rc<RefCell<Vec<Quad>>>);
impl PlatformHeadlessRenderer for HostedTooltipSceneRecorder {
    fn render_scene_to_image(
        &mut self,
        scene: &Scene,
        size: Size<DevicePixels>,
    ) -> anyhow::Result<image::RgbaImage> {
        self.render_scene(scene, size)?;
        anyhow::bail!("scene recorder does not rasterize images")
    }
    fn render_scene(&mut self, scene: &Scene, _size: Size<DevicePixels>) -> anyhow::Result<()> {
        self.0.replace(scene.quads.clone());
        Ok(())
    }
    fn sprite_atlas(&self) -> Arc<dyn PlatformAtlas> {
        Arc::new(HostedTooltipQuadOnlyAtlas)
    }
}

struct HostedTooltipQuadOnlyAtlas;
impl PlatformAtlas for HostedTooltipQuadOnlyAtlas {
    fn get_or_insert_with<'a>(
        &self,
        _key: &AtlasKey,
        _build: &mut dyn FnMut() -> anyhow::Result<Option<(Size<DevicePixels>, Cow<'a, [u8]>)>>,
    ) -> anyhow::Result<Option<AtlasTile>> {
        Ok(None)
    }
    fn remove(&self, _key: &AtlasKey) {}
}

const HOSTED_TOOLTIP_PROBE_W: f32 = 96.0;
const HOSTED_TOOLTIP_PROBE_H: f32 = 28.0;

/// Fixed-size tooltip body so its scene quad cannot be mistaken for the popover chrome.
struct HostedTooltipProbeBody;
impl Render for HostedTooltipProbeBody {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(HOSTED_TOOLTIP_PROBE_W))
            .h(px(HOSTED_TOOLTIP_PROBE_H))
            .bg(gpui::black())
    }
}

/// Always-open Moon popover hosting a button positioned well inside the popover box on every side.
struct HostedTooltipHarness {
    button_bounds: Rc<Cell<Bounds<Pixels>>>,
}
impl Render for HostedTooltipHarness {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let button_bounds = self.button_bounds.clone();
        MoonPopover::new("hosted-tooltip")
            .open(true)
            .width(400.0)
            .trigger(div().w(px(20.0)).h(px(20.0)))
            .content(
                div().relative().w(px(360.0)).h(px(240.0)).child(
                    div()
                        .id("hosted-tooltip:button")
                        .absolute()
                        .left(px(140.0))
                        .top(px(100.0))
                        .w(px(80.0))
                        .h(px(32.0))
                        .on_prepaint(move |bounds, _, _| button_bounds.set(bounds))
                        .child(MoonButton::new("hosted-button").tooltip("Hosted tooltip")),
                ),
            )
    }
}

/// Pumps N draw frames so prepaint capture and animation state settle deterministically.
fn hosted_tooltip_pump(cx: &mut HeadlessAppContext, window: AnyWindowHandle, frames: usize) {
    for _ in 0..frames {
        cx.update_window(window, |_, window, cx| {
            window.refresh();
            window.draw(cx).clear();
        })
        .expect("headless frame must draw");
        cx.run_until_parked();
    }
}

/// tooltip.rs TooltipOverlay::render .with_priority(LAYER_TOOLTIP) must keep the managed tooltip
/// above its hosting Moon popover (LAYER_TOOLTIP == 100_000 vs LAYER_MOON_POPOVER == 30_000).
/// Lowering it back to a literal, or pointing it at LAYER_OVERLAY, reintroduces MoonUI #628.
/// layer::tests pins the constants RELATION; this pins that the overlay actually USES it.
#[test]
fn popover_hosted_tooltip_paints_above_its_popover() {
    for (theme, config) in [
        ("dark", MoonThemeConfig::moon_terminal()),
        ("light", MoonThemeConfig::moon_light()),
    ] {
        let quads = Rc::new(RefCell::new(Vec::new()));
        let recorded = quads.clone();
        let mut cx = HeadlessAppContext::with_platform(
            Arc::new(NoopTextSystem::new()),
            Arc::new(()),
            move || Some(Box::new(HostedTooltipSceneRecorder(recorded.clone()))),
        );
        cx.update(crate::init);
        cx.update(|cx| MoonTheme::install_config(config.clone(), cx));

        let button_bounds = Rc::new(Cell::new(Bounds::default()));
        let window = cx
            .open_window(size(px(800.0), px(600.0)), {
                let button_bounds = button_bounds.clone();
                move |window, cx| {
                    let view = cx.new(|_| HostedTooltipHarness { button_bounds });
                    cx.new(|cx| Root::new(view, window, cx))
                }
            })
            .expect("headless window must open");

        hosted_tooltip_pump(&mut cx, window.into(), 4);
        let trigger_bounds = button_bounds.get();

        cx.update_window(window.into(), |_, window, cx| {
            let overlay =
                Root::tooltip_overlay(window, cx).expect("Root must own a tooltip overlay");
            overlay.update(cx, |overlay: &mut TooltipOverlay, cx| {
                let content = TooltipContent {
                    build: Rc::new(|_, cx| cx.new(|_| HostedTooltipProbeBody).into()),
                    trigger_bounds,
                };
                overlay.request_show(content, window, cx);
            });
        })
        .expect("headless window must accept the direct request_show drive");

        // Cold-start timer (tooltip.rs:409-434) never fires under run_until_parked alone
        // (dispatcher.rs:76-78); advance the simulated clock by hand.
        cx.advance_clock(Duration::from_millis(500));
        let scale = cx
            .update_window(window.into(), |_, window, _| window.scale_factor())
            .expect("headless window must retain its scale");
        hosted_tooltip_pump(&mut cx, window.into(), 8);
        // The recorder only fires on an actual render pass; a plain draw() never reaches it.
        let _ = cx.capture_screenshot(window.into());

        let quads = quads.borrow();
        let popovers: Vec<_> = quads
            .iter()
            .filter(|q| !q.background.is_transparent() && q.bounds.size.width.0 == 400.0 * scale)
            .collect();
        let tooltips: Vec<_> = quads
            .iter()
            .filter(|q| {
                !q.background.is_transparent()
                    && q.bounds.size.width.0 == HOSTED_TOOLTIP_PROBE_W * scale
                    && q.bounds.size.height.0 == HOSTED_TOOLTIP_PROBE_H * scale
            })
            .collect();

        assert_eq!(
            popovers.len(),
            1,
            "{theme}: unique hosting popover: {quads:?}"
        );
        // Must exist BEFORE any order comparison -- a "not found" path that skips the compare
        // could never redden under the named mutation.
        assert_eq!(
            tooltips.len(),
            1,
            "{theme}: tooltip quad must exist: {quads:?}"
        );
        let popover = popovers[0];
        let tooltip = tooltips[0];
        let target = tooltip.bounds.center();

        assert!(
            popover.bounds.contains(&target),
            "{theme}: tooltip must render inside its popover"
        );
        assert!(
            popover.content_mask.bounds.contains(&target),
            "{theme}: popover must not be clipped at the probe point"
        );
        assert!(
            tooltip.content_mask.bounds.contains(&target),
            "{theme}: tooltip must not be clipped at the probe point"
        );
        assert_ne!(
            tooltip.order, popover.order,
            "{theme}: overlapping surfaces must have distinct draw orders"
        );
        assert!(
            tooltip.order > popover.order,
            "{theme}: tooltip must paint above its hosting popover; popover={}, tooltip={}",
            popover.order,
            tooltip.order
        );
    }
}
