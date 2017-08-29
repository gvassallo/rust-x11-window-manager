//! The backend
//!
//! This is an X11 backend for the window manager.
//!
//! See the [runner] for how to run the window manager.
//!
//! [runner]: ../cplwm_runner/index.html
//!
//! To successfully complete the assignment, you don't have to understand any
//! of this or even look at this code.
//!
//! However, if you are interested in this code, feel free to have a look
//! around. The documentation of the code together with the [Xlib manual]
//! should be enough to understand most code. If not, you can always ask
//! questions on the discussion board on Toledo.
//!
//! The reason that almost all functionality is defined as methods of the
//! [`X11Backend`] is because nearly all functions need access to the Xlib
//! library or access to the window manager's state or the backend's state,
//! which is stored in the [`X11Backend`].
//!
//! One non-obvious trick employed in this project is the following: in Rust,
//! methods can be added via `impl` blocks, however, they must be in the same
//! module as the type to which you are adding methods. If we had to put all
//! methods of the [`X11Backend`] in this file, this would be a pretty huge
//! file (> 2000 SLOC) and it is already large enough (~ 600 SLOC). The trick
//! is that methods can also be defined in *submodules* of the module defining
//! the type. So the methods have been grouped and spread over different
//! submodules (the `methods` submodule is still a dump of various methods).
//! Rust's module system is expressive enough (note the `pub use`s in this
//! file) to hide this. If you are viewing the documentation of this module,
//! it will appear as one module.
//!
//! You will see a lot of unsafe code, that is because we are interoperating
//! with the Xlib library (via the [`x11_dl`] crate), which is written in C.
//! To be as explicit as possible, the smallest unit of unsafe code is always
//! put in an unsafe block, instead of just marking the whole function as
//! unsafe. You will notice some calls to `XFree` (Xlib's variant of `free`),
//! that is also because we are interoperating with the Xlib library, and Rust
//! doesn't magically make existing C code memory-safe. It could well be
//! possible that I forgot one and that there is a memory leak!
//!
//! If your memory is good, you might remember that the project assignment
//! said that there would be a `Backend` trait. This is no longer the case,
//! the design has evolved.
//!
//! [Xlib manual]: https://tronche.com/gui/x/xlib/
//! [`X11Backend`]: struct.X11Backend.html
//! [`x11_dl`]: https://crates.io/crates/x11-dl

#![deny(missing_docs)]

extern crate cplwm_api;
extern crate exec;
#[macro_use]
extern crate lazy_static;
extern crate libc;
#[macro_use]
extern crate log;
extern crate rustc_serialize;
extern crate x11_dl;
extern crate zombie;

mod event;
mod ewmh;
mod input;
mod macros;
mod methods;
mod mouse;
mod types;
mod util;

pub use self::event::*;
pub use self::ewmh::*;
pub use self::input::*;
pub use self::macros::*;
pub use self::methods::*;
pub use self::mouse::*;
pub use self::types::*;
pub use self::util::*;

use std::collections::HashSet;
use std::os::raw::{c_int, c_long, c_uint};
use std::ptr::{null, null_mut};

use cplwm_api::wm::{FloatSupport, FullscreenSupport, MinimiseSupport, WindowManager};
use cplwm_api::types::{Geometry, Screen, Window, WindowLayout, WindowWithInfo};

use x11_dl::xlib;

/// The border width of windows.
pub const WINDOW_BORDER_WIDTH: c_uint = 1;

/// The event mask for the root window.
///
/// This controls which general X events the event loop will receive.
const ROOT_MASK: XEventMask =
    xlib::SubstructureRedirectMask | xlib::SubstructureNotifyMask | xlib::EnterWindowMask |
    xlib::LeaveWindowMask | xlib::StructureNotifyMask | xlib::ButtonPressMask;

/// The event mask for client windows.
///
/// This controls which client window events the event loop will receive.
const CLIENT_MASK: XEventMask = xlib::StructureNotifyMask | xlib::EnterWindowMask;

/// The X11 Backend.
///
/// This struct contains all the state needed by the backend.
pub struct X11Backend<WM> {
    /// A reference to the Xlib library.
    ///
    /// We need this to call Xlib functions.
    xlib: xlib::Xlib,
    /// Cached display.
    ///
    /// We need this for many Xlib calls.
    display: *mut xlib::Display,
    /// Cached root window.
    ///
    /// The root window is the background window that you see.
    root_window: Window,
    /// Indicate whether the window manager was modified. Only if it is `true`
    /// should `apply_window_layout` be called.
    wm_modified: bool,
    /// The window manager.
    wm: WM,
    /// Keep track of the current event, we need this in `focus_window`.
    current_event: Option<xlib::XEvent>,
    /// The numlock modifier mask.
    numlock_mask: XKeyMask,
    /// The function to execute while dragging.
    ///
    /// For example the function to execute while dragging could be a function
    /// that moves the window to the right position.
    dragging: Option<Box<WhileDragging<WM>>>,
    /// The hidden windows. We need this to handle `UnmapNotify` events in
    /// `handler`.
    hidden: HashSet<Window>,
    /// A `Vec` of all the managed windows order from old to new.
    ///
    /// The order of the windows is as follows: the oldest window (first
    /// mapped) is first, the youngest (most recently mapped) window comes
    /// last.
    ///
    /// This `Vec` is needed for `set_client_list`. We can't use the
    /// `get_windows` method of the window manager, because the returned `Vec`
    /// won't have the right order.
    managed: Vec<Window>,
    /// Cached focused border color pixel.
    focused_border_color: xlib::XColor,
    /// Cached unfocused border color pixel.
    unfocused_border_color: xlib::XColor,
}

/// Access to the window manager.
impl<WM> X11Backend<WM> {
    /// Return an immutable borrow to the window manager.
    pub fn get_wm(&self) -> &WM {
        &self.wm
    }

    /// Return a mutable borrow to the window manager.
    ///
    /// Records that the window manager is modified.
    pub fn get_wm_mut(&mut self) -> &mut WM {
        self.wm_modified = true;
        &mut self.wm
    }
}


impl<WM> Drop for X11Backend<WM> {
    /// Close the connection to the display when the backend is stopped.
    fn drop(&mut self) {
        unsafe {
            (self.xlib.XCloseDisplay)(self.display);
        }
    }
}

/// Basic functionality
impl<WM> X11Backend<WM>
    where WM: WindowManager + FloatSupport + FullscreenSupport + MinimiseSupport
{
    /// Start the window manager.
    ///
    /// The `make_wm` argument is a function that creates a window manager
    /// based on a `Screen`. Use the static [`new`] method (constructor) of
    /// the `WindowManager` you want to use.
    ///
    /// This function will only return when the window manager stops, i.e.
    /// when it returns an error, or when it is terminated by the user.
    ///
    /// [`new`]: ../cplwm_api/wm/trait.WindowManager.html#tymethod.new
    pub fn start<F>(make_wm: F, config: X11Config<WM>) -> X11Result<()>
        where F: Fn(Screen) -> WM
    {
        let mut backend = Self::new(make_wm, &config);
        try!(backend.init(&config));
        backend.run(config)
    }

    /// Create a `X11Backend`.
    ///
    /// A connection to the X server is made, the window manager is created,
    /// and the initial state is created.
    fn new<F>(make_wm: F, config: &X11Config<WM>) -> X11Backend<WM>
        where F: Fn(Screen) -> WM
    {
        // Open Xlib library
        let xlib = xlib::Xlib::open().expect("Xlib library unavailable");

        let display = unsafe { (xlib.XOpenDisplay)(null()) };
        if display == null_mut() {
            panic!("Can't open display");
        }
        let screen_number = unsafe { (xlib.XDefaultScreen)(display) };
        let root_window = unsafe { (xlib.XRootWindow)(display, screen_number) };

        // It would be nicer if we could use get_screen here, but we don't
        // have an `X11Backend` yet.
        let xscreen = unsafe { (xlib.XDefaultScreenOfDisplay)(display) };
        let screen = Screen {
            width: unsafe { (*xscreen).width } as c_uint,
            height: unsafe { (*xscreen).height } as c_uint,
        };

        let colormap = unsafe { (xlib.XDefaultColormap)(display, screen_number) };
        // We unwrap here, so we crash when the color was invalid, but that's
        // okay.
        let focused_border_color =
            allocate_color(display, &xlib, config.focused_border_color, colormap).unwrap();
        let unfocused_border_color =
            allocate_color(display, &xlib, config.unfocused_border_color, colormap).unwrap();

        X11Backend {
            xlib: xlib,
            display: display,
            root_window: root_window,
            wm_modified: false,
            wm: make_wm(screen),
            current_event: None,
            numlock_mask: 0,
            dragging: None,
            hidden: HashSet::new(),
            managed: Vec::new(),
            focused_border_color: focused_border_color,
            unfocused_border_color: unfocused_border_color,
        }
    }

    /// Initialise the backend.
    ///
    /// The following things are done:
    ///
    /// * Try to replace a running window manager. Not every window manager
    ///   listens to this. When it doesn't we crash.
    /// * We try to restore the previous state in case of a restart (or
    ///   crash).
    /// * Set the background.
    /// * Initialise the keys and mouse buttons
    /// * ...
    fn init(&mut self, config: &X11Config<WM>) -> X11Result<()> {
        // Ty to replace another WM that might be running.
        self.replace_other_wm();

        // In case of a restart, try restoring the previous serialised
        // state of the WM.
        self.restore_state();

        // In case the WM has been shut down and restarted, remove all
        // windows managed by the WM that are no longer visible.
        let visible_windows = self.find_visible_windows();
        for managed_window in self.get_wm().get_windows() {
            if !visible_windows.contains(&managed_window) {
                try!(self.get_wm_mut().remove_window(managed_window));
            }
        }

        // Add all visible windows that are already present or, in case
        // the WM has been shut down and restarted, add all windows that
        // have been added since the shutdown.
        for visible_window in visible_windows {
            // Make sure we grabbed the input and events
            self.add_window(visible_window);

            if !self.get_wm().is_managed(visible_window) {
                let geometry = try!(self.get_window_geometry(visible_window));
                let float_or_tile = self.wants_to_float_or_tile(visible_window);
                let fullscreen = self.wants_to_be_fullscreen(visible_window);
                try!(self.get_wm_mut()
                    .add_window(WindowWithInfo::new(visible_window,
                                                    geometry,
                                                    float_or_tile,
                                                    fullscreen)));
            }
        }

        // Indicate that we are the running WM, this will fail when
        // another WM is still running.
        unsafe {
            (self.xlib.XSelectInput)(self.display, self.root_window, ROOT_MASK);
            (self.xlib.XSync)(self.display, xlib::False);
        }

        // Set the background color (of the root window).
        try!(self.set_background(config.background_color));

        self.set_numlock_mask();
        self.grab_keys(&config.key_bindings);
        self.grab_buttons(&config.button_bindings);

        // EWMH support
        self.set_net_supported(SUPPORTED_ATOM_NAMES.iter().map(|name| *name));

        // Apply the layout when the state was restored. Windows could have
        // moved in the meantime.
        if self.wm_modified {
            let empty_layout = WindowLayout::new();
            let restored_layout = self.get_wm().get_window_layout();
            self.apply_window_layout(&empty_layout, &restored_layout);
        }

        Ok(())
    }

    /// Update the X server so that the new window layout is reflected.
    ///
    /// The new window layout is compared with the old one. Windows that are
    /// no longer visible are hidden, new ones are revealed, the focus coud be
    /// updated, windows could be restacked, windows could be resized/moved.
    pub fn apply_window_layout(&mut self,
                               prev_window_layout: &WindowLayout,
                               new_window_layout: &WindowLayout) {
        trace!("apply_window_layout");

        let prev_windows: Vec<_> = prev_window_layout.windows.iter().map(|&(w, _)| w).collect();
        let new_windows: Vec<_> = new_window_layout.windows.iter().map(|&(w, _)| w).collect();

        // Determine which windows to reveal and which to hide
        let mut prev_window_set = HashSet::with_capacity(prev_windows.len());
        prev_window_set.extend(&prev_windows);
        let mut new_window_set = HashSet::with_capacity(new_windows.len());
        new_window_set.extend(&new_windows);

        for removed_window in prev_window_set.difference(&new_window_set) {
            self.hide_window(*removed_window);
        }
        for added_window in new_window_set.difference(&prev_window_set) {
            self.reveal_window(*added_window);
        }

        // Change the focus
        match (prev_window_layout.focused_window, new_window_layout.focused_window) {
            (Some(w1), Some(w2)) if w1 != w2 => {
                // A different window is focused
                self.unfocus_window(w1);
                self.focus_window(w2);
            }
            (Some(w), None) => self.unfocus_window(w),
            (None, Some(w)) => self.focus_window(w),
            // Focus is unchanged
            _ => (),
        }

        // Update the stack order. Dumb: also restacks when windows were only
        // added and/or removed.
        if prev_windows != new_windows {
            self.restack(new_windows.iter().map(|w| *w));
        }

        // Update the geometries: for every window in the new layout, look up
        // its geometry in the old layout. When the lookup fails or when the
        // geometry differs from the new one, update the geometry.
        for &(window, geometry) in &new_window_layout.windows {
            match prev_window_layout.windows.iter().find(|&&(w, _)| w == window) {
                // Same geometry -> do nothing
                Some(&(_, prev_geometry)) if prev_geometry == geometry => (),
                // Different geometry or no geometry -> set it
                _ => self.set_window_geometry(window, geometry),
            }
        }

        // Ignore any enter/leave events we may have generated while applying
        // the window layout.
        self.clear_events(xlib::EnterWindowMask | xlib::LeaveWindowMask);
    }

    /// Add a new window to the backend.
    ///
    /// Do not confuse this with the [`add_window`] method of the window
    /// manager. This method does the X related initialisation of the window,
    /// e.g. changes the border width and color, and other low-level details.
    ///
    ///
    /// [`add_window`]: ../cplwm_api/wm/trait.WindowManager.html#tymethod.add_window
    pub fn add_window(&mut self, window: Window) {
        trace!("add_window: {} \"{}\"",
               window,
               self.get_window_title(window).unwrap_or("(no title)".to_owned()));
        // Start listening for some of the window's events
        unsafe {
            (self.xlib.XSelectInput)(self.display, window, CLIENT_MASK);
        }
        self.set_button_grab(true, window, xlib::AnyButton as XButton, xlib::AnyModifier);
        self.set_wm_state(window, WindowState::Iconic);
        if !self.managed.contains(&window) {
            self.managed.push(window);
        }
        self.set_client_list(self.managed.iter());
        self.set_allowed_actions(window, ALLOWED_ACTIONS_ATOM_NAMES.iter().map(|name| *name));
        self.set_window_border_width(window, WINDOW_BORDER_WIDTH);
        self.set_window_border_color(window, self.unfocused_border_color);
    }

    /// Remove a window from the backend.
    ///
    /// Do not confuse this with the [`remove_window`] method of the window
    /// manager. This method does some bookkeeping.
    ///
    ///
    /// [`remove_window`]: ../cplwm_api/wm/trait.WindowManager.html#tymethod.remove_window
    pub fn remove_window(&mut self, window: Window) {
        trace!("remove_window: {}", window);
        // No need to actually call XUnmapWindow, as `hide_window` should
        // already be called on the window.

        // Remove the window from self.managed
        if let Some(i) = self.managed.iter().position(|w| *w == window) {
            self.managed.remove(i);
            self.set_client_list(self.managed.iter());
        }
    }

    /// Ask the X server to reveal a window.
    ///
    /// The window will become visible (mapped).
    pub fn reveal_window(&mut self, window: Window) {
        trace!("reveal_window: {}", window);
        unsafe {
            (self.xlib.XMapWindow)(self.display, window);
        }
        self.set_wm_state(window, WindowState::Normal);
    }

    /// Ask the X server to hide a window.
    ///
    /// The window will become invisible (unmapped).
    pub fn hide_window(&mut self, window: Window) {
        trace!("hide_window: {}", window);
        if self.get_wm().is_managed(window) {
            // Remember that we hid the window so when the UnmapNotify event
            // arrives, we can ignore it.
            self.hidden.insert(window);
            unsafe {
                (self.xlib.XUnmapWindow)(self.display, window);
            }
            self.set_wm_state(window, WindowState::Iconic);
        }
    }


    /// Ask the X server to focus a window.
    ///
    ///
    /// Based on:
    ///
    /// * https://tronche.com/gui/x/xlib/input/XSetInputFocus.html
    /// * XMonad
    /// * http://lists.suckless.org/dev/1104/7548.html
    /// * https://tronche.com/gui/x/icccm/sec-4.html#s-4.1.7
    /// * https://tronche.com/gui/x/xlib/ICC/client-to-window-manager/wm-hints.html
    pub fn focus_window(&mut self, window: Window) {
        trace!("focus_window: {}", window);
        // Ungrab the buttons on the window to focus, unless it's the root
        // window. This means we no longer get the mouse button events
        // generated by the window, they now go directly to the application of
        // the window itself.
        if window != self.root_window {
            self.set_button_grab(false, window, xlib::AnyButton as XButton, xlib::AnyModifier);
        }
        unsafe {
            (self.xlib.XSetInputFocus)(self.display,
                                       window,
                                       xlib::RevertToPointerRoot,
                                       xlib::CurrentTime);
        }

        let protocols = self.get_wm_protocols(window);
        let wm_take_focus = self.get_atom("WM_TAKE_FOCUS");

        // Try the WM_TAKE_FOCUS-based protocol
        if protocols.as_slice().contains(&wm_take_focus) {
            // If the current event caused the focus change, we must the
            // current event's timestamp instead of xlib::CurrentTime.
            let time = self.current_event
                .as_ref()
                .and_then(util::get_timed_event_time)
                .unwrap_or(xlib::CurrentTime);
            let mut data = xlib::ClientMessageData::new();
            data.set_long(0, wm_take_focus as c_long);
            data.set_long(1, time as c_long);
            let wm_protocols = self.get_atom("WM_PROTOCOLS");
            let mut xev: xlib::XEvent = xlib::XClientMessageEvent {
                    type_: xlib::ClientMessage,
                    serial: 0,
                    send_event: xlib::True,
                    display: self.display,
                    window: window,
                    message_type: wm_protocols,
                    format: 32,
                    data: data,
                }
                .into();
            unsafe {
                (self.xlib.XSendEvent)(self.display,
                                       window,
                                       xlib::False,
                                       xlib::NoEventMask,
                                       (&mut xev as *mut xlib::XEvent));
            }
        }

        self.set_window_border_color(window, self.focused_border_color);

        // Advertise via EWMH that the window is focused
        self.set_active_window(Some(window));
    }

    /// Unfocus a window.
    pub fn unfocus_window(&mut self, window: Window) {
        trace!("unfocus_window {}", window);

        // Setting the border color or ungrabbing on an unmapped window causes
        // an error.
        if self.managed.contains(&window) {
            self.set_window_border_color(window, self.unfocused_border_color);
            self.set_button_grab(true, window, xlib::AnyButton as XButton, xlib::AnyModifier);
        }

        // Advertise via EWMH that no window is focused
        self.set_active_window(None);
    }

    /// Ask the X server to restack the windows.
    ///
    /// The first element in the iterator is the bottom window, the last is
    /// the top window.
    pub fn restack<I: Iterator<Item = Window>>(&mut self, new_stack_order: I) {
        let mut window_vec = new_stack_order.collect::<Vec<Window>>();

        // Advertise via EWMH
        self.set_client_list_stacking(window_vec.iter());

        // XRestackWindows expects the top window at the beginning of the list.
        window_vec.reverse();
        let nwindows = window_vec.len();
        let windows = window_vec.as_mut_ptr();

        unsafe {
            (self.xlib.XRestackWindows)(self.display, windows, nwindows as c_int);
        }
    }

    /// Get the actual `Geometry` of a window according to the X server.
    ///
    /// Return an `Err` when the X server doesn't know the window.
    pub fn get_window_geometry(&self, window: Window) -> X11Result<Geometry> {
        let mut root = 0;
        let mut x = 0;
        let mut y = 0;
        let mut width = 0;
        let mut height = 0;
        let mut border_width = 0;
        let mut depth = 0;
        let status = unsafe {
            (self.xlib.XGetGeometry)(self.display,
                                     window,
                                     &mut root,
                                     &mut x,
                                     &mut y,
                                     &mut width,
                                     &mut height,
                                     &mut border_width,
                                     &mut depth)
        };
        if status != 0 {
            let geometry = Geometry {
                x: x,
                y: y,
                width: width,
                height: height,
            };
            trace!("get_window_geometry: {} {}", window, geometry);
            Ok(geometry)
        } else {
            error!("get_window_geometry: unknown window {}", window);
            Err(X11Error::UnknownWindow(window))
        }

    }

    /// Ask the X server to resize/move the window so it matches the given
    /// `Geometry`.
    pub fn set_window_geometry(&mut self, window: Window, new_geometry: Geometry) {
        trace!("set_window_geometry: {} {}", window, new_geometry);
        // Ignore invalid geometries
        if !valid_geometry(&new_geometry) {
            return;
        }
        let Geometry { x, y, width, height } = new_geometry;
        let mut changes = xlib::XWindowChanges {
            x: x,
            y: y,
            width: (width - 2 * WINDOW_BORDER_WIDTH) as c_int,
            height: (height - 2 * WINDOW_BORDER_WIDTH) as c_int,
            border_width: WINDOW_BORDER_WIDTH as c_int,
            sibling: 0,
            stack_mode: 0,
        };
        let mask = xlib::CWX | xlib::CWY | xlib::CWWidth | xlib::CWHeight | xlib::CWBorderWidth;
        unsafe {
            (self.xlib.XConfigureWindow)(self.display, window, mask as u32, &mut changes);
        }
    }
}
