use gpui::prelude::FluentBuilder;
use gpui::*;

use super::{foundation::box_shadow, text::MoonText, theme::MoonTheme, tokens::rgba_from};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoonAvatarSize {
    Compact,
    Normal,
    Large,
    Custom(f32),
}

impl MoonAvatarSize {
    fn px(self) -> f32 {
        match self {
            MoonAvatarSize::Compact => 20.0,
            MoonAvatarSize::Normal => 28.0,
            MoonAvatarSize::Large => 36.0,
            MoonAvatarSize::Custom(size) => size,
        }
    }
}

#[derive(IntoElement)]
pub struct MoonAvatar {
    src: Option<ImageSource>,
    name: Option<SharedString>,
    initials: Option<SharedString>,
    size: MoonAvatarSize,
    tone: Option<u32>,
}

impl MoonAvatar {
    pub fn new() -> Self {
        Self {
            src: None,
            name: None,
            initials: None,
            size: MoonAvatarSize::Normal,
            tone: None,
        }
    }

    pub fn src(mut self, source: impl Into<ImageSource>) -> Self {
        self.src = Some(source.into());
        self
    }

    pub fn name(mut self, name: impl Into<SharedString>) -> Self {
        let name = name.into();
        self.initials = Some(initials(&name).into());
        self.name = Some(name);
        self
    }

    pub fn initials(mut self, initials: impl Into<SharedString>) -> Self {
        self.initials = Some(initials.into());
        self
    }

    pub fn size(mut self, size: MoonAvatarSize) -> Self {
        self.size = size;
        self
    }

    pub fn tone(mut self, color: u32) -> Self {
        self.tone = Some(color);
        self
    }

    pub fn compact(self) -> Self {
        self.size(MoonAvatarSize::Compact)
    }

    pub fn large(self) -> Self {
        self.size(MoonAvatarSize::Large)
    }

    pub fn render(self) -> impl IntoElement {
        self
    }
}

impl Default for MoonAvatar {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for MoonAvatar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let size = tokens.ui(self.size.px());
        let initials = self.initials.unwrap_or_else(|| SharedString::from("--"));
        let palette = [p.blue, p.green, p.amber, p.orange, p.red];
        let color = self.tone.unwrap_or_else(|| {
            let ix = gpui::hash(&initials) as usize % palette.len();
            palette[ix]
        });
        let text = tokens.text(size * 0.38, size * 0.46);

        let root = div()
            .relative()
            .w(px(size))
            .h(px(size))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .overflow_hidden()
            .rounded(px(size * 0.5))
            .border(px(tokens.ui(1.0)))
            .border_color(rgba_from(color, 0.45))
            .bg(rgba_from(color, 0.14))
            .shadow(vec![box_shadow(
                px(0.0),
                px(tokens.ui(1.0)),
                px(tokens.ui(4.0)),
                px(0.0),
                rgba_from(p.shadow, 0.30),
            )]);

        if let Some(src) = self.src {
            root.child(img(src).w(px(size)).h(px(size)).rounded(px(size * 0.5)))
        } else {
            root.child(
                MoonText::new(initials)
                    .color(color)
                    .font_size(text.font_size)
                    .line_height(text.line_height)
                    .weight(700.0)
                    .mono(true)
                    .uppercase(true)
                    .render(),
            )
        }
    }
}

#[derive(IntoElement)]
pub struct MoonAvatarGroup {
    avatars: Vec<MoonAvatar>,
    size: MoonAvatarSize,
    limit: usize,
    ellipsis: bool,
}

impl MoonAvatarGroup {
    pub fn new() -> Self {
        Self {
            avatars: Vec::new(),
            size: MoonAvatarSize::Normal,
            limit: 4,
            ellipsis: false,
        }
    }

    pub fn child(mut self, avatar: MoonAvatar) -> Self {
        self.avatars.push(avatar);
        self
    }

    pub fn children(mut self, avatars: impl IntoIterator<Item = MoonAvatar>) -> Self {
        self.avatars.extend(avatars);
        self
    }

    pub fn size(mut self, size: MoonAvatarSize) -> Self {
        self.size = size;
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit.max(1);
        self
    }

    pub fn ellipsis(mut self, ellipsis: bool) -> Self {
        self.ellipsis = ellipsis;
        self
    }

    pub fn render(self) -> impl IntoElement {
        self
    }
}

impl Default for MoonAvatarGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for MoonAvatarGroup {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let overlap = -tokens.ui(self.size.px() * 0.28);
        let count = self.avatars.len();
        let mut row = div().flex().flex_row_reverse().items_center();

        if self.ellipsis && count > self.limit {
            row = row.child(
                MoonAvatar::new()
                    .initials(format!("+{}", count - self.limit))
                    .size(self.size)
                    .tone(MoonTheme::active_tokens(cx).palette.text_muted),
            );
        }

        for (ix, avatar) in self.avatars.into_iter().take(self.limit).enumerate().rev() {
            row = row.child(
                div()
                    .when(ix > 0, |this| this.ml(px(overlap)))
                    .child(avatar.size(self.size).render()),
            );
        }

        row
    }
}

fn initials(text: &str) -> String {
    let mut result = text
        .split_whitespace()
        .flat_map(|word| word.chars().next().map(|ch| ch.to_string()))
        .take(2)
        .collect::<Vec<String>>()
        .join("");
    if result.len() == 1 {
        result = text.chars().take(2).collect();
    }
    result.to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::initials;

    #[test]
    fn avatar_initials_match_common_names() {
        assert_eq!(initials("Jason Lee"), "JL");
        assert_eq!(initials("huacnlee"), "HU");
    }
}
