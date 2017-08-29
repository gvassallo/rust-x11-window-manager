//! EWMH Support.
//!
//! Extended Window Manager Hints (EWMH) are a way the window manager
//! communicates with the running applications. For example, it provides ways
//! for applications to indicate that they want to be hidden or displayed
//! fullscreen. It also lets panels or task bars know which windows are
//! visible on the current workspace and which window is focused.
//!
//! The protocol is extensive, but we only implement the necessary bits to get
//! the mentioned functionality working.
//!
//! See https://developer.gnome.org/wm-spec/ for more information.
//!
//! This is code based on:
//!
//! * https://hackage.haskell.org/package/xmonad-contrib/docs/XMonad-Hooks-EwmhDesktops.html
//! * https://hackage.haskell.org/package/xmonad-contrib/docs/XMonad-Util-WindowProperties.html
//! * https://hackage.haskell.org/package/xmonad-contrib/docs/XMonad-Hooks-SetWMName.htm

use cplwm_api::types::Window;
use cplwm_api::wm::{FloatSupport, FullscreenSupport, MinimiseSupport, WindowManager};

use std::os::raw::{c_int, c_long};
use std::error;

use super::*;

use x11_dl::xlib;

/// The hints this window manager supports.
pub const SUPPORTED_ATOM_NAMES: &'static [&'static str] = &["_NET_ACTIVE_WINDOW",
                                                            "_NET_CLIENT_LIST",
                                                            "_NET_CLIENT_LIST_STACKING",
                                                            "_NET_WM_STATE",
                                                            "_NET_WM_STATE_FULLSCREEN",
                                                            "_NET_WM_STATE_HIDDEN"];

/// The actions windows are allowed to perform.
pub const ALLOWED_ACTIONS_ATOM_NAMES: &'static [&'static str] = &["_NET_WM_ACTION_MOVE",
                                                                  "_NET_WM_ACTION_RESIZE",
                                                                  "_NET_WM_ACTION_MINIMIZE",
                                                                  "_NET_WM_ACTION_FULLSCREEN",
                                                                  "_NET_WM_ACTION_CLOSE"];

/// EWMH Support.
impl<WM> X11Backend<WM>
    where WM: WindowManager + FloatSupport + FullscreenSupport + MinimiseSupport
{
    /// Advertise which hints are supported.
    ///
    /// Sets the [`_NET_SUPPORTED`] property of the root window to a list of
    /// atoms to advertise to clients that the window manager supports these
    /// hints (functionality). You can use [`SUPPORTED_ATOM_NAMES`] for this.
    /// Clients will often check this before trying to use the functionality.
    /// This should must only be done once, at start-up.
    ///
    /// For example, the mpv video player will only try to make a window
    /// fullscreen via the [`_NET_WM_STATE`] hint if the [`_NET_SUPPORTED`]
    /// property contains `_NET_WM_STATE_FULLSCREEN`.
    ///
    /// [`_NET_SUPPORTED`]: https://developer.gnome.org/wm-spec/#idm140200472725888
    /// [`_NET_WM_STATE`]: https://developer.gnome.org/wm-spec/#idm140200472615568
    /// [`SUPPORTED_ATOM_NAMES`]: constant.SUPPORTED_ATOM_NAMES.html
    pub fn set_net_supported<Names: Iterator<Item = &'static str>>(&self, atom_names: Names) {
        let net_supported_atom = self.get_atom("_NET_SUPPORTED");
        let supported_atoms = atom_names.map(|name| self.get_atom(name));
        self.change_window_property32(self.root_window,
                                      net_supported_atom,
                                      xlib::XA_ATOM,
                                      xlib::PropModeReplace,
                                      supported_atoms.map(|atom| atom as c_int));
    }

    /// Advertise which actions are supported for the given window.
    ///
    /// Sets the [`_NET_WM_ALLOWED_ACTIONS`] property of the given window to a
    /// lits of atoms that advertise the actions to can be performed on the
    /// given window. You can use [`ALLOWED_ACTIONS_ATOM_NAMES`] for this.
    /// This should only be done once per window, when it is mapped.
    ///
    /// [`_NET_WM_ALLOWED_ACTIONS`]: https://developer.gnome.org/wm-spec/#idm140200472593792
    /// [`ALLOWED_ACTIONS_ATOM_NAMES`]: constant.ALLOWED_ACTIONS_ATOM_NAMES.html
    pub fn set_allowed_actions<Names: Iterator<Item = &'static str>>(&self,
                                                                     window: Window,
                                                                     atom_names: Names) {
        let net_wm_allowed_actions_atom = self.get_atom("_NET_WM_ALLOWED_ACTIONS");
        let allowed_actions_atoms = atom_names.map(|name| self.get_atom(name));
        self.change_window_property32(window,
                                      net_wm_allowed_actions_atom,
                                      xlib::XA_ATOM,
                                      xlib::PropModeReplace,
                                      allowed_actions_atoms.map(|atom| atom as c_int));
    }

    /// Advertise the managed windows.
    ///
    /// Sets the [`_NET_CLIENT_LIST`] property of the root window. The list of
    /// windows should be ordered by age: the oldest (first mapped) window
    /// should come first, the youngest (most recently mapped) last.
    ///
    /// [`_NET_CLIENT_LIST`]: https://developer.gnome.org/wm-spec/#idm140200472723904
    pub fn set_client_list<'a, Windows: Iterator<Item = &'a Window>>(&self, windows: Windows) {
        let net_current_client_list_atom = self.get_atom("_NET_CLIENT_LIST");
        self.change_window_property32(self.root_window,
                                      net_current_client_list_atom,
                                      xlib::XA_WINDOW,
                                      xlib::PropModeReplace,
                                      windows.map(|w| *w as c_int));
    }

    /// Advertise the managed windows.
    ///
    /// Sets the [`_NET_CLIENT_LIST_STACKING`] property of the root window.
    /// The list of windows should be ordered by the stacking order: the
    /// bottom window first, the top window last.
    ///
    /// [`_NET_CLIENT_LIST_STACKING`]: https://developer.gnome.org/wm-spec/#idm140200472723904
    pub fn set_client_list_stacking<'a, Windows: Iterator<Item = &'a Window>>(&self,
                                                                              windows: Windows) {
        let net_current_client_list_stacking_atom = self.get_atom("_NET_CLIENT_LIST_STACKING");
        self.change_window_property32(self.root_window,
                                      net_current_client_list_stacking_atom,
                                      xlib::XA_WINDOW,
                                      xlib::PropModeReplace,
                                      windows.map(|w| *w as c_int));
    }

    /// Advertise the focused window.
    ///
    /// Sets the [`_NET_ACTIVE_WINDOW`] property of the root window. Pass
    /// `None` when no window is focused.
    ///
    /// [`_NET_ACTIVE_WINDOW`]: https://developer.gnome.org/wm-spec/#idm140200472702304
    pub fn set_active_window(&self, focused_window: Option<Window>) {
        let net_active_window_atom = self.get_atom("_NET_ACTIVE_WINDOW");
        self.change_window_property32(self.root_window,
                                      net_active_window_atom,
                                      xlib::XA_WINDOW,
                                      xlib::PropModeReplace,
                                      Some(focused_window.unwrap_or(0) as c_int).into_iter());
    }

    /// Private helper function for `handle_ewmh_client_message`.
    fn net_wm_state_toggler<F, E>(&mut self,
                                  window: Window,
                                  net_wm_state_atom: xlib::Atom,
                                  existing_states: &mut Vec<c_int>,
                                  data: &Vec<c_long>,
                                  action: c_long,
                                  toggle_function: F)
                                  -> X11Result<()>
        where F: Fn(&mut X11Backend<WM>, Window) -> Result<(), E>,
              E: Into<X11Error> + error::Error + 'static
    {
        if data.contains(&(net_wm_state_atom as c_long)) {
            let existing_state_pos = existing_states.iter()
                .position(|state| net_wm_state_atom as c_int == *state);
            use self::AddRemoveNothing::*;
            let add_remove_nothing = match (action, existing_state_pos) {
                (_NET_WM_STATE_ADD, None) => Add,
                (_NET_WM_STATE_TOGGLE, None) => Add,
                (_NET_WM_STATE_TOGGLE, Some(pos)) => Remove(pos),
                (_NET_WM_STATE_REMOVE, Some(pos)) => Remove(pos),
                _ => DoNothing,
            };
            match add_remove_nothing {
                Add => existing_states.push(net_wm_state_atom as c_int),
                Remove(pos) => {
                    existing_states.remove(pos);
                    ()
                }
                DoNothing => (),
            }
            if !add_remove_nothing.is_do_nothing() {
                self.change_window_property32(window,
                                              net_wm_state_atom,
                                              xlib::XA_ATOM,
                                              xlib::PropModeReplace,
                                              existing_states.iter().map(|name| *name));
                try!(toggle_function(self, window));
            }
        }
        Ok(())
    }

    /// Handle an [`XClientMessageEvent`] that represents an EWMH action.
    ///
    /// The following actions are supported:
    ///
    /// * [`_NET_ACTIVE_WINDOW`]
    /// * [`_NET_CLOSE_WINDOW`]
    /// * [`_NET_WM_STATE`]: only `_NET_WM_STATE_FULLSCREEN` and `_NET_WM_STATE_HIDDEN`.
    ///
    /// [`XClientMessageEvent`]: ../x11_dl/xlib/struct.XClientMessageEvent.html
    /// [`_NET_ACTIVE_WINDOW`]: https://developer.gnome.org/wm-spec/#idm140200472702304
    /// [`_NET_CLOSE_WINDOW`]: https://developer.gnome.org/wm-spec/#idm140200472668896
    /// [`_NET_WM_STATE`]: https://developer.gnome.org/wm-spec/#idm140200472615568
    pub fn handle_ewmh_client_message(&mut self, xev: xlib::XClientMessageEvent) -> X11Result<()> {
        let net_active_window_atom = self.get_atom("_NET_ACTIVE_WINDOW");
        let net_close_window_atom = self.get_atom("_NET_CLOSE_WINDOW");
        let net_wm_state_atom = self.get_atom("_NET_WM_STATE");

        if xev.message_type == net_active_window_atom {

            if self.get_wm().is_managed(xev.window) {
                try!(self.get_wm_mut().focus_window(Some(xev.window)));
            }

        } else if xev.message_type == net_close_window_atom {

            if self.get_wm().is_managed(xev.window) {
                try!(self.get_wm_mut().remove_window(xev.window));
                self.remove_window(xev.window);
            }

        } else if xev.message_type == net_wm_state_atom {
            // The existing _NET_WM_STATE properties of the window
            let mut existing_states = self.get_window_property32(xev.window, net_wm_state_atom)
                .unwrap_or_default();

            // Make a `Vec` of the 0, 1, or 2 properties to alter
            let data = {
                let mut data_vec = Vec::new();
                if xev.data.get_long(1) != 0 {
                    data_vec.push(xev.data.get_long(1));
                }
                if xev.data.get_long(2) != 0 {
                    data_vec.push(xev.data.get_long(2));
                }
                data_vec
            };

            // Remove = 0, Add = 1, Toggle = 2
            let action = xev.data.get_long(0);

            let net_wm_state_fullscreen_atom = self.get_atom("_NET_WM_STATE_FULLSCREEN");
            try!(self.net_wm_state_toggler(xev.window,
                                           net_wm_state_fullscreen_atom,
                                           &mut existing_states,
                                           &data,
                                           action,
                                           |backend, window| {
                                               backend.get_wm_mut().toggle_fullscreen(window)
                                           }));
            let net_wm_state_hidden_atom = self.get_atom("_NET_WM_STATE_HIDDEN");
            try!(self.net_wm_state_toggler(xev.window,
                                           net_wm_state_hidden_atom,
                                           &mut existing_states,
                                           &data,
                                           action,
                                           |backend, window| {
                                               backend.get_wm_mut().toggle_minimised(window)
                                           }));
        }
        Ok(())
    }
}


// Private helpers for `net_wm_state_toggler`

/// Remove/unset a `_NET_WM_STATE_*` property
const _NET_WM_STATE_REMOVE: c_long = 0;

/// Add/set a `_NET_WM_STATE_*` property
const _NET_WM_STATE_ADD: c_long = 1;

/// Toggle a `_NET_WM_STATE_*` property
const _NET_WM_STATE_TOGGLE: c_long = 2;

/// Used in `handle_ewmh_client_message`
enum AddRemoveNothing {
    /// Add the property
    Add,
    /// Remove the property
    ///
    /// The argument is its current index in the `Vec`.
    Remove(usize),
    /// Do nothing
    DoNothing,
}

impl AddRemoveNothing {
    /// Return `true` in the case it's a `DoNothing`.
    fn is_do_nothing(&self) -> bool {
        match *self {
            AddRemoveNothing::DoNothing => true,
            _ => false,
        }
    }
}
