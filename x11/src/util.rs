//! General utilities
//!
//! None of these utility functions need direct access to the backend's state.

use std::cmp::{max, min};
use std::env;
use std::ffi::{CString, OsStr};
use std::fs::File;
use std::io::Write;
use std::mem::zeroed;
use std::os::raw::{c_int, c_uint, c_ulong};
use std::path::{Path, PathBuf};
use std::slice;

use super::{ColorName, X11Error, X11Result};

use cplwm_api::types::{Geometry, Screen};

use libc::{wchar_t, wcslen};
use rustc_serialize::json::{Decoder, Encoder, Json};
use rustc_serialize::{Decodable, Encodable};
use x11_dl::xlib;


/// Filename used for the state file.
///
/// Note that this is not the full path to the file, use `get_state_file_path`
/// for this.
const STATE_FILENAME: &'static str = "wm_state";

lazy_static! {
    /// The path to the state as a `PathBuf`.
    ///
    /// **Don't use this**, use `get_state_file_path` instead.
    static ref STATE_FILE_PATH: PathBuf = {
        let mut buf = env::temp_dir();
        buf.push(STATE_FILENAME);
        buf
    };
}



/// Allocate an [`XColor`] with the given name in the given [`Colormap`].
///
/// Return `None` when `color_name` could not be converted to a [`CString`] or
/// when [`XAllocNamedColor`] failed.
///
/// This function is not a method for [`X11Backend`] and requires the user to
/// explicitly pass the `display` and `xlib` because it is used before
/// [`X11Backend`] is available, i.e. in `X11Backend::new`.
///
/// [`XColor`]: ../x11_dl/xlib/struct.XColor.html
/// [`Colormap`]: ../x11_dl/xlib/type.Colormap.html
/// [`CString`]: https://doc.rust-lang.org/std/ffi/struct.CString.html
/// [`XAllocNamedColor`]: https://tronche.com/gui/x/xlib/color/XAllocNamedColor.html
/// [`X11Backend`]: struct.X11Backend.html
pub fn allocate_color(display: *mut xlib::Display,
                      xlib: &xlib::Xlib,
                      color_name: ColorName,
                      colormap: xlib::Colormap)
                      -> Option<xlib::XColor> {

    if let Some(cstr) = CString::new(color_name).ok() {
        let mut closest = unsafe { zeroed() };
        let mut exact = unsafe { zeroed() };
        let status = unsafe {
            (xlib.XAllocNamedColor)(display, colormap, cstr.as_ptr(), &mut closest, &mut exact)
        };
        if status != 0 {
            return Some(closest);
        }
    }
    None
}


/// Return the `time` field of a *timed* event.
///
/// The following events are *timed*:
///
/// * `KeyPress`
/// * `KeyRelease`
/// * `ButtonPress`
/// * `ButtonRelease`
/// * `EnterNotify`
/// * `LeaveNotify`
/// * `SelectionRequest`
///
/// In case the given `event` is not timed, `None` is returned.
pub fn get_timed_event_time(event: &xlib::XEvent) -> Option<c_ulong> {
    match event.get_type() {
        xlib::KeyPress | xlib::KeyRelease => {
            let xev: xlib::XKeyEvent = From::from(event);
            Some(xev.time)
        }
        xlib::ButtonPress | xlib::ButtonRelease => {
            let xev: xlib::XButtonEvent = From::from(event);
            Some(xev.time)
        }
        xlib::EnterNotify | xlib::LeaveNotify => {
            let xev: xlib::XCrossingEvent = From::from(event);
            Some(xev.time)
        }
        xlib::SelectionRequest => {
            let xev: xlib::XSelectionRequestEvent = From::from(event);
            Some(xev.time)
        }
        _ => None,
    }
}

/// Return the name of the event.
///
/// Use this for debugging/logging.
pub fn event_name(event: &xlib::XEvent) -> &'static str {
    match event.get_type() {
        xlib::KeyPress => "KeyPress",
        xlib::KeyRelease => "KeyRelease",
        xlib::ButtonPress => "ButtonPress",
        xlib::ButtonRelease => "ButtonRelease",
        xlib::MotionNotify => "MotionNotify",
        xlib::EnterNotify => "EnterNotify",
        xlib::LeaveNotify => "LeaveNotify",
        xlib::FocusIn => "FocusIn",
        xlib::FocusOut => "FocusOut",
        xlib::KeymapNotify => "KeymapNotify",
        xlib::Expose => "Expose",
        xlib::GraphicsExpose => "GraphicsExpose",
        xlib::NoExpose => "NoExpose",
        xlib::VisibilityNotify => "VisibilityNotify",
        xlib::CreateNotify => "CreateNotify",
        xlib::DestroyNotify => "DestroyNotify",
        xlib::UnmapNotify => "UnmapNotify",
        xlib::MapNotify => "MapNotify",
        xlib::MapRequest => "MapRequest",
        xlib::ReparentNotify => "ReparentNotify",
        xlib::ConfigureNotify => "ConfigureNotify",
        xlib::ConfigureRequest => "ConfigureRequest",
        xlib::GravityNotify => "GravityNotify",
        xlib::ResizeRequest => "ResizeRequest",
        xlib::CirculateNotify => "CirculateNotify",
        xlib::CirculateRequest => "CirculateRequest",
        xlib::PropertyNotify => "PropertyNotify",
        xlib::SelectionClear => "SelectionClear",
        xlib::SelectionRequest => "SelectionRequest",
        xlib::SelectionNotify => "SelectionNotify",
        xlib::ColormapNotify => "ColormapNotify",
        xlib::ClientMessage => "ClientMessage",
        xlib::MappingNotify => "MappingNotify",
        xlib::GenericEvent => "GenericEvent",
        xlib::LASTEvent => "LASTEvent",
        _ => "Unknown Event",
    }
}

/// Return the `Path` to the window manager state file.
pub fn get_state_file_path() -> &'static Path {
    STATE_FILE_PATH.as_path()
}

/// Serialise the given data as JSON to the given file.
///
/// Return an error when serialising or writing failed.
pub fn serialise_data_to_json_file<T>(path: &Path, data: T) -> X11Result<()>
    where T: Encodable
{
    let mut file = try!(File::create(path));
    let mut s = String::new();
    {
        let mut encoder = Encoder::new_pretty(&mut s);
        try!(data.encode(&mut encoder));
    }
    file.write_all(s.as_ref()).map_err(From::from)
}

/// Deserialise data from the given JSON file.
///
/// Return an error when deserialising or reading failed.
pub fn deserialise_data_from_json_file<T>(path: &Path) -> X11Result<T>
    where T: Decodable
{
    let mut file = try!(File::open(path));
    let json = try!(Json::from_reader(&mut file));
    let mut decoder = Decoder::new(json);
    let data = Decodable::decode(&mut decoder).map_err(From::from);
    data
}

/// Get the name of the executable that started the window manager.
///
/// If the executable was deleted but a new one with the same name has
/// reappeared, e.g., when recompiling, return that.
///
/// Return an error when the executable could not identified, or when it has
/// been deleted but not reappeared.
pub fn get_executable() -> X11Result<PathBuf> {
    let exe = try!(env::current_exe());
    debug!("Identified executable: {}", exe.display());

    // We use an Option here because of borrowing issues with `file_name_str`
    // and `exe`.
    let without_deleted = {
        let msg = "Could not extract the executable file name";
        let file_name_str = try!(exe.file_name()
            .and_then(OsStr::to_str)
            .ok_or(X11Error::msg(msg)));
        // If the executable has been deleted, it was probably recompiled.
        if file_name_str.ends_with(" (deleted)") {
            trace!("Executable has been deleted");
            // Return the executable without " (deleted)" at the end
            Some(exe.with_file_name(file_name_str.trim_right_matches(" (deleted)")))
        } else {
            None
        }
    };
    Ok(without_deleted.unwrap_or(exe))
}

/// Convert a raw string of `wchar_t` to a `String`.
///
/// Return `None` when the string contained invalid data.
pub fn wide_string_to_string(src: *const wchar_t) -> Option<String> {
    let len = unsafe { wcslen(src) };
    let slice: &[wchar_t] = unsafe { slice::from_raw_parts(src, len) };
    let utf16_vec: Vec<u16> = slice.iter().map(|c| *c as u16).collect();
    let utf16: &[u16] = utf16_vec.as_slice();
    String::from_utf16(utf16).ok()
}

/// Return true when the given `Geometry` is valid.
///
/// By *valid* we mean: a width and height greater than 0, but not extremely
/// large, which can lead to X11 errors.
///
/// Don't worry about the definition of extremely large: it is still orders of
/// magnitude larger than a fullscreen window on your triple QHD setup.
pub fn valid_geometry(geometry: &Geometry) -> bool {
    let Geometry { width, height, .. } = *geometry;
    let max = c_uint::max_value() / 2;
    0 < width && width < max && 0 < height && height < max
}

/// If the geometry does not specify a position, center the window on the
/// screen.
///
/// Only if the x- and y-coordinate are 0 will the window be moved.
pub fn center_geometry(window_geometry: &mut Geometry, screen: &Screen) {
    let Geometry { ref mut x, ref mut y, .. } = *window_geometry;

    // Do nothing if the window has a position
    if *x != 0 || *y != 0 {
        return;
    }

    // How much we should shift the window to the right/down to center it. We
    // use min here to prevent negative numbers (= overflow)
    let shift_right = (screen.width - min(screen.width, window_geometry.width)) / 2;
    let shift_down = (screen.height - min(screen.height, window_geometry.height)) / 2;
    *x = shift_right as c_int;
    *y = shift_down as c_int;
}


/// Make sure the `Geometry` respects the given `XSizeHints`.
///
/// See
/// https://tronche.com/gui/x/xlib/ICC/client-to-window-manager/wm-normal-hints.html
/// for more information about size hints.
///
/// The following hints are considered, all others are ignored.
///
/// * The min and maximum size.
/// * The obsolete size hint.
///
/// If the width or height of the window is still < 5 pixels with all hints
/// applied, it is set to 5 pixels to make sure the window is visible.
pub fn respect_hints(geometry: &mut Geometry, hints: &xlib::XSizeHints) {
    trace!("GEOMETRY BEFORE HINTS: {}", geometry);
    trace!("MinSize: {} {}x{}",
           hints.flags & xlib::PMinSize != 0,
           hints.min_width,
           hints.min_height);
    trace!("MaxSize: {} {}x{}",
           hints.flags & xlib::PMaxSize != 0,
           hints.max_width,
           hints.max_height);
    trace!("Size: {} {}x{}",
           hints.flags & xlib::PSize != 0,
           hints.width,
           hints.height);
    trace!("BaseSize: {} {}x{}",
           hints.flags & xlib::PBaseSize != 0,
           hints.base_width,
           hints.base_height);

    // Apply the min size hint
    if hints.flags & xlib::PMinSize != 0 {
        geometry.width = max(geometry.width, hints.min_width as c_uint);
        geometry.height = max(geometry.height, hints.min_height as c_uint);
    }
    // Apply the max size hint
    if hints.flags & xlib::PMaxSize != 0 {
        geometry.width = min(geometry.width, hints.max_width as c_uint);
        geometry.height = min(geometry.height, hints.max_height as c_uint);
    }

    // Apply the obsolete size hint
    if hints.flags & xlib::PSize != 0 {
        if hints.width > 0 {
            geometry.width = hints.width as c_uint;
        }
        if hints.height > 0 {
            geometry.height = hints.height as c_uint;
        }
    }

    // Make sure the height and width are at least 5 pixels.
    geometry.width = max(geometry.width, 5);
    geometry.height = max(geometry.height, 5);

    trace!("GEOMETRY AFTER HINTS: {}", geometry);
}
