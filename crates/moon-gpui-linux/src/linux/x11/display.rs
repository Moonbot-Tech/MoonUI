use std::{cell::RefCell, rc::Rc};

use anyhow::Context as _;
use uuid::Uuid;
use x11rb::{
    connection::Connection as _,
    protocol::{
        randr::{self, ConnectionExt as _},
        xproto::{self, ConnectionExt as _},
    },
    xcb_ffi::XCBConnection,
};

use gpui::{Bounds, DisplayId, Pixels, PlatformDisplay, Size, point, px, size};

mod geometry;

/// One display of an X screen: a RandR monitor, or the whole screen when the server names none.
#[derive(Debug)]
pub(crate) struct X11Display {
    id: DisplayId,
    x_screen_index: usize,
    bounds: Bounds<Pixels>,
    uuid: Uuid,
}

impl X11Display {
    /// Describe a whole X screen as one display.
    ///
    /// The fallback for a server that cannot list monitors (RandR older than 1.5, or none). On a
    /// multi-monitor desktop the X screen spans every monitor, so this rectangle must not be used
    /// where a monitor can be named: a window sized from it straddles the monitors.
    pub(crate) fn new(
        xcb: &XCBConnection,
        scale_factor: f32,
        x_screen_index: usize,
    ) -> anyhow::Result<Self> {
        let screen = xcb
            .setup()
            .roots
            .get(x_screen_index)
            .with_context(|| format!("No screen found with index {x_screen_index}"))?;
        Ok(Self {
            id: DisplayId::new(x_screen_index as u64),
            x_screen_index,
            bounds: Bounds {
                origin: Default::default(),
                size: Size {
                    width: px(screen.width_in_pixels as f32 / scale_factor),
                    height: px(screen.height_in_pixels as f32 / scale_factor),
                },
            },
            uuid: Uuid::from_bytes([0; 16]),
        })
    }

    /// Read the X screen index out of a display id, whichever kind of display it names.
    pub(crate) fn screen_index_of(id: DisplayId) -> usize {
        geometry::screen_index_of(u64::from(id))
    }

    /// List the active monitors of one X screen as displays, the primary first.
    ///
    /// `None` when the server could not answer, which is not the same as an empty list: an empty
    /// list is the server saying that no monitor shows a picture.
    fn monitors_of_screen(
        xcb: &XCBConnection,
        scale_factor: f32,
        x_screen_index: usize,
    ) -> Option<Vec<Rc<Self>>> {
        let root = xcb.setup().roots.get(x_screen_index)?.root;
        let displays = query_monitors(xcb, root)?
            .into_iter()
            .filter_map(|monitor| {
                let rect = geometry::logical_rect(
                    monitor.x,
                    monitor.y,
                    monitor.width,
                    monitor.height,
                    scale_factor,
                )?;
                // The derivation the Wayland backend applies to an output's name. A monitor
                // whose name the server would not give still gets an identity of its own, from
                // the atom that names it, so two of them never share one.
                let uuid = match &monitor.name {
                    Some(name) => Uuid::new_v5(&Uuid::NAMESPACE_DNS, name),
                    None => Uuid::new_v5(
                        &Uuid::NAMESPACE_DNS,
                        format!("x11-monitor-atom-{}", monitor.name_atom).as_bytes(),
                    ),
                };
                Some(Rc::new(Self {
                    id: DisplayId::new(geometry::monitor_display_id(
                        x_screen_index,
                        monitor.name_atom,
                    )),
                    x_screen_index,
                    bounds: Bounds {
                        origin: point(px(rect.x), px(rect.y)),
                        size: size(px(rect.width), px(rect.height)),
                    },
                    uuid,
                }))
            })
            .collect();
        Some(displays)
    }

    /// The whole X screen as the only display of that screen, or nothing if it cannot be read.
    fn whole_screen(
        xcb: &XCBConnection,
        scale_factor: f32,
        x_screen_index: usize,
    ) -> Vec<Rc<Self>> {
        Self::new(xcb, scale_factor, x_screen_index)
            .ok()
            .map(Rc::new)
            .into_iter()
            .collect()
    }

    fn rect(&self) -> geometry::Rect {
        rect_of(self.bounds)
    }

    /// Whether this is the whole-screen fallback rather than one monitor.
    fn is_whole_screen(&self) -> bool {
        u64::from(self.id) == self.x_screen_index as u64
    }
}

impl PlatformDisplay for X11Display {
    fn id(&self) -> DisplayId {
        self.id
    }

    fn uuid(&self) -> anyhow::Result<Uuid> {
        Ok(self.uuid)
    }

    fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }
}

/// The displays of every X screen, shared by the client and each of its windows.
///
/// Held rather than asked of the server on demand: a window is asked for its display on every
/// move, and a server round trip per pointer motion is what makes a drag stutter. The client
/// refreshes the list when RandR reports a change.
#[derive(Clone, Default)]
pub(crate) struct X11Displays(Rc<RefCell<Vec<Rc<X11Display>>>>);

impl X11Displays {
    /// Read the displays of every X screen from the server.
    pub(crate) fn query(xcb: &XCBConnection, scale_factor: f32) -> Self {
        let displays = Self::default();
        displays.refresh(xcb, scale_factor);
        displays
    }

    /// Replace the list with the server's current answer.
    ///
    /// The server is asked before the list is borrowed, so a reader can never observe a list
    /// that is half replaced.
    ///
    /// A screen the server could not answer for keeps the displays it had. One failed request
    /// in the middle of a hotplug must not swap the monitors for the whole-screen display: every
    /// window's display id would stop resolving, and the next window would be sized from a
    /// rectangle spanning all the monitors.
    pub(crate) fn refresh(&self, xcb: &XCBConnection, scale_factor: f32) {
        let previous = self.all();
        let displays: Vec<Rc<X11Display>> = (0..xcb.setup().roots.len())
            .flat_map(|x_screen_index| {
                match X11Display::monitors_of_screen(xcb, scale_factor, x_screen_index) {
                    Some(monitors) if !monitors.is_empty() => monitors,
                    // The server answered and named nothing usable.
                    Some(_) => X11Display::whole_screen(xcb, scale_factor, x_screen_index),
                    None => {
                        let known: Vec<Rc<X11Display>> = previous
                            .iter()
                            .filter(|display| {
                                display.x_screen_index == x_screen_index
                                    && !display.is_whole_screen()
                            })
                            .cloned()
                            .collect();
                        if known.is_empty() {
                            // No monitor was ever known: a server that cannot list them, which
                            // answers this way at start and on every later change alike. That is
                            // its normal state, not a failure worth a warning.
                            X11Display::whole_screen(xcb, scale_factor, x_screen_index)
                        } else {
                            log::warn!(
                                "x11: the monitors of screen {x_screen_index} could not be read; \
                                 keeping the displays known so far"
                            );
                            known
                        }
                    }
                }
            })
            .collect();
        *self.0.borrow_mut() = displays;
    }

    /// Every display, grouped by X screen, the primary monitor of each screen first.
    pub(crate) fn all(&self) -> Vec<Rc<X11Display>> {
        self.0.borrow().clone()
    }

    /// The display with this id, if it is still attached.
    pub(crate) fn find(&self, id: DisplayId) -> Option<Rc<X11Display>> {
        self.0
            .borrow()
            .iter()
            .find(|display| display.id == id)
            .cloned()
    }

    /// The primary display of an X screen: its primary monitor, or the first one listed.
    pub(crate) fn primary(&self, x_screen_index: usize) -> Option<Rc<X11Display>> {
        self.0
            .borrow()
            .iter()
            .find(|display| display.x_screen_index == x_screen_index)
            .cloned()
    }

    /// The display of `x_screen_index` that a window with these bounds belongs to.
    pub(crate) fn containing(
        &self,
        x_screen_index: usize,
        bounds: Bounds<Pixels>,
    ) -> Option<Rc<X11Display>> {
        let displays = self.0.borrow();
        let on_screen: Vec<&Rc<X11Display>> = displays
            .iter()
            .filter(|display| display.x_screen_index == x_screen_index)
            .collect();
        let rects: Vec<geometry::Rect> = on_screen.iter().map(|display| display.rect()).collect();
        geometry::containing(rect_of(bounds), &rects).map(|index| on_screen[index].clone())
    }
}

/// Ask the server to report monitor changes on every X screen.
///
/// Without this the display list read at start would describe the desktop as it was then for the
/// rest of the session. A server without RandR fails here and keeps the whole-screen display.
pub(crate) fn watch_monitor_changes(xcb: &XCBConnection) -> anyhow::Result<()> {
    let mask = randr::NotifyMask::SCREEN_CHANGE
        | randr::NotifyMask::CRTC_CHANGE
        | randr::NotifyMask::OUTPUT_CHANGE;
    for screen in &xcb.setup().roots {
        xcb.randr_select_input(screen.root, mask)
            .context("X11 RandR SelectInput could not be sent")?
            .check()
            .context("X11 RandR SelectInput failed")?;
    }
    Ok(())
}

/// One active RandR monitor, in the server's device pixels.
struct Monitor {
    name_atom: u32,
    name: Option<Vec<u8>>,
    x: i16,
    y: i16,
    width: u16,
    height: u16,
}

/// List the active monitors of one X screen, the primary first.
///
/// `None` when the server cannot answer — RandR missing or older than 1.5, which introduced
/// monitors. An empty list is a real answer and is returned as such.
fn query_monitors(xcb: &XCBConnection, root: xproto::Window) -> Option<Vec<Monitor>> {
    let version = xcb.randr_query_version(1, 6).ok()?.reply().ok()?;
    if (version.major_version, version.minor_version) < (1, 5) {
        return None;
    }
    // `true`: active monitors only, the ones that show a picture.
    let reply = xcb.randr_get_monitors(root, true).ok()?.reply().ok()?;

    // Every name is requested before any reply is awaited, so the names cost one round trip.
    let name_cookies: Vec<_> = reply
        .monitors
        .iter()
        .map(|monitor| xcb.get_atom_name(monitor.name).ok())
        .collect();
    let mut monitors: Vec<(bool, Monitor)> = reply
        .monitors
        .iter()
        .zip(name_cookies)
        .map(|(monitor, name_cookie)| {
            let name = name_cookie
                .and_then(|cookie| cookie.reply().ok())
                .map(|reply| reply.name);
            (
                monitor.primary,
                Monitor {
                    name_atom: monitor.name,
                    name,
                    x: monitor.x,
                    y: monitor.y,
                    width: monitor.width,
                    height: monitor.height,
                },
            )
        })
        .collect();
    // Stable, so the server's order is kept among the monitors that are not primary.
    monitors.sort_by_key(|(primary, _)| !primary);
    Some(monitors.into_iter().map(|(_, monitor)| monitor).collect())
}

fn rect_of(bounds: Bounds<Pixels>) -> geometry::Rect {
    geometry::Rect {
        x: f32::from(bounds.origin.x),
        y: f32::from(bounds.origin.y),
        width: f32::from(bounds.size.width),
        height: f32::from(bounds.size.height),
    }
}
