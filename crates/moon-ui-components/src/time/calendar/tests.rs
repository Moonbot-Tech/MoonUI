use chrono::NaiveDate;

use super::Date;

#[test]
fn test_date_to_string() {
    let date = Date::Single(Some(NaiveDate::from_ymd_opt(2024, 8, 3).unwrap()));
    assert_eq!(date.to_string(), "2024-08-03");

    let date = Date::Single(None);
    assert_eq!(date.to_string(), "nil");

    let date = Date::Range(
        Some(NaiveDate::from_ymd_opt(2024, 8, 3).unwrap()),
        Some(NaiveDate::from_ymd_opt(2024, 8, 5).unwrap()),
    );
    assert_eq!(date.to_string(), "2024-08-03 - 2024-08-05");

    let date = Date::Range(Some(NaiveDate::from_ymd_opt(2024, 8, 3).unwrap()), None);
    assert_eq!(date.to_string(), "2024-08-03 - nil");

    let date = Date::Range(None, Some(NaiveDate::from_ymd_opt(2024, 8, 5).unwrap()));
    assert_eq!(date.to_string(), "nil - 2024-08-05");

    let date = Date::Range(None, None);
    assert_eq!(date.to_string(), "nil");
}

use super::{Calendar, CalendarState};
use crate::moon::{MoonTheme, ThemeMode};
use gpui::{AppContext as _, Context, Entity, IntoElement, Render, VisualTestContext, Window};

/// Render two real month columns through the public Calendar builder.
struct MonthGridHarness(Entity<CalendarState>);
impl Render for MonthGridHarness {
    /// Keep adjacent five/six-week grids mounted together to detect boundary spill.
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Calendar::new(&self.0).number_of_months(2)
    }
}

/// Five fixed weeks lose August 30/31; flattening then chunking at five mixes September's month
/// offset. Clicking the rendered sixth-week date must emit/select the exact requested date.
#[gpui::test]
fn sixth_week_dates_remain_selectable_beside_next_month(cx: &mut gpui::TestAppContext) {
    cx.update(crate::init);
    for theme in [ThemeMode::Dark, ThemeMode::Light] {
        cx.update(|cx| MoonTheme::set_mode(theme, cx));
        let window = cx.add_window(|window, cx| {
            let state = cx.new(|cx| CalendarState::new(window, cx));
            state.update(cx, |state, cx| {
                state.set_date(NaiveDate::from_ymd_opt(2026, 8, 1).unwrap(), window, cx)
            });
            MonthGridHarness(state)
        });
        let state = window.read_with(cx, |view, _| view.0.clone()).unwrap();
        let mut visual = VisualTestContext::from_window(window.into(), cx);
        visual.run_until_parked();
        for (date, selector) in [
            ("2026-08-30", "calendar-day-2026-08-30_0"),
            ("2026-08-31", "calendar-day-2026-08-31_0"),
        ] {
            let bounds = visual
                .debug_bounds(selector)
                .expect("sixth-week date must render");
            visual.simulate_click(bounds.center(), gpui::Modifiers::default());
            visual.run_until_parked();
            visual.update(|_, cx| {
                assert_eq!(
                    state.read(cx).date(),
                    Date::Single(Some(date.parse().unwrap()))
                )
            });
        }
        let august = visual.debug_bounds("calendar-day-2026-08-01_0").unwrap();
        let september = visual
            .debug_bounds("calendar-day-2026-09-01_1")
            .expect("independent next month");
        assert!(
            september.left() > august.right(),
            "month columns must remain separate"
        );
    }
}
