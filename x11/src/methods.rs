//! More backend methods.

use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::From;
use std::env;
use std::ffi::{CStr, CString};
use std::fs;
use std::mem::{transmute, zeroed};
use std::os::raw::{c_int, c_long, c_uchar, c_uint, c_ulong};
use std::slice;
use std::sync::Mutex;

use cplwm_api::types::{FloatOrTile, Screen, Window};
use cplwm_api::wm::WindowManager;

use super::*;

use exec::execvp;
use libc::wchar_t;
use x11_dl::xlib;

lazy_static! {
    /// Private static cache from atom name to atom.
    static ref ATOM_CACHE: Mutex<HashMap<Cow<'static, str>, xlib::Atom>>
        = Mutex::new(HashMap::new());
}

/// More backend methods.
impl<WM: WindowManager> X11Backend<WM> {
    /// Return the X11 atom with the given name.
    ///
    /// Calls are memoised to minimise communication with the X server.
    ///
    /// The type `T: Into<Cow<'static, str>>>` is a generic way of saying that
    /// this method accepts an argument of type `&'static str` as well as
    /// `String`.
    pub fn get_atom<T: Into<Cow<'static, str>>>(&self, t: T) -> xlib::Atom {
        let atom_name_cow: Cow<'static, str> = t.into();
        // When the atom is already in the cache, return it, otherwise intern
        // it (via the X server) and insert it.
        //
        // Unwrapping is safe as we are single-threaded.
        *ATOM_CACHE.lock().unwrap().entry(atom_name_cow.clone()).or_insert_with(|| {
            let atom_name: String = atom_name_cow.into_owned();
            unsafe {
                (self.xlib.XInternAtom)(self.display,
                                        CString::new(atom_name).unwrap().as_ptr(),
                                        xlib::False)
            }
        })
    }

    /// Get the 32-bit items associated with the window's property.
    ///
    /// See [`XGetWindowProperty`].
    ///
    /// [`XGetWindowProperty`]:
    /// https://tronche.com/gui/x/xlib/window-information/XGetWindowProperty.html
    pub fn get_window_property32(&self,
                                 window: Window,
                                 property: xlib::Atom)
                                 -> Option<Vec<c_int>> {
        let mut actual_type_return = 0;
        let mut actual_format_return = 0;
        let mut nitems_return = 0;
        let mut bytes_after_return = 0;
        let mut prop_return: *mut c_uchar = unsafe { zeroed() };
        let status = unsafe {
            (self.xlib
                .XGetWindowProperty)(self.display,
                                     window,
                                     property,
                                     0,
                                     0xFFFFFFFF,
                                     xlib::False,
                                     xlib::AnyPropertyType as c_ulong,
                                     &mut actual_type_return,
                                     &mut actual_format_return,
                                     &mut nitems_return,
                                     &mut bytes_after_return,
                                     &mut prop_return)
        };
        // Call failed or the specified property does not exist for the
        // specified window.
        if status != 0 || actual_type_return == 0 {
            return None;
        }
        // The specified property exists but the property format does not
        // match the requested one
        if actual_format_return != 32 {
            unsafe {
                (self.xlib.XFree)(transmute(prop_return));
            }
            return None;
        }
        let prop_return32: *mut c_int = unsafe { transmute(prop_return) };
        let props = (unsafe { slice::from_raw_parts(prop_return32, nitems_return as usize) })
            .to_vec();
        unsafe {
            (self.xlib.XFree)(transmute(prop_return));
        }
        trace!("get_window_property32: {} {} {:?}", window, property, props);
        Some(props)
    }

    /// Change the 32-bit items associated with the window's property.
    ///
    /// Use [`PropModeReplace`], [`PropModeAppend`], or [`PropModePrepend`] as
    /// `property_type`. See [`XChangeProperty`] for more information.
    ///
    /// [`XChangeProperty`]:
    /// https://tronche.com/gui/x/xlib/window-information/XChangeProperty.html
    /// [`PropModeReplace`]: ../x11_dl/xlib/constant.PropModeReplace.html
    /// [`PropModeAppend`]: ../x11_dl/xlib/constant.PropModeAppend.html
    /// [`PropModePrepend`]: ../x11_dl/xlib/constant.PropModePrepend.html
    pub fn change_window_property32<Props>(&self,
                                           window: Window,
                                           property: xlib::Atom,
                                           property_type: xlib::Atom,
                                           mode: c_int,
                                           props: Props)
        where Props: Iterator<Item = c_int>
    {

        let prop_vec: Vec<c_int> = props.collect::<Vec<c_int>>();
        let nelements = prop_vec.len();
        let data: *const c_uchar = unsafe { transmute(prop_vec.as_slice().as_ptr()) };
        unsafe {
            (self.xlib.XChangeProperty)(self.display,
                                        window,
                                        property,
                                        property_type,
                                        32,
                                        mode,
                                        data,
                                        nelements as c_int);
        }
    }

    /// Get the [`WM_STATE`] property of the given window.
    ///
    /// Return `None`, when it could not be retrieved.
    ///
    /// [`WM_STATE`]: https://tronche.com/gui/x/icccm/sec-4.html#WM_STATE
    pub fn get_wm_state(&self, window: Window) -> Option<WindowState> {
        let wm_state_atom = self.get_atom("WM_STATE");
        self.get_window_property32(window, wm_state_atom)
            .and_then(|props| props.first().cloned())
            .and_then(|prop| WindowState::try_from(prop as c_uint))
    }

    /// Set the [`WM_STATE`] property of the given window.
    ///
    /// [`WM_STATE`]: https://tronche.com/gui/x/icccm/sec-4.html#WM_STATE
    pub fn set_wm_state(&self, window: Window, window_state: WindowState) {
        let wm_state_atom = self.get_atom("WM_STATE");
        // The first element is the state, the second is the window id of the
        // icon window. When no such window exists, as is the case here, it
        // should be `None` (0). See section 4.1.3.1 in
        // https://tronche.com/gui/x/icccm/sec-4.html
        let props = [From::from(window_state), 0];
        self.change_window_property32(window,
                                      wm_state_atom,
                                      wm_state_atom,
                                      xlib::PropModeReplace,
                                      props.iter().map(|p| *p));
    }

    /// Signal another running window manager to exit.
    ///
    /// There can only be one running window manager. This method tries to
    /// signal any running window manager to stop so we can take over. Note
    /// that not every other window manager supports this, this will result in
    /// a crash.
    ///
    /// Based on [`JWM`].
    ///
    /// [`JWM`]: https://joewing.net/projects/jwm/
    pub fn replace_other_wm(&mut self) {

        // Create a supporting window used to verify if we're running
        let supporting_window = unsafe {
            (self.xlib
                .XCreateSimpleWindow)(self.display, self.root_window, 0, 0, 1, 1, 0, 0, 0)
        };

        // Get the screen number
        let screen_number = unsafe { (self.xlib.XDefaultScreen)(self.display) };

        // Get the atom used for the window manager selection.
        let manager_selection = self.get_atom(format!("WM_S{}", screen_number));

        // Get the current window manager and take the selection.
        unsafe {
            (self.xlib.XGrabServer)(self.display);
            (self.xlib.XSync)(self.display, xlib::False);
        }
        let win = unsafe { (self.xlib.XGetSelectionOwner)(self.display, manager_selection) };
        unsafe {
            if win != 0 {
                info!("Screen {} already has a window manager", screen_number);
                (self.xlib.XSelectInput)(self.display, win, xlib::StructureNotifyMask);
            }
            (self.xlib.XSetSelectionOwner)(self.display,
                                           manager_selection,
                                           supporting_window,
                                           xlib::CurrentTime);
            (self.xlib.XUngrabServer)(self.display);
        }

        // Wait for the current selection owner to give up the selection.
        if win != 0 {
            // Note that we need to wait for the current selection owner to
            // exit before we can expect to select SubstructureRedirectMask.
            let mut event: xlib::XEvent = unsafe { zeroed() };
            loop {
                unsafe {
                    (self.xlib.XWindowEvent)(self.display,
                                             win,
                                             xlib::StructureNotifyMask,
                                             &mut event);
                }
                if event.get_type() == xlib::DestroyNotify {
                    let xev: xlib::XDestroyWindowEvent = From::from(event);
                    if xev.window == win {
                        break;
                    }
                }
            }
            unsafe {
                (self.xlib.XSync)(self.display, xlib::False);
            }
        }
        let mut data = xlib::ClientMessageData::new();
        data.set_long(0, xlib::CurrentTime as c_long);
        data.set_long(1, manager_selection as c_long);
        data.set_long(2, supporting_window as c_long);
        data.set_long(3, 2);
        data.set_long(4, 0);
        let mut event: xlib::XEvent = xlib::XClientMessageEvent {
                type_: xlib::ClientMessage,
                serial: 0,
                send_event: xlib::False,
                display: self.display,
                window: self.root_window,
                message_type: self.get_atom("MANAGER"),
                format: 32,
                data: data,
            }
            .into();
        unsafe {
            (self.xlib.XSendEvent)(self.display,
                                   self.root_window,
                                   xlib::False,
                                   xlib::StructureNotifyMask,
                                   &mut event);
            (self.xlib.XSync)(self.display, xlib::False);
        }
    }

    /// Find all existing visible windows.
    ///
    /// This method is used to add existing visible windows to the window
    /// manager when it just started.
    pub fn find_visible_windows(&mut self) -> Vec<Window> {
        let mut root_return = 0;
        let mut parent_return = 0;
        let mut children_return_ptr = unsafe { zeroed() };
        let mut nchildren_return = 0;
        let status = unsafe {
            (self.xlib.XQueryTree)(self.display,
                                   self.root_window,
                                   &mut root_return,
                                   &mut parent_return,
                                   &mut children_return_ptr,
                                   &mut nchildren_return)
        };
        if status == 0 {
            error!("XQueryTree failed");
            // Pretend there were no windows
            return Vec::new();
        }
        let windows =
            unsafe { slice::from_raw_parts(children_return_ptr, nchildren_return as usize) };
        // Function that determines when a window is visible
        let visible = |window: &Window| {
            let mut window_attrs = unsafe { zeroed() };
            let wa_status = unsafe {
                (self.xlib.XGetWindowAttributes)(self.display, *window, &mut window_attrs)
            };
            let name = self.get_window_title(*window).unwrap_or(window.to_string());
            // Windows with override_redirect should not be managed by the WM,
            // so don't consider them visible
            if wa_status == 0 || window_attrs.override_redirect == 1 {
                trace!("Invisible window: {}", name);
                return false;
            }
            if window_attrs.map_state == xlib::IsViewable {
                true
            } else {
                trace!("Invisible window: {}", name);
                false
            }
        };
        let visible_windows: Vec<Window> = windows.iter().map(|w| *w).filter(visible).collect();
        unsafe {
            (self.xlib.XFree)(transmute(children_return_ptr));
        }
        debug!("find_existing_windows: found {} window(s) of which {} visible",
               windows.len(),
               visible_windows.len());
        visible_windows
    }

    /// Restore the window manager state from the state file.
    ///
    /// When the state could not be read or deserialised, nothing is done.
    pub fn restore_state(&mut self) {
        match deserialise_data_from_json_file(get_state_file_path()) {
            Err(err) => {
                // If restoring the state fails, log it, and start from scratch
                warn!("restore_state failed: {:?}", err);
                return;
            }
            Ok(previous_state) => {
                trace!("restore_state succeeded");
                self.wm = previous_state;
                self.wm_modified = true;
            }
        }
    }

    /// Restart the (whole) window manager.
    ///
    /// When `restore_state` is true, the current state of the window manager
    /// will be saved and reloaded.
    ///
    /// When saving the state failed, the restart is aborted. When the window
    /// manager's executable could not be found, the restart is also aborted.
    ///
    /// Use this when you recompiled your code and want to restart without
    /// closing all windows.
    pub fn restart(&self, restore_state: bool) {
        unsafe {
            (self.xlib.XFlush)(self.display);
        }

        if restore_state {
            match serialise_data_to_json_file(get_state_file_path(), &self.wm) {
                Err(err) => {
                    // If it fails, log it, and don't restart
                    error!("Encoding the state failed: {:?}", err);
                    return;
                }
                _ => (),
            }
        } else {
            // Remove any stale state file
            match fs::remove_file(get_state_file_path()) {
                Err(err) => {
                    // If it fails, log it, but restart anyway
                    error!("Removing the previous state failed: {}", err);
                }
                _ => (),
            }
        }

        // Restart the process
        match get_executable() {
            Err(err) => {
                // If the executable couldn't be found, log it, and don't
                // try to restart.
                error!("Executable could not be found: {:?}", err);
            }
            Ok(exe) => {
                info!("Restarting using {}", exe.display());
                let error = execvp(exe, env::args_os());
                // The new process will take over the current process, so
                // execution stops here, unless an error occurred, in which we
                // log it, but continue running.
                error!("execvp failed: {}", error);
            }
        }
    }


    /// Set the background (wallpaper) color.
    ///
    /// When the color is invalid, an `Err` is returned.
    pub fn set_background(&self, color_name: &str) -> X11Result<()> {
        let screen_number = unsafe { (self.xlib.XDefaultScreen)(self.display) };
        let colormap = unsafe { (self.xlib.XDefaultColormap)(self.display, screen_number) };
        let mut screen_def_return: xlib::XColor = unsafe { zeroed() };
        let mut exact_def_return: xlib::XColor = unsafe { zeroed() };
        let status = unsafe {
            (self.xlib.XAllocNamedColor)(self.display,
                                         colormap,
                                         CString::new(color_name)
                                             .unwrap()
                                             .as_ptr(),
                                         &mut screen_def_return,
                                         &mut exact_def_return)
        };
        if status == 0 {
            Err(X11Error::msg(format!("Could not allocate color: {}", color_name)))
        } else {
            unsafe {
                (self.xlib.XSetWindowBackground)(self.display,
                                                 self.root_window,
                                                 screen_def_return.pixel);
                (self.xlib.XClearWindow)(self.display, self.root_window);
            }
            trace!("set_background {}", color_name);
            Ok(())
        }
    }

    /// Return the list of atoms stored in the `WM_PROTOCOLS` property of the
    /// given window.
    ///
    /// Uses [`XGetWMProtocols`].
    ///
    /// [`XGetWMProtocols`]:
    /// https://tronche.com/gui/x/xlib/ICC/client-to-window-manager/XGetWMProtocols.html
    pub fn get_wm_protocols(&self, window: Window) -> Vec<xlib::Atom> {
        let mut protocols_ptr = unsafe { zeroed() };
        let mut nb_protocols = 0;
        let status = unsafe {
            (self.xlib.XGetWMProtocols)(self.display, window, &mut protocols_ptr, &mut nb_protocols)
        };
        if status == 0 {
            trace!("get_wm_protocols: XGetWMProtocols failed");
            return Vec::new();
        }
        let protocols = unsafe { slice::from_raw_parts(protocols_ptr, nb_protocols as usize) };
        let protocols_vec = protocols.to_vec();
        unsafe {
            (self.xlib.XFree)(transmute(protocols_ptr));
        }
        trace!("get_wm_protocols: {:?}", protocols_vec);
        protocols_vec
    }

    /// Retrieve the title of the given window.
    ///
    /// Return `None` when the window has no title or when it could not be
    /// converted to a valid `String`.
    pub fn get_window_title(&self, window: Window) -> Option<String> {

        // Try to get the _NET_WM_NAME property of the window
        let net_wm_name_atom = self.get_atom("_NET_WM_NAME");
        let mut text_prop_return = unsafe { zeroed() };
        let status = unsafe {
            (self.xlib.XGetTextProperty)(self.display,
                                         window,
                                         &mut text_prop_return,
                                         net_wm_name_atom)
        };
        let maybe_title: Option<String> = if status == 0 {
            None
        } else {
            let mut list_return: *mut *mut wchar_t = unsafe { zeroed() };
            let mut count_return = 0;
            let status = unsafe {
                (self.xlib
                    .XwcTextPropertyToTextList)(self.display,
                                                &text_prop_return as *const xlib::XTextProperty,
                                                &mut list_return,
                                                &mut count_return)
            };
            if status >= xlib::Success as c_int && count_return > 0 {
                // Try to convert the first item to a string
                let maybe_string = wide_string_to_string(unsafe { *list_return });
                unsafe {
                    (self.xlib.XFreeStringList)(transmute(list_return));
                }
                maybe_string
            } else {
                None
            }
        };

        maybe_title.or_else(|| {
            // If the _NET_WM_NAME property could not be read, try the WM_NAME
            // property
            let mut wm_name = unsafe { zeroed() };
            let status = unsafe { (self.xlib.XFetchName)(self.display, window, &mut wm_name) };
            if status == 0 || wm_name.is_null() {
                None
            } else {
                let cstr = unsafe { CStr::from_ptr(wm_name) };
                let maybe_string = cstr.to_str().ok().map(|s| s.to_owned());
                unsafe {
                    (self.xlib.XFree)(transmute(wm_name));
                }
                maybe_string
            }
        })
    }

    /// Close the given window.
    ///
    /// When the window supports the [ICCCM protocol], the protocol is
    /// followed. Otherwise, the window's client is killed with [`XKillClient`].
    ///
    /// [ICCCM protocol]: https://tronche.com/gui/x/icccm/sec-4.html#s-4.2.8.1
    /// [`XKillClient`]: https://tronche.com/gui/x/xlib/window-and-session-manager/XKillClient.html
    pub fn close_window(&self, window: Window) {
        trace!("close_window: {}", window);
        let protocols = self.get_wm_protocols(window);
        let wm_delete_window = self.get_atom("WM_DELETE_WINDOW");

        // If the window supports the right protocol (see URL), follow it.
        if protocols.contains(&wm_delete_window) {
            let wm_protocols = self.get_atom("WM_PROTOCOLS");
            let mut data = xlib::ClientMessageData::new();
            data.set_long(0, wm_delete_window as c_long);
            data.set_long(1, xlib::CurrentTime as c_long);
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
            debug!("Asking window to delete itself: {}", window);
            unsafe {
                (self.xlib.XSendEvent)(self.display,
                                       window,
                                       xlib::False,
                                       xlib::NoEventMask,
                                       (&mut xev as *mut xlib::XEvent));
            }
        } else {
            // Otherwise, just kill the window.
            debug!("Killing window: {}", window);
            unsafe {
                (self.xlib.XKillClient)(self.display, window);
            }
        }
    }

    /// Return the `XSizeHints` retrieved via [`XGetWMNormalHints`].
    ///
    /// Return `None` when the function status was zero.
    ///
    /// [`XGetWMNormalHints`]:
    /// https://tronche.com/gui/x/xlib/ICC/client-to-window-manager/XGetWMNormalHints.html
    pub fn get_wm_normal_hints(&self, window: Window) -> Option<xlib::XSizeHints> {
        let mut hints = unsafe { zeroed() };
        let mut supplied_return = 0; // we ignore this
        let status = unsafe {
            (self.xlib.XGetWMNormalHints)(self.display, window, &mut hints, &mut supplied_return)
        };
        if status != 0 { Some(hints) } else { None }
    }

    /// Check whether the given window wants to float or tile.
    ///
    /// If one of the following conditions is true, the window should float:
    ///
    /// * [`_NET_WM_WINDOW_TYPE`] property of the window contains
    ///   `_NET_WM_WINDOW_TYPE_DIALOG`. In other words, it is a dialog window
    /// * The window is a transient window for another one
    ///   ([`XGetTransientForHint`]).
    /// * The size hints of the window indicate that it has a fixed size.
    ///
    /// [`_NET_WM_WINDOW_TYPE`]: https://developer.gnome.org/wm-spec/#idm140200472629520
    /// [`XGetTransientForHint`]:
    /// https://tronche.com/gui/x/xlib/ICC/client-to-window-manager/XGetTransientForHint.html
    pub fn wants_to_float_or_tile(&self, window: Window) -> FloatOrTile {
        // First condition
        let net_wm_window_type_dialog = self.get_atom("_NET_WM_WINDOW_TYPE_DIALOG");
        let net_wm_window_type = self.get_atom("_NET_WM_WINDOW_TYPE");
        let window_type_props = self.get_window_property32(window, net_wm_window_type)
            .unwrap_or_default();
        let is_dialog = window_type_props.contains(&(net_wm_window_type_dialog as c_int));
        if is_dialog {
            return FloatOrTile::Float;
        }

        // Second condition
        let mut prop_window_return = 0;
        let status = unsafe {
            (self.xlib.XGetTransientForHint)(self.display, window, &mut prop_window_return)
        };
        let is_transient = status != 0 && prop_window_return != 0;
        if is_transient {
            return FloatOrTile::Float;
        }

        // Third condition
        let is_fixed_size = if let Some(hints) = self.get_wm_normal_hints(window) {
            // the min and max size hints are both set
            hints.flags & xlib::PMinSize != 0 && hints.flags & xlib::PMaxSize != 0 &&
            // the min and max size hints are equal -> fixed size
            hints.min_width == hints.max_width && hints.min_height == hints.max_height
        } else {
            // We got `None` instead of `Some(hints)`
            false
        };
        if is_fixed_size {
            FloatOrTile::Float
        } else {
            FloatOrTile::Tile
        }
    }

    /// Check whether the given window wants to be fullscreen.
    ///
    /// This is done by checking whether `_NET_WM_STATE_FULLSCREEN` is in the
    /// [`_NET_WM_STATE`] property of the window.
    ///
    /// [`_NET_WM_STATE`]: https://developer.gnome.org/wm-spec/#idm140200472615568
    pub fn wants_to_be_fullscreen(&self, window: Window) -> bool {
        let net_wm_state_fullscreen = self.get_atom("_NET_WM_STATE_FULLSCREEN");
        let net_wm_state = self.get_atom("_NET_WM_STATE");
        let window_state_props = self.get_window_property32(window, net_wm_state)
            .unwrap_or_default();
        window_state_props.contains(&(net_wm_state_fullscreen as c_int))
    }

    /// Set the window border width using `XSetWindowBorderWidth`.
    pub fn set_window_border_width(&self, window: Window, border_width: c_uint) {
        trace!("set_window_border_width: {}, {}", window, border_width);
        unsafe {
            (self.xlib.XSetWindowBorderWidth)(self.display, window, border_width);
        }
    }

    /// Set the window border color using `XSetWindowBorder`.
    pub fn set_window_border_color(&self, window: Window, color: xlib::XColor) {
        trace!("set_window_border_color: {}, {}", window, color.pixel);
        unsafe {
            (self.xlib.XSetWindowBorder)(self.display, window, color.pixel);
        }
    }

    /// Return the actual `Screen` (size).
    ///
    /// Do not confuse this with the [`get_screen`] method of the window
    /// manager. This method does the retrieves the `Screen` by querying the X
    /// server.
    ///
    /// [`get_screen`]: ../cplwm_api/wm/trait.WindowManager.html#tymethod.get_screen
    pub fn get_screen(&self) -> Screen {
        let xscreen = unsafe { (self.xlib.XDefaultScreenOfDisplay)(self.display) };
        Screen {
            width: unsafe { (*xscreen).width } as c_uint,
            height: unsafe { (*xscreen).height } as c_uint,
        }
    }
}
