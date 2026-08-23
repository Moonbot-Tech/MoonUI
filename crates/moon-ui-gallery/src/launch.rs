//! Gallery application startup and window selection.

use super::*;

/// Initializes MoonUI and opens either the main gallery or handoff-case window.
pub(super) fn run_gallery() {
    let args = gallery_args_from_cli();
    let initial_page = args.page;
    let snapshot_dir = args.snapshot_dir;
    let case_snapshot_dir = args.case_snapshot_dir;
    let snapshot_case_ids = args.snapshot_case_ids;
    let theme_mode = args.theme_mode;
    application()
        .with_assets(moon_ui::MoonAssets)
        .run(move |cx: &mut App| {
            moon_ui::foundation::init(cx);
            let mut theme_config = MoonThemeConfig::moon_terminal();
            theme_config.mode = theme_mode;
            MoonTheme::install_config(theme_config, cx);

            let p = MoonPalette::active(cx);
            if let Some(case_dir) = case_snapshot_dir.clone() {
                let first_case = handoff::first_handoff_case_for_ids(&snapshot_case_ids);
                let bounds =
                    Bounds::centered(None, size(px(first_case.width), px(first_case.height)), cx);
                cx.open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        titlebar: Some(TitlebarOptions {
                            title: Some(SharedString::from("MoonUI Handoff Cases")),
                            appears_transparent: true,
                            traffic_light_position: None,
                        }),
                        window_clear_color: Some(rgba((p.shell << 8) | 0xFF)),
                        app_id: Some("pro.moonbot.moon-ui-handoff-cases".to_string()),
                        ..Default::default()
                    },
                    move |window, cx| {
                        let view = cx.new(|cx| {
                            handoff::CaseGallery::new(
                                window,
                                cx,
                                Some(case_dir.clone()),
                                snapshot_case_ids.clone(),
                                theme_mode,
                            )
                        });
                        cx.new(|cx| {
                            Root::new(view, window, cx)
                                .bordered(false)
                                .background_policy(MoonBackgroundPolicy::Opaque)
                                .background(MoonPalette::active(cx).shell)
                        })
                    },
                )
                .expect("open MoonUI handoff case window");
            } else {
                let bounds = Bounds::centered(None, size(px(1280.0), px(900.0)), cx);
                cx.open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        titlebar: Some(TitlebarOptions {
                            title: Some(SharedString::from("MoonUI Gallery")),
                            appears_transparent: true,
                            traffic_light_position: None,
                        }),
                        window_clear_color: Some(rgba((p.shell << 8) | 0xFF)),
                        app_id: Some("pro.moonbot.moon-ui-gallery".to_string()),
                        ..Default::default()
                    },
                    move |window, cx| {
                        let view = cx.new(|cx| {
                            gallery::Gallery::new(
                                window,
                                cx,
                                initial_page,
                                snapshot_dir.clone(),
                                theme_mode,
                            )
                        });
                        cx.new(|cx| {
                            Root::new(view, window, cx)
                                .background_policy(MoonBackgroundPolicy::Opaque)
                                .background(MoonPalette::active(cx).shell)
                        })
                    },
                )
                .expect("open MoonUI gallery window");
            }
            cx.activate(true);
        });
}
