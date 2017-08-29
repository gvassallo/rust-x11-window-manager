//! Types used throughout the backend.

use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::From;
use std::error;
use std::fmt;
use std::os::raw::{c_int, c_long, c_uint};

use super::*;

use cplwm_api::types::Window;

use x11_dl::xlib;

/// The type of errors the X11 backend can return.
pub enum X11Error {
    /// This window is not known by the backend.
    UnknownWindow(Window),
    /// An error message.
    Msg(Cow<'static, str>),
    /// An error implementing the [`Error`] trait.
    ///
    /// The errors returned by the window manager also implement this trait
    /// and will be stored in this constructor.
    ///
    /// [`Error`]: https://doc.rust-lang.org/std/error/trait.Error.html
    Error(Box<error::Error>),
}

impl X11Error {
    /// Return a new error based on the given message.
    ///
    /// A message can be a `&'static str` or a `String`.
    pub fn msg<Msg>(msg: Msg) -> X11Error
        where Msg: Into<Cow<'static, str>>
    {
        X11Error::Msg(msg.into())
    }
}

// Errors implementing the `Error` trait can be converted to an `X11Error`.
//
// This functionality is used all the time in the try! macro. Look up the
// documentation of the try! macro to see how this is used.
impl<E: error::Error + 'static> From<E> for X11Error {
    fn from(error: E) -> X11Error {
        X11Error::Error(Box::new(error))
    }
}

impl fmt::Debug for X11Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            X11Error::UnknownWindow(w) => write!(f, "Backend error: unknown window: {}", w),
            X11Error::Msg(ref error) => write!(f, "Error: {}", error),
            X11Error::Error(ref error) => write!(f, "Error: {}", error),
        }

    }
}

/// A `Result` with `X11Error` as error type.
pub type X11Result<T> = Result<T, X11Error>;

/// The type of an `XEvent` mask.
pub type XEventMask = c_long;

/// The type of a command that can be bound to a key press.
///
/// This type is parameterised over the type of the window manager, because it
/// (and the traits it implements) determines which methods a command can
/// call.
///
/// To modify the window manager, use the [`get_wm_mut`] method of the
/// backend.
///
/// [`get_wm_mut`]: struct.X11Backend.html#method.get_wm_mut
pub type KeyCommand<WM> = Box<Fn(&mut X11Backend<WM>) -> X11Result<()>>;

/// Map keys to commands.
pub type KeyBindings<WM> = HashMap<Key, KeyCommand<WM>>;

/// The type of a command that can be bound to a mouse button press.
///
/// This type is parameterised over the type of the window manager, because it
/// (and the traits it implements) determines which methods a command can
/// call.
///
/// To modify the window manager, use the [`get_wm_mut`] method of the
/// backend.
///
/// The [`XButtonEvent`] argument will contain information about the mouse
/// click, e.g. the coordinates of the pointer, etc. **Note** that the clicked
/// window is the `subwindow`. See the Xlib [documentation] for more
/// information about this type.
///
/// [`get_wm_mut`]: struct.X11Backend.html#method.get_wm_mut
/// [`XButtonEvent`]: ../x11_dl/xlib/struct.XButtonEvent.html
/// [documentation]: https://tronche.com/gui/x/xlib/events/keyboard-pointer/keyboard-pointer.html
pub type ButtonCommand<WM> = Box<Fn(&mut X11Backend<WM>, xlib::XButtonEvent) -> X11Result<()>>;

/// Map mouse buttons to commands.
pub type ButtonBindings<WM> = HashMap<Button, ButtonCommand<WM>>;

/// A color name as a string.
///
/// Colors can be hexadecimal, e.g. `"#ff00ff"` but also `"red"` or `"blue"`.
pub type ColorName = &'static str;

/// User configuration of the X11 backend.
pub struct X11Config<WM> {
    /// The key bindings chosen by the user.
    ///
    /// Use [`key_bindings`] to define these.
    ///
    /// [`key_bindings`]: macro.key_bindings!.html
    pub key_bindings: KeyBindings<WM>,
    /// The button bindings chosen by the user.
    ///
    /// Use [`button_bindings`] to define these.
    ///
    /// [`button_bindings`]: macro.button_bindings!.html
    pub button_bindings: ButtonBindings<WM>,
    /// The background (wallpaper) color.
    pub background_color: ColorName,
    /// The color used for the border of the focused window.
    pub focused_border_color: ColorName,
    /// The color used for the border of the unfocused windows.
    pub unfocused_border_color: ColorName,
}

impl<WM> Default for X11Config<WM> {
    /// A default `X11Config`.
    ///
    /// No bindings are defined, and some colors are chosen for the background
    /// and the borders.
    fn default() -> X11Config<WM> {
        X11Config {
            key_bindings: Default::default(),
            button_bindings: Default::default(),
            background_color: "#f4f4f4",
            focused_border_color: "#0f56c6",
            unfocused_border_color: "#c0d6f9",
        }
    }
}

/// The type of function that can be executed while dragging the mouse.
///
/// The two `c_int` arguments are the current x- and y-coordinates of the
/// dragged mouse pointer.
///
/// This is used to move/resize windows.
pub type WhileDragging<WM> = Fn(&mut X11Backend<WM>, c_int, c_int) -> X11Result<()>;

/// An enum to model the possible values for the `WM_STATE` property.
///
/// Instead of using constants, it is much safer to use an enum for this.
///
/// This is used by [`get_wm_state`] and [`set_wm_state`].
///
/// See https://tronche.com/gui/x/icccm/sec-4.html#WM_STATE
///
/// [`get_wm_state`]: struct.X11Backend.html#method.get_wm_state
/// [`set_wm_state`]: struct.X11Backend.html#method.set_wm_state
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum WindowState {
    /// The window is withdrawn (0)
    Withdrawn,
    /// The window is just visible (1)
    Normal,
    /// The window is minimised (3)
    Iconic,
}

impl From<WindowState> for c_uint {
    fn from(window_state: WindowState) -> c_uint {
        match window_state {
            WindowState::Withdrawn => 0,
            WindowState::Normal => 1,
            WindowState::Iconic => 3,
        }
    }
}

// Sometimes we need to convert it to `c_int` as well.
impl From<WindowState> for c_int {
    fn from(window_state: WindowState) -> c_int {
        let cuint: c_uint = From::from(window_state);
        cuint as c_int
    }
}

impl WindowState {
    /// Try to convert the `c_uint` to the corresponding `WindowState`.
    ///
    /// We can't implement [`From`] when returning an `Option` and [`TryFrom`]
    /// is unstable, so we just define a method.
    ///
    /// [`From`]: https://doc.rust-lang.org/std/convert/trait.From.html
    /// [`TryFrom`]: https://doc.rust-lang.org/std/convert/trait.TryFrom.html
    pub fn try_from(n: c_uint) -> Option<WindowState> {
        match n {
            0 => Some(WindowState::Withdrawn),
            1 => Some(WindowState::Normal),
            3 => Some(WindowState::Iconic),
            _ => None,
        }
    }
}
