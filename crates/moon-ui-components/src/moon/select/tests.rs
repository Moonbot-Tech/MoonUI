//! Scene-order regression for select menus overlapping their hosting popover.

use super::{MoonSelect, MoonSelectItem, MoonSelectState};
use crate::moon::{IndexPath, MoonPopover, MoonTheme, MoonThemeConfig};
use gpui::{
    AppContext as _, AtlasKey, AtlasTile, Context, DevicePixels, Entity, HeadlessAppContext,
    IntoElement, NoopTextSystem, ParentElement as _, PlatformAtlas, PlatformHeadlessRenderer, Quad,
    Render, Scene, Size, Styled as _, Window, div, point, px, size,
};
use std::{borrow::Cow, cell::RefCell, rc::Rc, sync::Arc};

/// Records submitted quads without pretending to rasterize text or pixels.
struct SceneRecorder(Rc<RefCell<Vec<Quad>>>);

impl PlatformHeadlessRenderer for SceneRecorder {
    /// Record the final scene; image output is intentionally unsupported.
    fn render_scene_to_image(
        &mut self,
        scene: &Scene,
        size: Size<DevicePixels>,
    ) -> anyhow::Result<image::RgbaImage> {
        self.render_scene(scene, size)?;
        anyhow::bail!("scene recorder does not rasterize images")
    }

    /// Replace the preceding frame so assertions inspect only the final submission.
    fn render_scene(&mut self, scene: &Scene, _size: Size<DevicePixels>) -> anyhow::Result<()> {
        self.0.replace(scene.quads.clone());
        Ok(())
    }

    /// Quad ordering does not depend on glyph or icon textures.
    fn sprite_atlas(&self) -> Arc<dyn PlatformAtlas> {
        Arc::new(QuadOnlyAtlas)
    }
}

/// Omits sprites from this quad-only scene probe.
struct QuadOnlyAtlas;

impl PlatformAtlas for QuadOnlyAtlas {
    /// No texture is allocated because the recorder only consumes solid quads.
    fn get_or_insert_with<'a>(
        &self,
        _key: &AtlasKey,
        _build: &mut dyn FnMut() -> anyhow::Result<Option<(Size<DevicePixels>, Cow<'a, [u8]>)>>,
    ) -> anyhow::Result<Option<AtlasTile>> {
        Ok(None)
    }

    /// There are no allocated textures to remove.
    fn remove(&self, _key: &AtlasKey) {}
}

/// A tall popover whose surface overlaps the child select menu.
struct NestedSelectHarness {
    state: Entity<MoonSelectState<usize>>,
    opt_in: bool,
}

impl Render for NestedSelectHarness {
    /// Distinct fixed widths identify the actual parent and menu surface quads.
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let mut select = MoonSelect::new(&self.state).menu_width(180.0);
        if self.opt_in {
            select = select.in_popover();
        }
        MoonPopover::new("nested-select")
            .open(true)
            .width(220.0)
            .trigger(div().w(px(20.0)).h(px(20.0)))
            .content(
                div()
                    .w(px(200.0))
                    .h(px(220.0))
                    .child(div().w(px(180.0)).h(px(32.0)).child(select)),
            )
    }
}

/// Removing the Moon in_popover forwarding paints the overlapping menu below its parent.
/// Raising every select instead breaks the false case, which preserves ordinary paint order.
#[test]
fn popover_menu_opt_in_controls_overlapping_quad_order() {
    for (theme, config) in [
        ("dark", MoonThemeConfig::moon_terminal()),
        ("light", MoonThemeConfig::moon_light()),
    ] {
        for opt_in in [false, true] {
            let quads = Rc::new(RefCell::new(Vec::new()));
            let recorded = quads.clone();
            let mut cx = HeadlessAppContext::with_platform(
                Arc::new(NoopTextSystem::new()),
                Arc::new(()),
                move || Some(Box::new(SceneRecorder(recorded.clone()))),
            );
            cx.update(crate::init);
            cx.update(|cx| MoonTheme::install_config(config.clone(), cx));
            let window = cx
                .open_window(size(px(800.0), px(600.0)), |window, cx| {
                    let state = cx.new(|cx| {
                        MoonSelectState::new(
                            [
                                MoonSelectItem::new(0, "First"),
                                MoonSelectItem::new(1, "Second"),
                            ],
                            Some(IndexPath::new(1)),
                            window,
                            cx,
                        )
                    });
                    state.update(cx, |state, _| state.set_open(true));
                    cx.new(|_| NestedSelectHarness { state, opt_in })
                })
                .expect("headless window must open");
            for _ in 0..8 {
                cx.update_window(window.into(), |_, window, cx| {
                    window.refresh();
                    window.draw(cx).clear();
                })
                .expect("headless frame must draw");
                cx.run_until_parked();
            }
            let scale = cx
                .update_window(window.into(), |_, window, _| window.scale_factor())
                .expect("headless window must retain its scale");
            // The recorder captures submission and explicitly refuses pixel output.
            let error = cx
                .capture_screenshot(window.into())
                .expect_err("quad recorder has no pixels");
            assert_eq!(
                error.to_string(),
                "scene recorder does not rasterize images"
            );
            let quads = quads.borrow();
            let parents: Vec<_> = quads
                .iter()
                .filter(|quad| {
                    !quad.background.is_transparent()
                        && quad.bounds.size.width.0 == 220.0 * scale
                        && quad.bounds.size.height.0 > 220.0 * scale
                })
                .collect();
            let menus: Vec<_> = quads
                .iter()
                .filter(|quad| {
                    !quad.background.is_transparent()
                        && quad.bounds.size.width.0 == 180.0 * scale
                        && quad.bounds.size.height.0 > 40.0 * scale
                })
                .collect();
            assert_eq!(
                parents.len(),
                1,
                "{theme}/{opt_in}: unique parent surface: {quads:?}"
            );
            assert_eq!(
                menus.len(),
                1,
                "{theme}/{opt_in}: unique menu surface: {quads:?}"
            );
            let parent = parents[0];
            let menu = menus[0];
            let target = point(
                menu.bounds.center().x,
                menu.bounds.top() + gpui::ScaledPixels(20.0 * scale),
            );
            assert!(parent.bounds.contains(&target), "probe must overlap parent");
            assert!(menu.bounds.contains(&target), "probe must overlap menu");
            assert!(
                parent.content_mask.bounds.contains(&target),
                "parent must not be clipped at probe"
            );
            assert!(
                menu.content_mask.bounds.contains(&target),
                "menu must not be clipped at probe"
            );
            assert_ne!(
                menu.order, parent.order,
                "overlapping surfaces must have distinct draw orders"
            );
            assert_eq!(
                menu.order > parent.order,
                opt_in,
                "{theme}/{opt_in}: overlapping menu must paint above parent exactly when opted in; parent={}, menu={}",
                parent.order,
                menu.order
            );
            eprintln!(
                "{theme}/{opt_in}: parent={}, menu={}",
                parent.order, menu.order
            );
        }
    }
}
