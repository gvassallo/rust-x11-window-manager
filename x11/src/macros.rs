//! Macros to define key and button bindings
//!
//! The only two things you will use directly from this module are the
//! [`key_bindings`] and [`button_bindings`] macros. All the other functions
//! and macros are helpers.
//!
//! [`key_bindings`]: ../macro.key_bindings!.html
//! [`button_bindings`]: ../macro.button_bindings!.html

use std::os::raw::c_uint;
use std::ops::BitOr;

use super::{Button, Key};

use x11_dl::xlib::KeySym;


/// Make a [`Key`] from a `Vec` of key symbols.
///
/// The last element in the `Vec` is the actual key, all elements before it
/// are modifiers. Note that no validation is done to ensure that the last key
/// is a valid key and not a modifier.
///
/// This function panics when the `Vec` is empty, because there must be at
/// least a key to press.
///
/// **Note**: don't use this function, it is a helper function for
/// [`key_bindings`].
///
/// # Examples
///
/// ```
/// let key1 = make_key(vec![xlib::Mod4Mask, xlib::ShiftMask,
///                          keysym::XK_Return]);
/// assert_eq!(Key::new(xlib::Mod4Mask | xlib::ShiftMask,
///                     keysym::XK_Return as KeySym),
///            key1);
/// ```
/// ```
/// let key2 = make_key(vec![keysym::XK_r]);
/// assert_eq!(Key::new(0, keysym::XK_Return as KeySym), key2);
/// ```
///
/// [`Key`]: struct.Key.html
/// [`key_bindings`]: macro.key_bindings!.html
pub fn make_key(keysyms: Vec<c_uint>) -> Key {

    if let Some(keysym) = keysyms.last() {
        let modifiers = &keysyms[0..keysyms.len() - 1];
        Key::new(modifiers.iter().fold(0, BitOr::bitor), *keysym as KeySym)
    } else {
        panic!("Empty key")
    }
}


#[cfg(test)]
#[test]
fn test_make_key() {
    use x11_dl::keysym;
    use x11_dl::xlib;
    assert_eq!(Key::new(xlib::Mod4Mask | xlib::ShiftMask,
                        keysym::XK_Return as KeySym),
               make_key(vec![xlib::Mod4Mask, xlib::ShiftMask, keysym::XK_Return]));
    assert_eq!(Key::new(xlib::Mod4Mask | xlib::ShiftMask,
                        keysym::XK_Return as KeySym),
               make_key(vec![xlib::ShiftMask, xlib::Mod4Mask, keysym::XK_Return]));
    assert_eq!(Key::new(0, keysym::XK_r as KeySym),
               make_key(vec![keysym::XK_r]));
}

#[cfg(test)]
#[test]
#[should_panic]
fn test_make_key_empty_vec() {
    let _ = make_key(Vec::new());
}

/// Translate a key name to a keysym (`c_uint`).
///
/// Key names are user-friendly names for existing modifiers. Regular keysyms,
/// e.g., [`XK_Tab`], [`XK_c`], are not translated and just prefixed with
/// `keysym::`. See [`x11_dl::keysym`](../x11_dl/keysym/index.html) for a list
/// of available keys.
///
/// The following translations are performed:
///
/// * `Control` becomes [`xlib::ControlMask`].
/// * `Shift` becomes [`xlib::ShiftMask`].
/// * `Alt` becomes [`xlib::Mod1Mask`].
/// * `Super` (the Windows key) becomes [`xlib::Mod4Mask`].
///
/// **Note**: don't use this macro, it is a helper for [`key_bindings`].
///
/// [`XK_Tab`]: ../x11_dl/keysym/constant.XK_Tab.html
/// [`XK_c`]: ../x11_dl/keysym/constant.XK_c.html
/// [`xlib::ControlMask`]: ../x11_dl/xlib/constant.ControlMask.html
/// [`xlib::ShiftMask`]: ../x11_dl/xlib/constant.ShiftMask.html
/// [`xlib::Mod1Mask`]: ../x11_dl/xlib/constant.Mod1Mask.html
/// [`xlib::Mod4Mask`]: ../x11_dl/xlib/constant.Mod4Mask.html
///
/// [`key_bindings`]: macro.key_bindings!.html
#[macro_export]
macro_rules! translate_key_name {
    (Control)    => { xlib::ControlMask };
    (Shift)      => { xlib::ShiftMask };
    (Alt)        => { xlib::Mod1Mask };
    (Super)      => { xlib::Mod4Mask };
    ($key:ident) => { keysym::$key };
}

/// Translate a user-friendly key to a [`Key`].
///
/// See [`translate_key_name`] for the available key names.
///
///
/// # Examples
///
/// ```
/// let key1 = translate_key!((XK_Return));
/// assert_eq!(Key::new(0, keysym::XK_Return as KeySym), key1);
/// ```
/// ```
/// let key2 = translate_key!((Control - Shift - Alt - Super - XK_t));
/// assert_eq!(Key::new(xlib::ControlMask | xlib::ShiftMask | xlib::Mod1Mask |
///                     xlib::Mod4Mask,
///                     keysym::XK_t as KeySym), key2);
/// ```
///
/// Note the parentheses around the input, these are required due to
/// pattern-matching.
///
/// **Note**: don't use this macro, it is a helper for [`key_bindings`].
///
/// [`Key`]: struct.Key.html
/// [`translate_key_name`]: macro.translate_key_name!.html
/// [`key_bindings`]: macro.key_bindings!.html
#[macro_export]
macro_rules! translate_key {
    ((
        // Key names are identifiers
        $($key_name:ident)
        // Separated by -
            -
        // There must be one or more
            +
    )) => {{
        use x11_dl::xlib;
        use x11_dl::keysym;
        let mut keysyms = ::std::vec::Vec::new();
        // Translate and push each key name to the vec
        $(
            keysyms.push(translate_key_name!($key_name));
        )+;
        $crate::make_key(keysyms)
    }};
}


#[cfg(test)]
#[test]
fn test_translate_key() {
    use x11_dl::keysym;
    use x11_dl::xlib;
    assert_eq!(Key::new(xlib::ControlMask | xlib::ShiftMask | xlib::Mod1Mask | xlib::Mod4Mask,
                        keysym::XK_t as KeySym),
               translate_key!((Control - Shift - Alt - Super - XK_t)));
    assert_eq!(Key::new(xlib::ControlMask, keysym::XK_0 as KeySym),
               translate_key!((Control - XK_0)));
    assert_eq!(Key::new(0, keysym::XK_Return as KeySym),
               translate_key!((XK_Return)));
}

/// User-friendly way to define [`KeyBindings<WM>`] for a window manager `WM`.
///
/// Instead of:
///
/// ```
/// let mut m: KeyBindings<WM> = HashMap::new();
/// m.insert(Key::new(xlib::Mod4Mask, keysym::XK_Return as KeySym),
///          Box::new(|_| spawn("xterm")));
/// m
/// ```
///
/// you can write the following using this macro:
///
/// ```
/// key_bindings! { WM =>
///     (Super - XK_return) => |_| spawn("xterm")
/// }
/// ```
///
/// Of course multiple bindings (separated by a comma) are supported. A
/// trailing comma is unfortunately not allowed because of how Rust macros are
/// parsed.
///
/// Note the `WM =>` at the start of the macro, this is the type of the window
/// manager and the type parameter in [`KeyBindings<WM>`].
///
/// The left-hand side of each arrow is a user-friendly name for a key. See
/// [`translate_key_name`] for which key names are accepted.
///
/// The right-hand side of each arrow is a closure of type [`KeyCommand<WM>`].
///
/// [`translate_key_name`]: macro.translate_key_name!.html
/// [`KeyBindings<WM>`]: type.KeyBindings.html
/// [`KeyCommand<WM>`]: type.KeyCommand.html
#[macro_export]
macro_rules! key_bindings {
    (
        $wm:ty =>
            $($keys:tt => $closure:expr), *
    ) => {{
        let mut m: $crate::KeyBindings<$wm> = ::std::collections::HashMap::new();

        $( m.insert(translate_key!($keys), Box::new($closure)); )*;
        m
    }};
}


/// Make a [`Button`] from a `Vec` of symbols.
///
/// The last element in the `Vec` is the actual button, all elements before it
/// are modifiers. Note that no validation is done to ensure that the last
/// element is a button and not a modifier.
///
/// This function panics when the `Vec` is empty, because there must be at
/// least a button to press.
///
/// **Note**: don't use this function, it is a helper function for
/// [`button_bindings`].
///
/// # Examples
///
/// ```
/// let button1 = make_button(vec![xlib::Mod4Mask, xlib::ShiftMask,
///                                xlib::Button1]);
/// assert_eq!(Button::new(xlib::Mod4Mask | xlib::ShiftMask, xlib::Button1),
///            button1);
/// ```
/// ```
/// let button2 = make_button(vec![xlib::Button3]);
/// assert_eq!(Button::new(0, xlib::Button), button2);
/// ```
///
/// [`Button`]: struct.Button.html
/// [`button_bindings`]: macro.button_bindings!.html
pub fn make_button(symbols: Vec<c_uint>) -> Button {
    if let Some(button) = symbols.last() {
        let modifiers = &symbols[0..symbols.len() - 1];
        Button::new(modifiers.iter().fold(0, BitOr::bitor), *button)
    } else {
        panic!("Empty button")
    }
}


#[cfg(test)]
#[test]
fn test_make_button() {
    use x11_dl::xlib;
    assert_eq!(Button::new(xlib::Mod4Mask | xlib::ShiftMask, xlib::Button1),
               make_button(vec![xlib::Mod4Mask, xlib::ShiftMask, xlib::Button1]));
    assert_eq!(Button::new(xlib::Mod4Mask | xlib::ShiftMask, xlib::Button2),
               make_button(vec![xlib::ShiftMask, xlib::Mod4Mask, xlib::Button2]));
    assert_eq!(Button::new(0, xlib::Button5),
               make_button(vec![xlib::Button5]));
}

#[cfg(test)]
#[test]
#[should_panic]
fn test_make_button_empty_vec() {
    let _ = make_button(Vec::new());
}

/// Translate a button or modifier name to a symbol (`c_uint`).
///
/// User-friendly names are translated to the existing modifier and button
/// names.
///
/// The following translations are performed:
///
/// * `Control` becomes [`xlib::ControlMask`].
/// * `Shift` becomes [`xlib::ShiftMask`].
/// * `Alt` becomes [`xlib::Mod1Mask`].
/// * `Super` (the Windows key) becomes [`xlib::Mod4Mask`].
/// * `LMB`/`MMB`/`RMB` (left/middle/right mouse button) become
///   [`xlib::Button1`]/[`xlib::Button2`]/[`xlib::Button3`].
/// * `MB[1-5]` (mouse button 1 to 5) become `xlib::Button[1-5]`.
///
/// Note that the right mouse button is [`xlib::Button3`] and not
/// [`xlib::Button2`], which is the middle mouse button.
///
/// **Note**: don't use this macro, it is a helper for [`button_bindings`].
///
/// [`xlib::ControlMask`]: ../x11_dl/xlib/constant.ControlMask.html
/// [`xlib::ShiftMask`]: ../x11_dl/xlib/constant.ShiftMask.html
/// [`xlib::Mod1Mask`]: ../x11_dl/xlib/constant.Mod1Mask.html
/// [`xlib::Mod4Mask`]: ../x11_dl/xlib/constant.Mod4Mask.html
/// [`xlib::Button1`]: ../x11_dl/xlib/constant.Button1.html
/// [`xlib::Button2`]: ../x11_dl/xlib/constant.Button2.html
/// [`xlib::Button3`]: ../x11_dl/xlib/constant.Button3.html
///
/// [`button_bindings`]: macro.button_bindings!.html
#[macro_export]
macro_rules! translate_button_name {
    (Control) => { xlib::ControlMask };
    (Shift)   => { xlib::ShiftMask };
    (Alt)     => { xlib::Mod1Mask };
    (Super)   => { xlib::Mod4Mask };
    (LMB)     => { xlib::Button1 };
    (MMB)     => { xlib::Button2 };
    (RMB)     => { xlib::Button3 };
    (MB1)     => { xlib::Button1 };
    (MB2)     => { xlib::Button2 };
    (MB3)     => { xlib::Button3 };
    (MB4)     => { xlib::Button4 };
    (MB5)     => { xlib::Button5 };
}

/// Translate a user-friendly button to a [`Button`].
///
/// See [`translate_button_name`] for the available button names.
///
/// # Examples
///
/// ```
/// let button1 = translate_button!((LMB));
/// assert_eq!(Button::new(0, xlib::Button1), button1);
/// ```
/// ```
/// let button2 = translate_button!((Control - Shift - Alt - Super - RMB));
/// assert_eq!(Button::new(xlib::ControlMask | xlib::ShiftMask | xlib::Mod1Mask |
///                        xlib::Mod4Mask,
///                        xlib::Button3), button2);
/// ```
///
/// Note the parentheses around the input, these are required due to
/// pattern-matching.
///
/// **Note**: don't use this macro, it is a helper for [`button_bindings`].
///
/// [`Button`]: struct.Button.html
/// [`translate_button_name`]: macro.translate_button_name!.html
/// [`button_bindings`]: macro.button_bindings!.html
#[macro_export]
macro_rules! translate_button {
    ((
        // Button names are identifiers
        $($button_name:ident)
        // Separated by -
            -
        // There must be one or more
            +
    )) => {{
        use x11_dl::xlib;
        let mut symbols = ::std::vec::Vec::new();
        // Translate and push each key name to the vec
        $(
            symbols.push(translate_button_name!($button_name));
        )+;
        $crate::make_button(symbols)
    }};
}


#[cfg(test)]
#[test]
fn test_translate_button() {
    use x11_dl::xlib;
    assert_eq!(Button::new(xlib::ControlMask | xlib::ShiftMask | xlib::Mod1Mask | xlib::Mod4Mask,
                           xlib::Button1),
               translate_button!((Control - Shift - Alt - Super - LMB)));
    assert_eq!(Button::new(xlib::ControlMask, xlib::Button3),
               translate_button!((Control - RMB)));
    assert_eq!(Button::new(0, xlib::Button4), translate_button!((MB4)));
}


/// User-friendly way to define [`ButtonBindings<WM>`] for a window
/// manager`WM`.
///
/// Instead of:
///
/// ```
/// let mut m: ButtonBindings<WM> = HashMap::new();
/// m.insert(Button::new(xlib::Mod4Mask, xlib::Button1),
///          Box::new(|backend, ev| {
///              backend.mouse_move_window(ev.subwindow)
///          }));
/// m
/// ```
///
/// you can write the following using this macro:
///
/// ```
/// button_bindings! { WM =>
///     (Super - LMB) => |backend, ev| {
///         backend.mouse_move_window(ev.subwindow)
///     }
/// }
/// ```
///
/// Of course multiple bindings (separated by a comma) are supported. A
/// trailing comma is unfortunately not allowed because of how Rust macros are
/// parsed.
///
/// Note the `WM =>` at the start of the macro, this is the type of the window
/// manager and the type parameter in [`ButtonBindings<WM>`].
///
/// The left-hand side of each arrow is a user-friendly name for a button. See
/// [`translate_button_name`] for which button names are accepted.
///
/// The right-hand side of each arrow is a closure of type
/// [`ButtonCommand<WM>`].
///
/// [`translate_button_name`]: macro.translate_button_name!.html
/// [`ButtonBindings<WM>`]: type.ButtonBindings.html
/// [`ButtonCommand<WM>`]: type.ButtonCommand.html
#[macro_export]
macro_rules! button_bindings {
    (
        $wm:ty =>
            $($buttons:tt => $closure:expr), *
    ) => {{
        let mut m: $crate::ButtonBindings<$wm> = ::std::collections::HashMap::new();

        $( m.insert(translate_button!($buttons), Box::new($closure)); )*;
        m
    }};
}
