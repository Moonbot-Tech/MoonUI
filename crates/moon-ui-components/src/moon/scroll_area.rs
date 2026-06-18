use std::time::Instant;

use gpui::prelude::FluentBuilder;
use gpui::*;

use super::tokens::{MoonPalette, rgba_from};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonScrollAxis {
    Vertical,
    Horizontal,
    Both,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonScrollbarVisibility {
    Scrolling,
    Hover,
    Always,
    Hidden,
}

impl MoonScrollbarVisibility {
    pub(crate) fn is_visible(self) -> bool {
        !matches!(self, Self::Hidden)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MoonScrollbarDragAxis {
    Vertical,
    Horizontal,
}

#[derive(Clone, Debug)]
struct MoonScrollbarDrag {
    axis: MoonScrollbarDragAxis,
}

impl Render for MoonScrollbarDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

#[derive(Default)]
struct MoonScrollbarRuntimeState {
    active_axis: Option<MoonScrollbarDragAxis>,
    hovered_axis: Option<MoonScrollbarDragAxis>,
    last_offset: Point<Pixels>,
    last_scroll_at: Option<Instant>,
    grab_x: f32,
    grab_y: f32,
}

#[derive(Clone, Copy, Debug)]
struct MoonScrollbarRuntimeSnapshot {
    active_axis: Option<MoonScrollbarDragAxis>,
    hovered_axis: Option<MoonScrollbarDragAxis>,
    vertical_alpha: f32,
    horizontal_alpha: f32,
    needs_animation: bool,
}

impl MoonScrollbarRuntimeState {
    fn snapshot(
        &mut self,
        offset: Point<Pixels>,
        visibility: MoonScrollbarVisibility,
        now: Instant,
    ) -> MoonScrollbarRuntimeSnapshot {
        if self.last_offset != offset {
            self.last_offset = offset;
            self.last_scroll_at = Some(now);
        }
        let vertical_alpha = self.alpha_for(MoonScrollbarDragAxis::Vertical, visibility, now);
        let horizontal_alpha = self.alpha_for(MoonScrollbarDragAxis::Horizontal, visibility, now);
        MoonScrollbarRuntimeSnapshot {
            active_axis: self.active_axis,
            hovered_axis: self.hovered_axis,
            vertical_alpha,
            horizontal_alpha,
            needs_animation: matches!(visibility, MoonScrollbarVisibility::Scrolling)
                && (vertical_alpha > 0.0 || horizontal_alpha > 0.0)
                && self.active_axis.is_none(),
        }
    }

    fn alpha_for(
        &self,
        axis: MoonScrollbarDragAxis,
        visibility: MoonScrollbarVisibility,
        now: Instant,
    ) -> f32 {
        if self.active_axis == Some(axis) {
            return 1.0;
        }
        match visibility {
            MoonScrollbarVisibility::Always => 1.0,
            MoonScrollbarVisibility::Hidden => 0.0,
            MoonScrollbarVisibility::Hover => {
                if self.hovered_axis == Some(axis) {
                    1.0
                } else {
                    0.0
                }
            }
            MoonScrollbarVisibility::Scrolling => self
                .last_scroll_at
                .map(|last| {
                    let elapsed = now.duration_since(last).as_secs_f32();
                    if elapsed <= 2.0 {
                        1.0
                    } else if elapsed <= 3.0 {
                        1.0 - (elapsed - 2.0)
                    } else {
                        0.0
                    }
                })
                .unwrap_or(0.0),
        }
    }
}

pub fn moon_scrollbar_overlay_with_palette(
    id: impl Into<SharedString>,
    scroll_handle: &ScrollHandle,
    axis: MoonScrollAxis,
    visibility: MoonScrollbarVisibility,
    p: MoonPalette,
    window: &mut Window,
    cx: &mut App,
) -> Option<AnyElement> {
    if !visibility.is_visible() {
        return None;
    }

    let id = id.into();
    let bounds = scroll_handle.bounds();
    let max = scroll_handle.max_offset();
    let offset = scroll_handle.offset();
    let track = rgba_from(p.panel_high, 0.34);
    let state = window.use_keyed_state(
        ElementId::from(SharedString::from(format!("{id}:state"))),
        cx,
        |_, _| MoonScrollbarRuntimeState::default(),
    );
    let now = Instant::now();
    let runtime = state.update(cx, |state, _| state.snapshot(offset, visibility, now));
    if runtime.needs_animation {
        window.request_animation_frame();
    }

    let mut layer = div()
        .id(ElementId::from(id.clone()))
        .absolute()
        .top_0()
        .right_0()
        .bottom_0()
        .left_0();

    if matches!(axis, MoonScrollAxis::Vertical | MoonScrollAxis::Both) && f32::from(max.y) > 0.0 {
        let viewport = f32::from(bounds.size.height).max(1.0);
        let content = viewport + f32::from(max.y);
        let thumb_h = (viewport / content * viewport).clamp(18.0_f32.min(viewport), viewport);
        let top = (f32::from(offset.y) / f32::from(max.y) * (viewport - thumb_h))
            .clamp(0.0, viewport - thumb_h);
        let state_for_down = state.clone();
        let state_for_track_hover = state.clone();
        let state_for_thumb_hover = state.clone();
        let state_for_move = state.clone();
        let state_for_up = state.clone();
        let state_for_up_out = state.clone();
        let scroll_for_move = scroll_handle.clone();
        let scroll_for_track = scroll_handle.clone();
        let scroll_bounds = bounds;
        let drag = MoonScrollbarDrag {
            axis: MoonScrollbarDragAxis::Vertical,
        };
        layer = layer.child(
            div()
                .id(ElementId::from(SharedString::from(format!("{id}:v-track"))))
                .absolute()
                .right(px(0.0))
                .top(px(0.0))
                .bottom(px(0.0))
                .w(px(8.0))
                .cursor(CursorStyle::Arrow)
                .on_hover(move |hovered, _window, cx| {
                    state_for_track_hover.update(cx, |state, cx| {
                        state.hovered_axis = (*hovered)
                            .then_some(MoonScrollbarDragAxis::Vertical)
                            .or_else(|| {
                                (state.hovered_axis != Some(MoonScrollbarDragAxis::Vertical))
                                    .then_some(state.hovered_axis)
                                    .flatten()
                            });
                        cx.notify();
                    });
                })
                .on_mouse_down(
                    MouseButton::Left,
                    move |event: &MouseDownEvent, _window, cx| {
                        let track_len = (viewport - thumb_h).max(1.0);
                        let local = (f32::from(event.position.y)
                            - f32::from(scroll_bounds.origin.y)
                            - thumb_h * 0.5)
                            .clamp(0.0, track_len);
                        let ratio = local / track_len;
                        let mut current = scroll_for_track.offset();
                        current.y = px(ratio * f32::from(max.y));
                        scroll_for_track.set_offset(current);
                        cx.stop_propagation();
                    },
                ),
        );
        layer = layer.child(
            div()
                .id(ElementId::from(SharedString::from(format!("{id}:v"))))
                .absolute()
                .right(px(2.0))
                .top(px(top + 2.0))
                .w(px(4.0))
                .h(px((thumb_h - 4.0).max(12.0)))
                .rounded_full()
                .bg(
                    if runtime.active_axis == Some(MoonScrollbarDragAxis::Vertical) {
                        rgba_from(p.text, 0.82)
                    } else if runtime.hovered_axis == Some(MoonScrollbarDragAxis::Vertical) {
                        rgba_from(p.text_soft, 0.72 * runtime.vertical_alpha)
                    } else {
                        rgba_from(p.text_muted, 0.58 * runtime.vertical_alpha)
                    },
                )
                .cursor(CursorStyle::OpenHand)
                .hover(move |this| this.bg(rgba_from(p.text_soft, 0.72)))
                .on_hover(move |hovered, _window, cx| {
                    state_for_thumb_hover.update(cx, |state, cx| {
                        state.hovered_axis = (*hovered)
                            .then_some(MoonScrollbarDragAxis::Vertical)
                            .or_else(|| {
                                (state.hovered_axis != Some(MoonScrollbarDragAxis::Vertical))
                                    .then_some(state.hovered_axis)
                                    .flatten()
                            });
                        cx.notify();
                    });
                })
                .on_mouse_down(
                    MouseButton::Left,
                    move |event: &MouseDownEvent, _window, cx| {
                        state_for_down.update(cx, |state, cx| {
                            state.active_axis = Some(MoonScrollbarDragAxis::Vertical);
                            state.grab_y = (f32::from(event.position.y)
                                - f32::from(scroll_bounds.origin.y)
                                - top)
                                .clamp(0.0, thumb_h);
                            cx.notify();
                        });
                        cx.stop_propagation();
                    },
                )
                .on_mouse_up(MouseButton::Left, move |_, _window, cx| {
                    state_for_up.update(cx, |state, cx| {
                        state.active_axis = None;
                        cx.notify();
                    });
                })
                .on_mouse_up_out(MouseButton::Left, move |_, _window, cx| {
                    state_for_up_out.update(cx, |state, cx| {
                        state.active_axis = None;
                        cx.notify();
                    });
                })
                .on_drag(drag, |drag, _, _, cx| {
                    cx.stop_propagation();
                    cx.new(|_| drag.clone())
                })
                .on_drag_move(window.listener_for(
                    &state_for_move,
                    move |state, event: &DragMoveEvent<MoonScrollbarDrag>, _, cx| {
                        if event.drag(cx).axis != MoonScrollbarDragAxis::Vertical {
                            return;
                        }
                        let track_len = (viewport - thumb_h).max(1.0);
                        let local = (f32::from(event.event.position.y)
                            - f32::from(scroll_bounds.origin.y)
                            - state.grab_y)
                            .clamp(0.0, track_len);
                        let ratio = local / track_len;
                        let mut current = scroll_for_move.offset();
                        current.y = px(ratio * f32::from(max.y));
                        scroll_for_move.set_offset(current);
                        cx.notify();
                    },
                )),
        );
    }

    if matches!(axis, MoonScrollAxis::Horizontal | MoonScrollAxis::Both) && f32::from(max.x) > 0.0 {
        let viewport = f32::from(bounds.size.width).max(1.0);
        let content = viewport + f32::from(max.x);
        let thumb_w = (viewport / content * viewport).clamp(18.0_f32.min(viewport), viewport);
        let left = (f32::from(offset.x) / f32::from(max.x) * (viewport - thumb_w))
            .clamp(0.0, viewport - thumb_w);
        let state_for_down = state.clone();
        let state_for_track_hover = state.clone();
        let state_for_thumb_hover = state.clone();
        let state_for_move = state.clone();
        let state_for_up = state.clone();
        let state_for_up_out = state.clone();
        let scroll_for_move = scroll_handle.clone();
        let scroll_for_track = scroll_handle.clone();
        let scroll_bounds = bounds;
        let drag = MoonScrollbarDrag {
            axis: MoonScrollbarDragAxis::Horizontal,
        };
        layer = layer.child(
            div()
                .id(ElementId::from(SharedString::from(format!("{id}:h-track"))))
                .absolute()
                .left(px(0.0))
                .right(px(0.0))
                .bottom(px(0.0))
                .h(px(8.0))
                .cursor(CursorStyle::Arrow)
                .on_hover(move |hovered, _window, cx| {
                    state_for_track_hover.update(cx, |state, cx| {
                        state.hovered_axis = (*hovered)
                            .then_some(MoonScrollbarDragAxis::Horizontal)
                            .or_else(|| {
                                (state.hovered_axis != Some(MoonScrollbarDragAxis::Horizontal))
                                    .then_some(state.hovered_axis)
                                    .flatten()
                            });
                        cx.notify();
                    });
                })
                .on_mouse_down(
                    MouseButton::Left,
                    move |event: &MouseDownEvent, _window, cx| {
                        let track_len = (viewport - thumb_w).max(1.0);
                        let local = (f32::from(event.position.x)
                            - f32::from(scroll_bounds.origin.x)
                            - thumb_w * 0.5)
                            .clamp(0.0, track_len);
                        let ratio = local / track_len;
                        let mut current = scroll_for_track.offset();
                        current.x = px(ratio * f32::from(max.x));
                        scroll_for_track.set_offset(current);
                        cx.stop_propagation();
                    },
                ),
        );
        layer = layer.child(
            div()
                .id(ElementId::from(SharedString::from(format!("{id}:h"))))
                .absolute()
                .left(px(left + 2.0))
                .bottom(px(2.0))
                .w(px((thumb_w - 4.0).max(12.0)))
                .h(px(4.0))
                .rounded_full()
                .bg(
                    if runtime.active_axis == Some(MoonScrollbarDragAxis::Horizontal) {
                        rgba_from(p.text, 0.82)
                    } else if runtime.hovered_axis == Some(MoonScrollbarDragAxis::Horizontal) {
                        rgba_from(p.text_soft, 0.72 * runtime.horizontal_alpha)
                    } else {
                        rgba_from(p.text_muted, 0.58 * runtime.horizontal_alpha)
                    },
                )
                .cursor(CursorStyle::OpenHand)
                .hover(move |this| this.bg(rgba_from(p.text_soft, 0.72)))
                .on_hover(move |hovered, _window, cx| {
                    state_for_thumb_hover.update(cx, |state, cx| {
                        state.hovered_axis = (*hovered)
                            .then_some(MoonScrollbarDragAxis::Horizontal)
                            .or_else(|| {
                                (state.hovered_axis != Some(MoonScrollbarDragAxis::Horizontal))
                                    .then_some(state.hovered_axis)
                                    .flatten()
                            });
                        cx.notify();
                    });
                })
                .on_mouse_down(
                    MouseButton::Left,
                    move |event: &MouseDownEvent, _window, cx| {
                        state_for_down.update(cx, |state, cx| {
                            state.active_axis = Some(MoonScrollbarDragAxis::Horizontal);
                            state.grab_x = (f32::from(event.position.x)
                                - f32::from(scroll_bounds.origin.x)
                                - left)
                                .clamp(0.0, thumb_w);
                            cx.notify();
                        });
                        cx.stop_propagation();
                    },
                )
                .on_mouse_up(MouseButton::Left, move |_, _window, cx| {
                    state_for_up.update(cx, |state, cx| {
                        state.active_axis = None;
                        cx.notify();
                    });
                })
                .on_mouse_up_out(MouseButton::Left, move |_, _window, cx| {
                    state_for_up_out.update(cx, |state, cx| {
                        state.active_axis = None;
                        cx.notify();
                    });
                })
                .on_drag(drag, |drag, _, _, cx| {
                    cx.stop_propagation();
                    cx.new(|_| drag.clone())
                })
                .on_drag_move(window.listener_for(
                    &state_for_move,
                    move |state, event: &DragMoveEvent<MoonScrollbarDrag>, _, cx| {
                        if event.drag(cx).axis != MoonScrollbarDragAxis::Horizontal {
                            return;
                        }
                        let track_len = (viewport - thumb_w).max(1.0);
                        let local = (f32::from(event.event.position.x)
                            - f32::from(scroll_bounds.origin.x)
                            - state.grab_x)
                            .clamp(0.0, track_len);
                        let ratio = local / track_len;
                        let mut current = scroll_for_move.offset();
                        current.x = px(ratio * f32::from(max.x));
                        scroll_for_move.set_offset(current);
                        cx.notify();
                    },
                )),
        );
    }

    Some(
        layer
            .when(
                matches!(visibility, MoonScrollbarVisibility::Always),
                |this| {
                    this.child(
                        div()
                            .absolute()
                            .right(px(0.0))
                            .top(px(0.0))
                            .bottom(px(0.0))
                            .w(px(1.0))
                            .bg(track),
                    )
                },
            )
            .into_any_element(),
    )
}
