//! Regression coverage for stroke-width rendering of Moon SVG icons.

use super::{stroke_atlas_key, stroke_ratio, with_rendered_stroke};
use gpui::px;

/// Catches `svg.rs:with_rendered_stroke` rewriting only the first stroke or scaling against a fixed
/// 24-unit grid: a multi-path icon drawn from a 16-unit `viewBox` would render its strokes at
/// mixed or wrong weights. Both strokes must render 1px wide at 8px (2 units of 16), and the shapes
/// must stay untouched.
#[test]
fn rewrites_every_stroke_against_the_icons_own_view_box() {
    let icon = concat!(
        r#"<svg viewBox="0 0 16 16" fill="none">"#,
        r#"<path d="M2 8H14" stroke="currentColor" stroke-width="1.5"/>"#,
        r#"<circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1"/>"#,
        "</svg>",
    );

    let rewritten = with_rendered_stroke(icon, 1.0, 8.0).expect("icon has a viewBox");

    assert_eq!(rewritten.matches(r#"stroke-width="2.0000""#).count(), 2);
    assert_eq!(rewritten.matches("stroke-width=").count(), 2);
    assert!(rewritten.contains(r#"d="M2 8H14""#));
    assert!(rewritten.contains(r#"cx="8" cy="8" r="6""#));
}

/// Catches `svg.rs:with_rendered_stroke` guessing a scale for an icon without a `viewBox`: it must
/// refuse the rewrite so the painter can fall back to the authored stroke.
#[test]
fn refuses_an_icon_without_a_view_box() {
    let icon = r#"<svg width="24" height="24"><path d="M5 12H19" stroke-width="2"/></svg>"#;

    assert_eq!(with_rendered_stroke(icon, 2.0, 14.0), None);
}

/// Catches `svg.rs:stroke_atlas_key` or `stroke_ratio` naming stroke variants wrongly. Two icons
/// from one file at one size with different strokes must get different sprite keys, or the second
/// draws with the first one's stroke; the same stroke under 1.5x UI zoom must keep its key, or
/// every zoom step re-rewrites and re-rasterizes the icon; and a zero width must not be drawn.
#[test]
fn stroke_variants_are_named_by_their_stroke_to_size_ratio() {
    let path = "icons/moon-checkbox-check.svg";
    let thin = stroke_ratio(px(1.67), px(12.)).expect("drawable");
    let bold = stroke_ratio(px(2.), px(12.)).expect("drawable");
    let thin_zoomed = stroke_ratio(px(1.67 * 1.5), px(12. * 1.5)).expect("drawable");

    assert_ne!(stroke_atlas_key(path, thin), stroke_atlas_key(path, bold));
    assert_eq!(
        stroke_atlas_key(path, thin),
        stroke_atlas_key(path, thin_zoomed)
    );
    assert_eq!(stroke_ratio(px(2.), px(0.)), None);
}

/// Catches whitespace-only parsing, which rejects valid comma-separated viewBox coordinates
/// and prevents applying the requested stroke weight to those icons.
#[test]
fn accepts_comma_and_mixed_view_box_separators() {
    for view_box in ["0,0,16,16", "0, 0  16,16", "0 0 16 16"] {
        let icon = format!(r#"<svg viewBox="{view_box}"><path stroke-width="1"/></svg>"#);
        let expected = format!(r#"<svg viewBox="{view_box}"><path stroke-width="2.0000"/></svg>"#);
        assert_eq!(with_rendered_stroke(&icon, 1., 8.), Some(expected));
    }
}

/// One valid authored icon without a viewBox; rasterization must load it after rewrite refusal.
struct AuthoredIcon(std::sync::Arc<std::sync::atomic::AtomicUsize>);

impl gpui::AssetSource for AuthoredIcon {
    /// Counts reads of the fixture to distinguish rewrite loading from fallback rasterization.
    fn load(&self, path: &str) -> anyhow::Result<Option<std::borrow::Cow<'static, [u8]>>> {
        assert_eq!(path, "authored.svg");
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(Some(std::borrow::Cow::Borrowed(
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><path d="M2 8H14" stroke="black" stroke-width="2"/></svg>"#,
        )))
    }

    /// This fixture exposes only its one authored icon.
    fn list(&self, _: &str) -> anyhow::Result<Vec<gpui::SharedString>> {
        Ok(vec!["authored.svg".into()])
    }
}

/// Draws an icon whose authored SVG is valid but whose stroke cannot be rewritten.
struct FallbackIcon;

impl gpui::Render for FallbackIcon {
    /// Returns the requested-stroke icon, tinted explicitly so paint reaches asset loading.
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        use gpui::Styled as _;
        super::moon_svg("authored.svg")
            .stroke_width(px(1.))
            .size(px(16.))
            .text_color(gpui::black())
    }
}

/// Catches returning from MoonSvg::paint on rewrite refusal: the authored asset must reach
/// GPUI's rasterizer, requiring a second load after the failed rewrite instead of disappearing.
#[test]
fn rewrite_failure_paints_the_authored_svg() {
    use gpui::{AppContext as _, HeadlessAppContext, NoopTextSystem, size};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let loads = Arc::new(AtomicUsize::new(0));
    let mut cx = HeadlessAppContext::with_asset_source(
        Arc::new(NoopTextSystem::new()),
        Arc::new(AuthoredIcon(loads.clone())),
    );
    // Opening the headless window paints its initial frame.
    let _window = cx
        .open_window(size(px(32.), px(32.)), |_, cx| cx.new(|_| FallbackIcon))
        .expect("window");
    assert_eq!(
        loads.load(Ordering::SeqCst),
        2,
        "rewrite refusal must still load the authored SVG for painting"
    );
}
