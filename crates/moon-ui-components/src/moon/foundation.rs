use std::rc::Rc;

use gpui::{
    App, BoxShadow, ClickEvent, Corners, DefiniteLength, Div, Edges, FontWeight, Hsla, Pixels,
    Refineable, SharedString, StyleRefinement, Styled, Window, div, point,
};
use serde::{Deserialize, Serialize};

#[inline(always)]
pub fn h_flex() -> Div {
    div().flex().flex_row().items_center()
}

#[inline(always)]
pub fn v_flex() -> Div {
    div().flex().flex_col()
}

#[inline(always)]
pub fn box_shadow(
    x: impl Into<Pixels>,
    y: impl Into<Pixels>,
    blur: impl Into<Pixels>,
    spread: impl Into<Pixels>,
    color: Hsla,
) -> BoxShadow {
    BoxShadow {
        offset: point(x.into(), y.into()),
        blur_radius: blur.into(),
        spread_radius: spread.into(),
        inset: false,
        color,
    }
}

pub trait StyledExt: Styled + Sized {
    fn refine_style(mut self, style: &StyleRefinement) -> Self {
        self.style().refine(style);
        self
    }

    #[inline(always)]
    fn h_flex(self) -> Self {
        self.flex().flex_row().items_center()
    }

    #[inline(always)]
    fn v_flex(self) -> Self {
        self.flex().flex_col()
    }

    fn paddings<L>(self, paddings: impl Into<Edges<L>>) -> Self
    where
        L: Into<DefiniteLength> + Clone + Default + std::fmt::Debug + PartialEq,
    {
        let paddings = paddings.into();
        self.pt(paddings.top.into())
            .pb(paddings.bottom.into())
            .pl(paddings.left.into())
            .pr(paddings.right.into())
    }

    fn margins<L>(self, margins: impl Into<Edges<L>>) -> Self
    where
        L: Into<DefiniteLength> + Clone + Default + std::fmt::Debug + PartialEq,
    {
        let margins = margins.into();
        self.mt(margins.top.into())
            .mb(margins.bottom.into())
            .ml(margins.left.into())
            .mr(margins.right.into())
    }

    fn corner_radii(self, radius: Corners<Pixels>) -> Self {
        self.rounded_tl(radius.top_left)
            .rounded_tr(radius.top_right)
            .rounded_bl(radius.bottom_left)
            .rounded_br(radius.bottom_right)
    }

    fn font_normal(self) -> Self {
        self.font_weight(FontWeight::NORMAL)
    }

    fn font_medium(self) -> Self {
        self.font_weight(FontWeight::MEDIUM)
    }

    fn font_semibold(self) -> Self {
        self.font_weight(FontWeight::SEMIBOLD)
    }

    fn font_bold(self) -> Self {
        self.font_weight(FontWeight::BOLD)
    }
}

impl<E: Styled> StyledExt for E {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoonSize {
    XSmall,
    Small,
    #[default]
    Medium,
    Large,
}

pub type Size = MoonSize;

pub trait Sizable<S = MoonSize>: Sized {
    fn size(self, size: S) -> Self;
}

pub trait Disableable: Sized {
    fn disabled(self, disabled: bool) -> Self;
}

pub type MoonClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
pub type MoonBoolChangeHandler = Rc<dyn Fn(&bool, &mut Window, &mut App)>;
pub type MoonF32ChangeHandler = Rc<dyn Fn(f32, &mut Window, &mut App)>;
pub type MoonIndexedClickHandler = Rc<dyn Fn(usize, &ClickEvent, &mut Window, &mut App)>;
pub type MoonSelectHandler = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

pub fn init(cx: &mut App) {
    crate::init(cx);
    super::theme::MoonTheme::install(cx);
    super::input::bind_moon_input_keys(cx);
    super::select::bind_moon_select_keys(cx);
    super::text_area::bind_moon_text_area_keys(cx);
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeMode {
    Light,
    #[default]
    Dark,
    System,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Theme {
    pub mode: ThemeMode,
}
