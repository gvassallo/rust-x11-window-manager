//! Basic types used throughout the whole project.
//!
//! # C Types
//!
//! Note that instead of `i32`, `u32`, etc. we use `c_int`, `c_uint`, etc. The
//! main reason for using these is that they match the types used by the Xlib
//! library. These are simply type synonyms that match the current operating
//! system's definition of the corresponding C type (which are
//! platform-specific), e.g. `c_int` matches C's `int` type, etc.
//!
//! If your code compiles on Windows but not on Linux or Mac, you are probably
//! using `i32` or `u32` instead of `c_int` or `c_uint`. *To maintain
//! cross-platform compatibility use the `c_*` types*.
//!
//! # Derived Traits
//!
//! As mentioned in its documentation, the struct or enum representing a
//! [`WindowManager`](../wm/trait.WindowManager.html) must implement a number
//! of traits besides the obvious `WindowManager` trait: `Encodable`,
//! `Decodable`, `Debug`, `Clone`. Lucky for you they can be automatically
//! derived using `#[derive(RustcEncodable, ...)]`, but this requires that all
//! types used in the struct or enum also implement these traits. That is why
//! we automatically derive most useful traits for every struct/enum in this
//! module.


use std::fmt;
use std::os::raw::{c_int, c_uint, c_ulong};

/// A window is just defined as an identifier that matches the identifier used
/// by X11.
///
/// In other words, a window is just a number that can be freely copied, no
/// deallocation needs to happen.
pub type Window = c_ulong;

/// The geometry of a window determines its location and size.
///
/// Note that the origin lies in the top-left corner. The X-axis goes from
/// left to right and the Y-axis from top to bottom.
#[derive(Copy, Clone, RustcDecodable, RustcEncodable, Debug, PartialEq, Eq, Hash)]
pub struct Geometry {
    /// X-coordinate of the top-left corner of a window.
    pub x: c_int,
    /// Y-coordinate of the top-left corner of a window.
    pub y: c_int,
    /// The width of the window.
    pub width: c_uint,
    /// The height of the window.
    pub height: c_uint,
}

impl fmt::Display for Geometry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "Geometry {}x{} at {},{}",
               self.width,
               self.height,
               self.x,
               self.y)
    }
}

/// A screen is simply defined by its width and height.
///
/// We assume there is only a single screen at all times, so there is no need
/// to track the position of a screen relative to the origin or another
/// screen.
#[derive(Copy, Clone, RustcDecodable, RustcEncodable, Debug, PartialEq, Eq, Hash)]
pub struct Screen {
    /// The width of the screen.
    pub width: c_uint,
    /// The height of the screen.
    pub height: c_uint,
}

impl fmt::Display for Screen {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Screen {}x{}", self.width, self.height)
    }
}

impl Screen {
    /// Return a `Geometry` based on the given `Screen`.
    ///
    /// The geometry will have the `width` and `height` of the screen, but `x`
    /// and `y` will both be zero.
    pub fn to_geometry(&self) -> Geometry {
        Geometry {
            x: 0,
            y: 0,
            width: self.width,
            height: self.height,
        }
    }
}

/// A type that is either *float* or *tile*.
///
/// Using a simple data type like this instead of a boolean is much clearer
/// and will not lead to confusion.
#[derive(Copy, Clone, RustcDecodable, RustcEncodable, Debug, PartialEq, Eq, Hash)]
pub enum FloatOrTile {
    /// Floating above the tiled windows.
    Float,
    /// Not floating but tiled.
    Tile,
}

/// A `WindowWithInfo` is the combination of a `Window` with additional
/// information: its `Geometry`, whether it should float or not
/// (`float_or_tile`), and whether it should be displayed fullscreen or not
/// (`fullscreen`).
///
/// The `float_or_tile` field will be set to `Float` by the backend when the
/// window is a dialog or popup, otherwise `Tile`. Window managers not
/// implementing [`FloatSupport`](../wm/trait.FloatSupport.html) can safely
/// ignore this.
///
/// Analogously for the `fullscreen` field, it will be set to `true` by the
/// backend when the window wants to be displayed fullscreen, otherwise
/// `false`. Window managers not implementing
/// [`FullscreenSupport`](../wm/trait.FullscreenSupport.html) can safely
/// ignore this.
///
/// This is a separate type used by the `add_window` and `get_window_info`
/// methods of the [`WindowManager`](../wm/trait.WindowManager.html) trait,
/// and will also be useful when defining a window manager data type yourself.
#[derive(Copy, Clone, RustcDecodable, RustcEncodable, Debug, PartialEq, Eq, Hash)]
pub struct WindowWithInfo {
    /// The window.
    pub window: Window,
    /// The geometry of the window.
    pub geometry: Geometry,
    /// Indicate whether the window should float or tile.
    pub float_or_tile: FloatOrTile,
    /// Indicate whether the window should be displayed fullscreen or not.
    pub fullscreen: bool,
}

impl WindowWithInfo {
    /// Create a new tiled non-fullscreen `WindowWithInfo`.
    pub fn new_tiled(window: Window, geometry: Geometry) -> WindowWithInfo {
        Self::new(window, geometry, FloatOrTile::Tile, false)
    }
    /// Create a new floating non-fullscreen `WindowWithInfo`.
    pub fn new_float(window: Window, geometry: Geometry) -> WindowWithInfo {
        Self::new(window, geometry, FloatOrTile::Float, false)
    }
    /// Create a new fullscreen `WindowWithInfo`.
    ///
    /// The `float_or_tile` field is set to `Tile`.
    pub fn new_fullscreen(window: Window, geometry: Geometry) -> WindowWithInfo {
        Self::new(window, geometry, FloatOrTile::Tile, true)
    }
    /// Create a new `WindowWithInfo` with the given arguments.
    pub fn new(window: Window,
               geometry: Geometry,
               float_or_tile: FloatOrTile,
               fullscreen: bool)
               -> WindowWithInfo {
        WindowWithInfo {
            window: window,
            geometry: geometry,
            float_or_tile: float_or_tile,
            fullscreen: fullscreen,
        }
    }
}

/// As explained in the assignment, the `WindowLayout` struct fully describes
/// the layout of the windows that the window manager should display.
///
/// This data structure is passed to the backend to make sure the display
/// server actually displays the desired window layout of the window manager.
#[derive(Clone, RustcDecodable, RustcEncodable, Debug, PartialEq, Eq, Hash)]
pub struct WindowLayout {
    /// The focused window or `None` in which case no window is focused.
    ///
    /// **Invariant**: if `focused_window = Some(w)`, `w` must be in
    /// `windows`.
    pub focused_window: Option<Window>,
    /// A `Vec` of all the *visible* windows along with their geometry.
    ///
    /// The order of this `Vec` determines the stacking order of the windows:
    /// the first window is the bottom window, the last window is the top
    /// window, see the ASCII figure below.
    ///
    /// ```
    /// +-------+
    /// | last  |-----+
    /// +-------+     |
    ///       | first |
    ///       |       |
    ///       +-------+
    /// ```
    ///
    /// This main reason for choosing this order is that it is easier to
    /// append an element to the end of a vector than it is to prepend one.
    pub windows: Vec<(Window, Geometry)>,
}

impl WindowLayout {
    /// Construct a new, empty window layout without any windows and with
    /// nothing focused.
    pub fn new() -> WindowLayout {
        WindowLayout {
            focused_window: None,
            windows: Vec::new(),
        }
    }
}

/// A type that is either *previous* or *next*.
///
/// Using a simple data type like this instead of a boolean is much clearer
/// and will not lead to confusion.
#[derive(Copy, Clone, RustcDecodable, RustcEncodable, Debug, PartialEq, Eq, Hash)]
pub enum PrevOrNext {
    /// Previous
    Prev,
    /// Next
    Next,
}

impl PrevOrNext {
    /// Return the opposite of the given direction, i.e. the opposite of
    /// `Prev` is `Next` and vice versa.
    pub fn opposite(&self) -> Self {
        use self::PrevOrNext::*;
        match *self {
            Prev => Next,
            Next => Prev,
        }
    }
}

/// The size of a gap.
///
/// Note that a gap cannot be negative.
pub type GapSize = c_uint;

/// The type of a workspace index.
///
/// Used by the
/// [`MultiWorkspaceSupport`](../wm/trait.MultiWorkspaceSupport.html) as
/// indices for workspaces.
pub type WorkspaceIndex = usize;

/// The highest `WorkspaceIndex`.
///
/// As this is an index (starting from 0), this means there will be
/// `MAX_WORKSPACE_INDEX + 1` workspaces.
pub static MAX_WORKSPACE_INDEX: WorkspaceIndex = 3;
