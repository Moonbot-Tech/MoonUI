# Key Dispatch

GPUI is designed for keyboard-first interactivity.

To expose functionality to the mouse, you render a button with a click handler.

To expose functionality to the keyboard, you declare an _action_, listen for it on a focused element, and register a key binding with `App::bind_keys`.

Unit actions are declared with the `actions!` macro. The first argument is the namespace that appears in the action name (`menu::MoveUp`):

```rust
actions!(menu, [MoveUp, MoveDown]);
```

An action that carries data derives `Action` instead. The derive requires `Clone` and `PartialEq`, plus `serde::Deserialize` and `schemars::JsonSchema` unless you pass `#[action(no_json)]`:

```rust
#[derive(Clone, PartialEq, serde::Deserialize, schemars::JsonSchema, gpui::Action)]
#[action(namespace = menu)]
pub struct Move {
    pub select: bool,
}
```

`on_action` on an element takes `Fn(&Action, &mut Window, &mut App)`. It does not receive the view. `Context::listener` adapts a view method so the callback can update that view. The element has to be on the focus path (`track_focus`); `key_context` is the name a binding's context matches:

```rust
impl Menu {
    fn move_up(&mut self, _: &MoveUp, _: &mut Window, cx: &mut Context<Self>) {
        // ...
    }

    fn move_down(&mut self, _: &MoveDown, _: &mut Window, cx: &mut Context<Self>) {
        // ...
    }
}

impl Render for Menu {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus_handle(cx))
            .key_context("menu")
            .on_action(cx.listener(Self::move_up))
            .on_action(cx.listener(Self::move_down))
    }
}
```

Register the keys on the app. `KeyBinding::new` takes the keystroke string, an action value, and an optional context. A context of `Some("menu")` fires only when the focused path contains `key_context("menu")`. `None` matches every context.

```rust
cx.bind_keys([
    KeyBinding::new("up", MoveUp, Some("menu")),
    KeyBinding::new("down", MoveDown, Some("menu")),
    KeyBinding::new("shift-up", Move { select: true }, Some("menu")),
]);
```

A binding can be a sequence of keystrokes separated by spaces, such as `"cmd-k left"`.
