# Contexts

GPUI makes extensive use of _context parameters_ (typically named `cx`) to provide access to application state and services. These contexts are references passed to functions, enabling interaction with global state, windows, entities, and system services.

---

## `App`

The root context granting access to the application's global state. This context owns all entities' data and can be used to read or update the data referenced by an `Entity<T>`.

## `Context<T>`

A context provided when interacting with an `Entity<T>`, with additional methods related to that specific entity such as notifying observers and emitting events. This context dereferences into `App`, meaning any function which can take an `App` reference can also take a `Context<T>` reference, allowing you to access the application's global state.

## `AsyncApp` and `AsyncWindowContext`

`App::to_async` and `TestAppContext::to_async` return an `AsyncApp`. `Context<T>` dereferences to `App`, so `to_async` on a context reference is `App::to_async` and returns an `AsyncApp` as well. `Window::to_async` takes `&App` and returns an `AsyncWindowContext`. `Context::spawn` passes the task a `WeakEntity<T>` and an `&mut AsyncApp`, and `Context::spawn_in` passes an `&mut AsyncWindowContext`. Both async contexts have a static lifetime and can be held across `await` points.

`AsyncApp` holds a weak reference to the app. `Entity::update` on an `AsyncApp` panics if the app has already been dropped, and that call does not return `Result`. `AsyncApp::update_window` and `AsyncWindowContext::update` return `Result` when the app has been dropped, the app is quitting, or the window is gone. `WeakEntity::update` returns `Result` when the entity was released, on every context, not only an async one.

## `TestAppContext`

A `#[gpui::test]` that takes `&mut TestAppContext` receives one. The context owns the app through `Rc`, so a dropped app is not one of its failure modes. `TestAppContext::update_window` returns `Result` when the window is gone. `VisualTestContext::update` unwraps that result and panics. The test context also exposes input and window helpers such as `simulate_input` and `simulate_window_resize`.

---

# Non-Context Core Types

## `Window`

`Window` is the state of one window, including its root view (an `Entity` that implements `Render`). It is not a context. `WindowHandle::update` takes an `AppContext` — `&mut App`, or a `Context<T>`, which dereferences to `App` — and a closure. The closure receives `&mut V`, `&mut Window`, and `&mut Context<V>`. The call returns `Result`, because the window may already be closed.

## `Entity<T>`

A handle to a structure requiring state. This data is owned by the `App` and can be accessed and modified via references to contexts. If `T` implements `Render`, then the entity is sometimes referred to as a view. Entities can be observed by other entities and windows, allowing a closure to be called when `notify` is called on the entity's `Context`.
