//! Keyboard and mouse management.
//!
//! A lot of this code is based on [XMonad](https://github.com/xmonad/xmonad).

use std::fmt;
use std::os::raw::{c_int, c_uint, c_ulong};
use std::slice;

use super::*;

use cplwm_api::types::Window;
use cplwm_api::wm::WindowManager;

use x11_dl::keysym;
use x11_dl::xlib;


const MASKS: &'static [(XKeyMask, &'static str)] = &[(xlib::Mod5Mask, "M5"),
                                                     (xlib::Mod4Mask, "Super"),
                                                     (xlib::Mod3Mask, "M3"),
                                                     (xlib::Mod2Mask, "M2"),
                                                     (xlib::Mod1Mask, "Alt"),
                                                     (xlib::ControlMask, "Control"),
                                                     (xlib::LockMask, "CapsLock"),
                                                     (xlib::ShiftMask, "Shift")];

/// The type of a key mask.
///
/// A modifier key (Shift, Control, Alt, Super, ...) is a key mask. You can
/// combine modifiers by ORing their key masks together.
pub type XKeyMask = c_uint;

/// The type of a mouse button.
pub type XButton = c_uint;


/// A key pressed on the keyboard.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Key {
    /// The key mask, i.e. the active modifier keys.
    pub mask: XKeyMask,
    /// The key symbol. See the [`keysym`](../x11_dl/keysym/index.html) module
    /// for the possible key symbols. E.g., the R key on your keyboard is
    /// [`XK_r`](../x11_dl/keysym/constant.XK_r.html) (lower-case).
    pub sym: xlib::KeySym,
}
impl Key {
    /// Constructor for `Key`.
    pub fn new(mask: XKeyMask, sym: xlib::KeySym) -> Key {
        Key {
            mask: mask,
            sym: sym,
        }
    }
}

impl fmt::Display for Key {
    // Example output for:
    // `Key::new(xlib::Mod1Mask | xlib::Mod4Mask, XK_r as xlib::KeySym)`
    //
    // "Key: Alt - Control - 114"
    //
    // Note that there is no easy way to go from a keysym, e.g., XK_r, a
    // constant defined to be 114, to "XK_r" or "r", so we print the constant
    // number. Better than nothing. Storing all the keys (a lot!) with their
    // names in a hashmap is a solution.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(f, "Key: "));
        for &(mask, mask_name) in MASKS.iter() {
            if (self.mask & mask) != 0 {
                try!(write!(f, "{} - ", mask_name));
            }
        }
        write!(f, "{}", self.sym)
    }
}

/// A button pressed on the mouse.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Button {
    /// The key mask, i.e. the active modifier keys.
    pub mask: XKeyMask,
    /// The button. You can use [`xlib::Button1`] through [`xlib::Button5`].
    /// Note the right mouse button is `Button3` and not `Button2`, which is
    /// the middle mouse button.
    ///
    /// [`xlib::Button1`]: ../x11_dl/xlib/constant.Button1.html
    /// [`xlib::Button5`]: ../x11_dl/xlib/constant.Button5.html
    pub button: XButton,
}

impl Button {
    /// Constructor for `Button`.
    pub fn new(mask: XKeyMask, button: XButton) -> Button {
        Button {
            mask: mask,
            button: button,
        }
    }
}

impl fmt::Display for Button {
    // Example output for:
    // `Button::new(xlib::Mod1Mask | xlib::Mod4Mask, xlib::Button1)`
    //
    // "Key: Alt - Control - 1"
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(f, "Button: "));
        for &(mask, mask_name) in MASKS.iter() {
            if (self.mask & mask) != 0 {
                try!(write!(f, "{} - ", mask_name));
            }
        }
        write!(f, "{}", self.button)
    }
}

/// Keyboard and mouse management.
impl<WM> X11Backend<WM>
    where WM: WindowManager
{
    /// Figure out the numlock key mask and store in the backend.
    pub fn set_numlock_mask(&mut self) {
        let mut numlock_mask = 0;
        let modifier_keymap_ptr = unsafe { (self.xlib.XGetModifierMapping)(self.display) };
        let modifier_keymap = unsafe { *modifier_keymap_ptr };
        let keycodes = unsafe {
            slice::from_raw_parts(modifier_keymap.modifiermap,
                                  8 * modifier_keymap.max_keypermod as usize)
        };
        for (modifier, keycodes_per_modifier) in (xlib::ShiftMapIndex..xlib::Mod5MapIndex)
            .zip(keycodes.chunks(modifier_keymap.max_keypermod as usize)) {
            for keycode in keycodes_per_modifier {
                let keysym = unsafe { (self.xlib.XKeycodeToKeysym)(self.display, *keycode, 0) };
                if keysym == keysym::XK_Num_Lock as c_ulong {
                    numlock_mask |= 1 << modifier;
                }
            }
        }
        self.numlock_mask = numlock_mask;
        unsafe {
            (self.xlib.XFreeModifiermap)(modifier_keymap_ptr);
        }
    }

    /// Remove numlock and capslock from the [`XKeyMask`](type.XKeyMask.html).
    pub fn clean_mask(&self, mask: XKeyMask) -> XKeyMask {
        let nlm = self.numlock_mask;
        !(nlm | xlib::LockMask) & mask
    }


    /// Grab the keys.
    ///
    /// For each key binding, we *grab* the key. This means that we start
    /// listening for the events generated when pressing or releasing one of
    /// these keys.
    ///
    /// See https://tronche.com/gui/x/xlib/input/XGrabKey.html.
    pub fn grab_keys(&self, key_bindings: &KeyBindings<WM>) {
        // Ungrab everything first
        unsafe {
            (self.xlib.XUngrabKey)(self.display,
                                   xlib::AnyKey,
                                   xlib::AnyModifier,
                                   self.root_window)
        };
        let nlm = self.numlock_mask;
        let modifier_masks = vec![0, nlm, xlib::LockMask, nlm | xlib::LockMask];
        // Grab all the key bindings
        for &Key { mask, sym } in key_bindings.keys() {
            for modifier_mask in &modifier_masks {
                let keycode = unsafe { (self.xlib.XKeysymToKeycode)(self.display, sym) };
                if keycode != 0 {
                    unsafe {
                        (self.xlib.XGrabKey)(self.display,
                                             keycode as c_int,
                                             mask | modifier_mask,
                                             self.root_window,
                                             xlib::True,
                                             xlib::GrabModeAsync,
                                             xlib::GrabModeAsync)
                    };
                }
            }
        }
    }

    /// Grab or ungrab the given button and keymask on the given window.
    ///
    /// Grab when `grab` is `true`, ungrab when `false`.
    pub fn set_button_grab(&self, grab: bool, window: Window, button: XButton, mask: XKeyMask) {
        unsafe {
            if grab {
                (self.xlib.XGrabButton)(self.display,
                                        button,
                                        mask,
                                        window,
                                        xlib::False,
                                        xlib::ButtonPressMask as c_uint,
                                        xlib::GrabModeAsync,
                                        xlib::GrabModeSync,
                                        0,
                                        0);
            } else {
                (self.xlib.XUngrabButton)(self.display, button, mask, window);
            }
        }
    }

    /// Grab the buttons.
    ///
    /// For each button binding, we *grab* the button. This means that we
    /// start listening for the events generated when pressing or releasing
    /// one of these buttons.
    pub fn grab_buttons(&self, button_bindings: &ButtonBindings<WM>) {
        // Ungrab everything first
        self.set_button_grab(false,
                             self.root_window,
                             xlib::AnyButton as XButton,
                             xlib::AnyModifier);
        let nlm = self.numlock_mask;
        let modifier_masks = vec![0, nlm, xlib::LockMask, nlm | xlib::LockMask];
        // Grab all the button bindings
        for &Button { mask, button } in button_bindings.keys() {
            for modifier_mask in &modifier_masks {
                self.set_button_grab(true, self.root_window, button, mask | modifier_mask);
            }
        }
    }
}
