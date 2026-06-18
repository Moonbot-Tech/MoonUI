use std::rc::Rc;

use gpui::{
    AnyElement, App, DefiniteLength, Edges, EdgesRefinement, Entity, Hsla, InteractiveElement as _,
    IntoElement, MouseButton, ParentElement as _, Pixels, RenderOnce, StyleRefinement, Styled,
    TextAlign, Window, div, px, relative,
};
use gpui::prelude::FluentBuilder as _;

use crate::button::{Button, ButtonVariants as _};
use crate::input::clear_button;
use crate::moon_skin::{MoonSkinPalette, moon_color};
use crate::native_menu::NativeMenu;
use crate::spinner::Spinner;
use crate::{ActiveTheme, v_flex};
use crate::{IconName, Size};
use crate::{Selectable, StyledExt, h_flex};
use crate::Sizable;

use super::{InputState, element::EditorScrollbar};

/// Returns `(background, foreground)` colors for input-like components.
pub(crate) fn input_style(disabled: bool, _cx: &App) -> (Hsla, Hsla) {
    let p = MoonSkinPalette::TERMINAL;
    if disabled {
        (moon_color(p.panel, 0.55), moon_color(p.text_muted, 0.45))
    } else {
        (moon_color(p.shell_high, 1.0), moon_color(p.text_soft, 1.0))
    }
}

#[derive(Clone, Copy)]
struct MoonInputMetrics {
    height: Pixels,
    font_size: Pixels,
    line_height: Pixels,
    pad_x: Pixels,
    radius: Pixels,
    gap: Pixels,
}

impl MoonInputMetrics {
    fn for_size(size: Size) -> Self {
        match size {
            Size::XSmall | Size::Small => Self {
                height: px(22.),
                font_size: px(10.),
                line_height: px(13.),
                pad_x: px(7.),
                radius: px(4.),
                gap: px(6.),
            },
            Size::Size(height) => Self {
                height,
                font_size: height * 0.38,
                line_height: height * 0.5,
                pad_x: px(9.),
                radius: px(4.),
                gap: px(7.),
            },
            Size::Medium | Size::Large => Self {
                height: px(28.),
                font_size: px(10.5),
                line_height: px(14.),
                pad_x: px(9.),
                radius: px(4.),
                gap: px(7.),
            },
        }
    }
}

/// A text input element bind to an [`InputState`].
#[derive(IntoElement)]
pub struct Input {
    state: Entity<InputState>,
    style: StyleRefinement,
    size: Size,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
    height: Option<DefiniteLength>,
    appearance: bool,
    cleanable: bool,
    mask_toggle: bool,
    disabled: bool,
    bordered: bool,
    focus_bordered: bool,
    tab_index: isize,
    selected: bool,

    /// An optional context menu builder to allow a custom context menu on the input.
    ///
    /// If set, this overrides the built-in context menu.
    context_menu_builder: Option<Rc<dyn Fn(NativeMenu, &mut Window, &mut App) -> NativeMenu>>,
}

impl Sizable for Input {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Selectable for Input {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl Input {
    /// Create a new [`Input`] element bind to the [`InputState`].
    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            state: state.clone(),
            size: Size::default(),
            style: StyleRefinement::default(),
            prefix: None,
            suffix: None,
            height: None,
            appearance: true,
            cleanable: false,
            mask_toggle: false,
            disabled: false,
            bordered: true,
            focus_bordered: true,
            tab_index: 0,
            selected: false,
            context_menu_builder: None,
        }
    }

    pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
        self.prefix = Some(prefix.into_any_element());
        self
    }

    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }

    /// Set full height of the input (Multi-line only).
    pub fn h_full(mut self) -> Self {
        self.height = Some(relative(1.));
        self
    }

    /// Set height of the input (Multi-line only).
    pub fn h(mut self, height: impl Into<DefiniteLength>) -> Self {
        self.height = Some(height.into());
        self
    }

    /// Set the appearance of the input field, if false the input field will no border, background.
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }

    /// Set the bordered for the input, default: true
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// Set focus border for the input, default is true.
    pub fn focus_bordered(mut self, bordered: bool) -> Self {
        self.focus_bordered = bordered;
        self
    }

    /// Set whether to show the clear button when the input field is not empty, default is false.
    pub fn cleanable(mut self, cleanable: bool) -> Self {
        self.cleanable = cleanable;
        self
    }

    /// Set to enable toggle button for password mask state.
    pub fn mask_toggle(mut self) -> Self {
        self.mask_toggle = true;
        self
    }

    /// Set to disable the input field.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the tab index for the input, default is 0.
    pub fn tab_index(mut self, index: isize) -> Self {
        self.tab_index = index;
        self
    }

    /// Sets a custom context menu builder for the input, shown as a native OS menu.
    ///
    /// If set, this overrides the built-in right-click context menu.
    pub fn context_menu(
        mut self,
        f: impl Fn(NativeMenu, &mut Window, &mut App) -> NativeMenu + 'static,
    ) -> Self {
        self.context_menu_builder = Some(Rc::new(f));
        self
    }

    fn render_toggle_mask_button(state: &Entity<InputState>, cx: &App) -> impl IntoElement {
        let masked = state.read(cx).masked;
        Button::new("toggle-mask")
            .icon(if masked {
                IconName::Eye
            } else {
                IconName::EyeOff
            })
            .xsmall()
            .ghost()
            .tab_stop(false)
            .on_click({
                let state = state.clone();
                move |_, window, cx| {
                    state.update(cx, |state, cx| {
                        state.set_masked(!state.masked, window, cx);
                    })
                }
            })
    }

    /// This method must after the refine_style.
    fn render_editor(
        paddings: EdgesRefinement<DefiniteLength>,
        input_state: &Entity<InputState>,
        state: &InputState,
        window: &Window,
    ) -> impl IntoElement {
        let base_size = window.text_style().font_size;
        let rem_size = window.rem_size();

        let paddings = Edges {
            left: paddings
                .left
                .map(|v| v.to_pixels(base_size, rem_size))
                .unwrap_or(px(0.)),
            right: paddings
                .right
                .map(|v| v.to_pixels(base_size, rem_size))
                .unwrap_or(px(0.)),
            top: paddings
                .top
                .map(|v| v.to_pixels(base_size, rem_size))
                .unwrap_or(px(0.)),
            bottom: paddings
                .bottom
                .map(|v| v.to_pixels(base_size, rem_size))
                .unwrap_or(px(0.)),
        };

        state.editor_scrollbar_paddings.set(paddings);
        state.editor_scrollbar_snapshot.set(None);

        v_flex()
            .size_full()
            .children(state.search_panel.clone())
            .child(
                div()
                    .relative()
                    .flex_1()
                    .child(input_state.clone())
                    .child(EditorScrollbar::new(input_state.clone())),
            )
    }
}

impl Styled for Input {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Input {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let text_align = self.style.text.text_align.unwrap_or(TextAlign::Left);
        let metrics = MoonInputMetrics::for_size(self.size);
        let p = MoonSkinPalette::TERMINAL;

        self.state.update(cx, |state, _| {
            state.context_menu_builder = self.context_menu_builder.clone();
            state.disabled = self.disabled;
            state.size = self.size;

            // Only for single line mode
            if state.mode.is_single_line() {
                state.text_align = text_align;
            }
        });

        let state = self.state.read(cx);
        let focused = state.focus_handle.is_focused(window) && !state.disabled;
        let gap_x = metrics.gap;

        let (bg, _) = input_style(state.disabled, cx);
        let bg = if state.mode.is_code_editor() {
            cx.theme().editor_background()
        } else {
            bg
        };

        let prefix = self.prefix;
        let suffix = self.suffix;
        let show_clear_button = self.cleanable
            && !state.disabled
            && !state.loading
            && state.text.len() > 0
            && state.mode.is_single_line();
        let has_suffix = suffix.is_some() || state.loading || self.mask_toggle || show_clear_button;

        div()
            .id(("input", self.state.entity_id()))
            .flex()
            .key_context(crate::input::CONTEXT)
            .track_focus(&state.focus_handle.clone())
            .tab_index(self.tab_index)
            .when(!state.disabled, |this| {
                this.on_action(window.listener_for(&self.state, InputState::backspace))
                    .on_action(window.listener_for(&self.state, InputState::delete))
                    .on_action(
                        window.listener_for(&self.state, InputState::delete_to_beginning_of_line),
                    )
                    .on_action(window.listener_for(&self.state, InputState::delete_to_end_of_line))
                    .on_action(window.listener_for(&self.state, InputState::delete_previous_word))
                    .on_action(window.listener_for(&self.state, InputState::delete_next_word))
                    .on_action(window.listener_for(&self.state, InputState::enter))
                    .on_action(window.listener_for(&self.state, InputState::escape))
                    .on_action(window.listener_for(&self.state, InputState::paste))
                    .on_action(window.listener_for(&self.state, InputState::cut))
                    .on_action(window.listener_for(&self.state, InputState::undo))
                    .on_action(window.listener_for(&self.state, InputState::redo))
                    .when(state.mode.is_multi_line(), |this| {
                        this.on_action(window.listener_for(&self.state, InputState::indent_inline))
                            .on_action(window.listener_for(&self.state, InputState::outdent_inline))
                            .on_action(window.listener_for(&self.state, InputState::indent_block))
                            .on_action(window.listener_for(&self.state, InputState::outdent_block))
                    })
                    .on_action(
                        window.listener_for(&self.state, InputState::on_action_toggle_code_actions),
                    )
            })
            .on_action(window.listener_for(&self.state, InputState::left))
            .on_action(window.listener_for(&self.state, InputState::right))
            .on_action(window.listener_for(&self.state, InputState::select_left))
            .on_action(window.listener_for(&self.state, InputState::select_right))
            .when(state.mode.is_multi_line(), |this| {
                let result = this
                    .on_action(window.listener_for(&self.state, InputState::up))
                    .on_action(window.listener_for(&self.state, InputState::down))
                    .on_action(window.listener_for(&self.state, InputState::select_up))
                    .on_action(window.listener_for(&self.state, InputState::select_down))
                    .on_action(window.listener_for(&self.state, InputState::page_up))
                    .on_action(window.listener_for(&self.state, InputState::page_down));

                let result = result.on_action(
                    window.listener_for(&self.state, InputState::on_action_go_to_definition),
                );

                result
            })
            .on_action(window.listener_for(&self.state, InputState::select_all))
            .on_action(window.listener_for(&self.state, InputState::select_to_start_of_line))
            .on_action(window.listener_for(&self.state, InputState::select_to_end_of_line))
            .on_action(window.listener_for(&self.state, InputState::select_to_previous_word))
            .on_action(window.listener_for(&self.state, InputState::select_to_next_word))
            .on_action(window.listener_for(&self.state, InputState::home))
            .on_action(window.listener_for(&self.state, InputState::end))
            .on_action(window.listener_for(&self.state, InputState::move_to_start))
            .on_action(window.listener_for(&self.state, InputState::move_to_end))
            .on_action(window.listener_for(&self.state, InputState::move_to_previous_word))
            .on_action(window.listener_for(&self.state, InputState::move_to_next_word))
            .on_action(window.listener_for(&self.state, InputState::select_to_start))
            .on_action(window.listener_for(&self.state, InputState::select_to_end))
            .on_action(window.listener_for(&self.state, InputState::show_character_palette))
            .on_action(window.listener_for(&self.state, InputState::copy))
            .on_action(window.listener_for(&self.state, InputState::on_action_search))
            .on_key_down(window.listener_for(&self.state, InputState::on_key_down))
            .on_mouse_down(
                MouseButton::Left,
                window.listener_for(&self.state, InputState::on_mouse_down),
            )
            .on_mouse_down(
                MouseButton::Right,
                window.listener_for(&self.state, InputState::on_mouse_down),
            )
            .on_mouse_up(
                MouseButton::Left,
                window.listener_for(&self.state, InputState::on_mouse_up),
            )
            .on_mouse_up(
                MouseButton::Right,
                window.listener_for(&self.state, InputState::on_mouse_up),
            )
            .on_mouse_move(window.listener_for(&self.state, InputState::on_mouse_move))
            .on_scroll_wheel(window.listener_for(&self.state, InputState::on_scroll_wheel))
            .size_full()
            .line_height(metrics.line_height)
            .px(metrics.pad_x)
            .h(metrics.height)
            .text_size(metrics.font_size)
            .font_family("Geist Mono")
            .text_color(moon_color(
                if state.disabled { p.text_muted } else { p.text_soft },
                if state.disabled { 0.45 } else { 1.0 },
            ))
            .when(!self.disabled, |this| this.cursor_text())
            .items_center()
            .when(state.mode.is_multi_line(), |this| {
                this.h_auto()
                    .when_some(self.height, |this, height| this.h(height))
            })
            .when(self.appearance, |this| {
                this.bg(bg)
                    .rounded(metrics.radius)
                    .when(self.bordered, |this| {
                        this.border_color(moon_color(
                            if self.selected { p.blue } else { p.border },
                            if self.selected { 0.78 } else { 1.0 },
                        ))
                            .border_1()
                            .when(focused && self.focus_bordered, |this| {
                                this.border_color(moon_color(p.blue, 0.92))
                            })
                    })
                    .when(!state.disabled, |this| {
                        this.hover(|this| {
                            this.bg(moon_color(0x1D2025, 1.0)).border_color(moon_color(
                                if self.selected { p.blue } else { 0x343840 },
                                if self.selected { 0.82 } else { 1.0 },
                            ))
                        })
                    })
            })
            .items_center()
            .gap(gap_x)
            .refine_style(&self.style)
            .children(prefix)
            .when(state.mode.is_multi_line(), |mut this| {
                let paddings = this.style().padding.clone();
                this.child(Self::render_editor(paddings, &self.state, &state, window))
            })
            .when(!state.mode.is_multi_line(), |this| {
                this.child(self.state.clone())
            })
            .when(has_suffix, |this| {
                this.pr(metrics.pad_x).child(
                    h_flex()
                        .id("suffix")
                        .gap(gap_x)
                        .items_center()
                        .when(state.loading, |this| {
                            this.child(Spinner::new().color(cx.theme().muted_foreground))
                        })
                        .when(self.mask_toggle, |this| {
                            this.child(Self::render_toggle_mask_button(&self.state, cx))
                        })
                        .when(show_clear_button, |this| {
                            this.child(clear_button(cx).on_click({
                                let state = self.state.clone();
                                move |_, window, cx| {
                                    state.update(cx, |state, cx| {
                                        state.clean(window, cx);
                                        state.focus(window, cx);
                                    })
                                }
                            }))
                        })
                        .children(suffix),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_moon_input_metrics_match_terminal_palette() {
        let compact = MoonInputMetrics::for_size(Size::Small);
        assert_eq!(compact.height, px(22.));
        assert_eq!(compact.font_size, px(10.));
        assert_eq!(compact.line_height, px(13.));
        assert_eq!(compact.pad_x, px(7.));
        assert_eq!(compact.radius, px(4.));
        assert_eq!(compact.gap, px(6.));

        let normal = MoonInputMetrics::for_size(Size::Medium);
        assert_eq!(normal.height, px(28.));
        assert_eq!(normal.font_size, px(10.5));
        assert_eq!(normal.line_height, px(14.));
        assert_eq!(normal.pad_x, px(9.));
        assert_eq!(normal.radius, px(4.));
        assert_eq!(normal.gap, px(7.));
    }

    #[gpui::test]
    fn test_input_style_uses_moon_tokens(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        let window = cx.add_empty_window();
        window.update(|_, cx| {
            let p = MoonSkinPalette::TERMINAL;
            assert_eq!(input_style(false, cx), (moon_color(p.shell_high, 1.0), moon_color(p.text_soft, 1.0)));
            assert_eq!(input_style(true, cx), (moon_color(p.panel, 0.55), moon_color(p.text_muted, 0.45)));
        });
    }
}
