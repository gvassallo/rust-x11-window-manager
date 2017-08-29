//! Event-related methods.

use std::mem::zeroed;
use std::os::raw::{c_int, c_uint, c_ushort};

use cplwm_api::types::{Geometry, WindowWithInfo};
use cplwm_api::wm::{FloatSupport, FullscreenSupport, MinimiseSupport, WindowManager};

use super::*;

use x11_dl::xlib;
use zombie;

/// Event-related methods.
impl<WM> X11Backend<WM>
    where WM: WindowManager + FloatSupport + FullscreenSupport + MinimiseSupport
{
    /// Run the main event loop.
    ///
    /// Calls [`handler`] for each event. When the handler modified the window
    /// manager, [`apply_window_layout`] is called to apply the changes.
    ///
    /// [`handler`]: struct.X11Backend.html#method.handler
    /// [`apply_window_layout`]: struct.X11Backend.html#method.apply_window_layout
    pub fn run(&mut self, config: X11Config<WM>) -> X11Result<()> {
        let mut event: xlib::XEvent = unsafe { zeroed() };
        loop {
            unsafe {
                (self.xlib.XNextEvent)(self.display, &mut event);
            }
            // Store the current event because we'll need it later in
            // focus_window.
            self.current_event = Some(event);
            // Remember the previous window layout
            let prev_window_layout = self.get_wm().get_window_layout();
            self.wm_modified = false;
            // Run the handler. When it returns an error, the window manager
            // stops. In general this is very undesirable for a window
            // manager, because when it stops, the user can't do much. This is
            // not so much a problem when you it in a nested X session, as we
            // mostly do. So instead of swallowing errors, we crash, which
            // leads to quicker discovery of bugs.
            try!(self.handler(&event, &config));
            // Only if the window manager was modified, i.e. a mutable borrow
            // occurred using `get_wm_mut`, do we have to apply the changes.
            if self.wm_modified {
                let new_window_layout = self.get_wm().get_window_layout();
                self.apply_window_layout(&prev_window_layout, &new_window_layout);
            }
        }
    }


    /// Main event handler.
    ///
    /// See the implementation for more information.
    pub fn handler(&mut self, event: &xlib::XEvent, config: &X11Config<WM>) -> X11Result<()> {
        debug!("Event: {}", event_name(event));
        match event.get_type() {
            // A key was pressed, look up the command bound to it and execute
            // it. Only for a key that was grabbed will this event be
            // generated.
            xlib::KeyPress => {
                let xev: xlib::XKeyEvent = From::from(event);
                let keysym: xlib::KeySym =
                    unsafe { (self.xlib.XKeycodeToKeysym)(self.display, xev.keycode as u8, 0) };
                let keymask: XKeyMask = xev.state;
                let key = Key {
                    mask: self.clean_mask(keymask),
                    sym: keysym,
                };
                trace!("{}", key);
                if let Some(command) = config.key_bindings.get(&key) {
                    try!(command(self));
                }
            }
            // A mouse button was clicked. If the root window was clicked and
            // a command is bound to the mouse button, execute it. Otherwise,
            // it is a click to focus another window.
            xlib::ButtonPress => {
                let xev: xlib::XButtonEvent = From::from(event);
                let keymask: XKeyMask = xev.state;
                let button = Button {
                    mask: self.clean_mask(keymask),
                    button: xev.button,
                };
                trace!("{} on {} {}", button, xev.window, xev.subwindow);
                match config.button_bindings.get(&button) {
                    Some(command) if self.root_window == xev.window => {
                        // If a command was bound to the button using
                        // grab_buttons, execute it.
                        try!(command(self, xev));
                    }
                    _ => {
                        // Otherwise, it's a click to focus another
                        // window, unless it's a click on the root window.
                        let to_focus = if self.root_window == xev.window {
                            if xev.subwindow == 0 {
                                None
                            } else {
                                Some(xev.subwindow)
                            }
                        } else {
                            Some(xev.window)
                        };
                        if self.get_wm().get_focused_window() != to_focus {
                            try!(self.get_wm_mut().focus_window(to_focus));
                        }
                    }
                }
            }
            // A mouse button was released, if we were dragging, stop it.
            xlib::ButtonRelease => {
                if let Some(_) = self.dragging.take() {
                    unsafe {
                        (self.xlib.XUngrabPointer)(self.display, xlib::CurrentTime);
                    }
                }
            }
            // The mouse was moved. This event will only occur when we're
            // dragging something, so execute the current dragging function.
            xlib::MotionNotify => {
                let xev: xlib::XMotionEvent = From::from(event);
                // Note the use of `take`: we remove the function from
                // `self.dragging` because the `while_dragging` function needs
                // a mutable reference to `self`, which would not be possible
                // if `while_dragging` were borrowed immutably. That's why we
                // remove it from `self` and restore it afterwards.
                if let Some(while_dragging) = self.dragging.take() {
                    let res = while_dragging(self, xev.x, xev.y);
                    // Ignore any events generate while executing the function
                    self.clear_events(xlib::PointerMotionMask);
                    try!(res);
                    // Restore the it
                    self.dragging = Some(while_dragging);
                }
            }
            // The mouse entered another window, focus it.
            xlib::EnterNotify => {
                let xev: xlib::XCrossingEvent = From::from(event);
                if xev.mode == xlib::NotifyNormal {
                    match self.get_wm().get_focused_window() {
                        // Do nothing if the window is already focused.
                        Some(w) if w == xev.window => trace!("Already focused"),
                        // If it is the root window, do nothing, i.e. leave
                        // the currently focused window focused.
                        _ if xev.window == self.root_window => trace!("Root, keep focus"),
                        // Otherwise, focus the window
                        _ => {
                            trace!("Focus window: {}", xev.window);
                            try!(self.get_wm_mut().focus_window(Some(xev.window)));
                        }
                    }
                }
            }
            // A new window wants to be managed.
            xlib::MapRequest => {
                let xev: xlib::XMapRequestEvent = From::from(event);
                let mut window_attrs = unsafe { zeroed() };
                unsafe {
                    (self.xlib.XGetWindowAttributes)(self.display, xev.window, &mut window_attrs)
                };
                // Only add the window if it isn't already and if it
                // didn't indicate that it should not be managed (e.g.
                // popups or fullscreen windows).
                if !self.get_wm().is_managed(xev.window) && window_attrs.override_redirect == 0 {
                    let mut geometry = Geometry {
                        x: window_attrs.x,
                        y: window_attrs.y,
                        width: window_attrs.width as c_uint,
                        height: window_attrs.height as c_uint,
                    };
                    if let Some(hints) = self.get_wm_normal_hints(xev.window) {
                        respect_hints(&mut geometry, &hints);
                    }
                    let screen = self.get_wm().get_screen();
                    center_geometry(&mut geometry, &screen);
                    let float_or_tile = self.wants_to_float_or_tile(xev.window);
                    let fullscreen = self.wants_to_be_fullscreen(xev.window);
                    self.add_window(xev.window);
                    try!(self.get_wm_mut()
                        .add_window(WindowWithInfo::new(xev.window,
                                                        geometry,
                                                        float_or_tile,
                                                        fullscreen)));
                }
            }
            // The keyboard mapping was changed, regrab the keys.
            xlib::MappingNotify => {
                let mut xev: xlib::XMappingEvent = From::from(event);
                unsafe { (self.xlib.XRefreshKeyboardMapping)(&mut xev) };
                if xev.request == xlib::MappingKeyboard || xev.request == xlib::MappingModifier {
                    self.set_numlock_mask();
                    self.grab_keys(&config.key_bindings);
                }
            }
            // When a window is destroyed, remove it from the window manager
            // if it is managed by it.
            xlib::DestroyNotify => {
                let xev: xlib::XDestroyWindowEvent = From::from(event);
                if self.get_wm().is_managed(xev.window) {
                    try!(self.get_wm_mut().remove_window(xev.window));
                    self.remove_window(xev.window);
                }
            }
            // A window is unmapped, i.e. removed from the window manager.
            // Hiding a window also generates this event, so ignore it when
            // the window manager caused it. When the window manager hides a
            // window, it is stored in the `hidden` field.
            xlib::UnmapNotify => {
                let xev: xlib::XUnmapEvent = From::from(event);
                if self.get_wm().is_managed(xev.window) {
                    // Only remove the window when we didn't hide it.
                    if !self.hidden.contains(&xev.window) {
                        try!(self.get_wm_mut().remove_window(xev.window));
                        self.remove_window(xev.window);
                    }
                }
                // Be a good parent and reap your zombie children. Children,
                // i.e. processes of windows spawned by the window manager
                // itself (for example an xterm spawned via a key binding),
                // become zombies when they terminate, e.g., after closing
                // their sole window, but their process is still lingering
                // around. See https://en.wikipedia.org/wiki/Zombie_process
                //
                // The alternative is to use
                // https://github.com/rust-lang/rust/pull/26470, but this is
                // not yet stable.
                zombie::collect_zombies();
            }
            // A request was made to reconfigure (resize, move, restack, ...)
            // a window. Comply if the window floats. If the window is tiled,
            // don't do anything, but respond with an event stating that we
            // have 'reconfigured' the window, but mention its current
            // configuration.
            xlib::ConfigureRequest => {
                let xev: xlib::XConfigureRequestEvent = From::from(event);
                // We can't configure a window we don't manage.
                if !self.get_wm().is_managed(xev.window) {
                    return Ok(());
                }
                let geometry = try!(self.get_window_geometry(xev.window));
                if self.get_wm().is_floating(xev.window) {
                    let mask = xev.value_mask as c_ushort;
                    let new_geometry = Geometry {
                        x: if mask & xlib::CWX != 0 {
                            xev.x
                        } else {
                            geometry.x
                        },
                        y: if mask & xlib::CWY != 0 {
                            xev.y
                        } else {
                            geometry.y
                        },
                        // We have to add the border width here, because it
                        // gets subtracted in set_window_geometry. If we don't
                        // do this, some windows will keep sending these
                        // requests and slowly shrink.
                        width: if mask & xlib::CWWidth != 0 {
                            xev.width as c_uint + 2 * WINDOW_BORDER_WIDTH
                        } else {
                            geometry.width
                        },
                        height: if mask & xlib::CWHeight != 0 {
                            xev.height as c_uint + 2 * WINDOW_BORDER_WIDTH
                        } else {
                            geometry.height
                        },
                    };
                    try!(self.get_wm_mut().set_window_geometry(xev.window, new_geometry));
                } else {
                    // Just send the event
                    let mut event: xlib::XEvent = xlib::XConfigureEvent {
                            type_: xlib::ConfigureNotify,
                            serial: xev.serial,
                            send_event: xlib::True,
                            display: self.display,
                            event: xev.window,
                            window: xev.window,
                            x: geometry.x,
                            y: geometry.y,
                            width: (geometry.width - 2 * WINDOW_BORDER_WIDTH) as c_int,
                            height: (geometry.height - 2 * WINDOW_BORDER_WIDTH) as c_int,
                            border_width: WINDOW_BORDER_WIDTH as c_int,
                            above: 0,
                            override_redirect: xlib::False,
                        }
                        .into();
                    unsafe {
                        (self.xlib.XSendEvent)(self.display,
                                               self.root_window,
                                               xlib::False,
                                               xlib::StructureNotifyMask,
                                               &mut event);
                    }
                }
                unsafe { (self.xlib.XSync)(self.display, xlib::False) };
            }
            // When the root window is 'reconfigured', the display settings
            // have changed.
            xlib::ConfigureNotify => {
                let xev: xlib::XConfigureEvent = From::from(event);
                if xev.window == self.root_window {
                    let screen = self.get_screen();
                    // Update the window manager with the changed screen.
                    self.get_wm_mut().resize_screen(screen);
                }
            }
            // Messages sent by client, i.e. applications
            xlib::ClientMessage => {
                let xev: xlib::XClientMessageEvent = From::from(event);
                // Delegate to the EWMH handler
                try!(self.handle_ewmh_client_message(xev));
            }
            _ => (),
        }
        Ok(())
    }


    /// Clear all events matching the mask from the event queue.
    ///
    /// Uses [`XCheckMaskEvent`].
    ///
    /// [`XCheckMaskEvent`]:
    /// https://tronche.com/gui/x/xlib/event-handling/manipulating-event-queue/XCheckMaskEvent.html
    pub fn clear_events(&self, mask: XEventMask) {
        unsafe {
            (self.xlib.XSync)(self.display, xlib::False);
            let mut xev: xlib::XEvent = zeroed();
            while (self.xlib.XCheckMaskEvent)(self.display, mask, &mut xev) != 0 {
            }
        }
    }
}
