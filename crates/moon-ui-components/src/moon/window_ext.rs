use gpui::{App, SharedString, Window};

use super::{MoonDialog, MoonNotification, MoonPlacement, MoonSheet};

pub trait MoonWindowExt {
    fn open_moon_sheet<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(MoonSheet, &mut Window, &mut App) -> MoonSheet + 'static;

    fn open_moon_sheet_at<F>(&mut self, placement: MoonPlacement, cx: &mut App, build: F)
    where
        F: Fn(MoonSheet, &mut Window, &mut App) -> MoonSheet + 'static;

    fn close_moon_sheet(&mut self, cx: &mut App);

    fn open_moon_dialog<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(MoonDialog, &mut Window, &mut App) -> MoonDialog + 'static;

    fn open_unique_moon_dialog<F>(&mut self, id: impl Into<SharedString>, cx: &mut App, build: F)
    where
        F: Fn(MoonDialog, &mut Window, &mut App) -> MoonDialog + 'static;

    fn close_dialog(&mut self, cx: &mut App);

    fn close_context_menu(&mut self, cx: &mut App);

    fn push_notification(&mut self, note: MoonNotification, cx: &mut App);
}

impl MoonWindowExt for Window {
    fn open_moon_sheet<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(MoonSheet, &mut Window, &mut App) -> MoonSheet + 'static,
    {
        <Window as crate::WindowExt>::open_sheet(self, cx, build);
    }

    fn open_moon_sheet_at<F>(&mut self, placement: MoonPlacement, cx: &mut App, build: F)
    where
        F: Fn(MoonSheet, &mut Window, &mut App) -> MoonSheet + 'static,
    {
        <Window as crate::WindowExt>::open_sheet_at(self, placement, cx, build);
    }

    fn close_moon_sheet(&mut self, cx: &mut App) {
        <Window as crate::WindowExt>::close_sheet(self, cx);
    }

    fn open_moon_dialog<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(MoonDialog, &mut Window, &mut App) -> MoonDialog + 'static,
    {
        <Window as crate::WindowExt>::open_dialog(self, cx, move |dialog, window, cx| {
            build(MoonDialog::from_inner(dialog), window, cx).into_inner()
        });
    }

    fn open_unique_moon_dialog<F>(&mut self, id: impl Into<SharedString>, cx: &mut App, build: F)
    where
        F: Fn(MoonDialog, &mut Window, &mut App) -> MoonDialog + 'static,
    {
        <Window as crate::WindowExt>::open_unique_dialog(
            self,
            id,
            cx,
            move |dialog, window, cx| {
                build(MoonDialog::from_inner(dialog), window, cx).into_inner()
            },
        );
    }

    fn close_dialog(&mut self, cx: &mut App) {
        <Window as crate::WindowExt>::close_dialog(self, cx);
    }

    fn close_context_menu(&mut self, cx: &mut App) {
        <Window as crate::WindowExt>::close_context_menu(self, cx);
    }

    fn push_notification(&mut self, note: MoonNotification, cx: &mut App) {
        <Window as crate::WindowExt>::push_notification(self, note, cx);
    }
}
