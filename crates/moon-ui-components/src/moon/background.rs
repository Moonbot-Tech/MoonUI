use gpui::{Styled, prelude::FluentBuilder};

use super::tokens::rgba_from;

pub type MoonBackgroundPolicy = crate::BackgroundPolicy;

pub trait MoonBackgroundPolicyExt {
    fn apply<E>(self, element: E, color: u32, alpha: f32) -> E
    where
        E: Styled + FluentBuilder + Sized;
}

impl MoonBackgroundPolicyExt for MoonBackgroundPolicy {
    fn apply<E>(self, element: E, color: u32, alpha: f32) -> E
    where
        E: Styled + FluentBuilder + Sized,
    {
        element.when(self.paints_fill(), |this| {
            this.bg(rgba_from(color, self.fill_alpha(alpha)))
        })
    }
}
