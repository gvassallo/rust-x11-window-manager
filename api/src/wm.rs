//! Window manager functionality defined as traits
//!
//! This module defines a number of traits, each corresponding to an
//! assignment. Every window manager must at least implement the
//! `WindowManager` trait.
//!
//! # Interaction Between Traits
//!
//! When writing a window manager that implements multiple traits, e.g.,
//! [`TilingSupport`] and [`FloatSupport`], take care of the interaction
//! between the traits. For example, when calling `swap_with_master` (from the
//! [`TilingSupport`] trait) with a window that is currently floating
//! ([`FloatSupport`]), a possible thing to do is to tile the window first and
//! then swap it with the master window. Or when calling `toggle_floating`
//! with a minimised window ([`MinimiseSupport`]), unminimise it first before
//! floating it. Try to make reasonable and consistent choices when it comes
//! to these edge cases (keep the listed invariants in mind). Document them
//! and write test cases for them.
//!
//! [`TilingSupport`]: trait.TilingSupport.html
//! [`FloatSupport`]: trait.FloatSupport.html
//! [`MinimiseSupport`]: trait.MinimiseSupport.html

use rustc_serialize::{Decodable, Encodable};
use std::error;
use std::fmt::Debug;

use types::{GapSize, Geometry, PrevOrNext, Screen, Window, WindowLayout, WindowWithInfo,
            WorkspaceIndex};

/// A basic window manager.
///
/// Every window manager defined by a struct or enum representing its state,
/// must implement this trait.
///
/// # Supertraits
///
/// Note what comes after `WindowManager:`, `Encodable + Decodable + Debug +
/// Clone`. These are the “supertraits” of `WindowManager`: to implement this
/// trait, these four other traits must be implemented as well.
///
/// ## Encodable and Decodable
///
/// The first two, `Encodable` and `Decodable`, indicate that it should be
/// possible to serialise and deserialise a window manager. This functionality
/// is used in the backends, which allow reloading a live window manager
/// without losing its state. This is useful during development: one can
/// easily switch to a newly compiled version of the window manager without
/// losing all the state. Before reloading, the whole window manager is
/// serialised to JSON which is written to `/tmp`. The window manager is then
/// restarted from scratch. During startup, the window manager tries to read
/// this JSON, deserialises it and uses the result as the window manager,
/// including all the state instead of a new empty window manager. Windows
/// that have been added or removed since the state was saved will be added
/// and removed to/from the restored window manager.
///
/// All this is taken care of in the backend, so the only thing you have to do
/// is make sure your window manager is `Encodable` and `Decodable`. Lucky for
/// you, this traits can be derived *automatically* using the `derive`
/// annotation.
///
/// For example:
///
/// ```
/// #[derive(RustcEncodable, RustcDecodable, ...)]
/// pub struct MyWM {
///     ...
/// }
/// ```
///
/// In the example, a struct `MyWM` with some fields is defined. The `derive`
/// annotation on the struct will implement both interfaces automatically for
/// you. In order for this to work, every field of the struct must implement
/// these interfaces too. Most basic types available in Rust (`i32`, `Vec`,
/// ...) already implement this, but if you define additional structs, you
/// also have to put the same annotation on them.
///
/// Note that you can also implement these traits manually, but this will be a
/// lot more work.
///
/// ## Debug
///
/// The `Debug` trait is used to convert a window manager to a string
/// representation to print it during development, in test output or when
/// tracing.
///
/// This trait can also be derived automatically using `#[derive(Debug)]`.
///
/// ## Clone
///
/// The `Clone` is used to make a clone of a window manager.
///
/// This trait can also be derived automatically using `#[derive(Clone)]`.
///
/// ## In Summary
///
/// The example below demonstrates that you can implement these four traits
/// using one line of code:
///
/// ```
/// #[derive(RustcEncodable, RustcDecodable, Debug, Clone)]
/// pub struct MyWM {
///     ...
/// }
/// ```
///
/// You also have to make sure that every type you use in the definition of
/// your window manager struct (or enum) also implements these four traits.
pub trait WindowManager: Encodable + Decodable + Debug + Clone {
    /// The type of the errors of the window manager.
    ///
    /// You can define your own enum with different types of errors you window
    /// manager can return. Many functions of this trait will return a
    /// `Result<(), Self::Error>`: either an `Ok(())`, meaning that everything
    /// went ok, or an `Err(err)` where `err` is of this `Error` type, meaning
    /// that an error occurred.
    ///
    /// Note the two “supertraits” your error type must implement:
    /// [`error::Error`](https://doc.rust-lang.org/std/error/trait.Error.html)
    /// and `'static`. The first is the base trait for all Rust errors, click
    /// on it to see the methods you must implement. This trait also has three
    /// “supertraits”:
    ///
    /// * `Debug`: a `toString`-like method for debugging that can be derived
    ///   automatically using `#[derive(Debug, ...)]`.
    /// * `Display`: a `toString`-like method that will be used to show error
    ///   to users (as opposed to developers). This method has to be manually
    ///   implemented.
    /// * `Reflect`: you can safely ignore this.
    ///
    /// The second trait is not an actual trait, but a constraint that says
    /// that your error must be static and should not contain any references
    /// with a limited
    /// [lifetime](https://doc.rust-lang.org/book/lifetimes.html). In other
    /// words, your error type may not contain any type `&'a T`.
    ///
    /// # Examples
    /// ```
    /// /// Error type for my window manager
    /// #[derive(Debug)]  // Implement `Debug` automatically
    /// pub enum MyWMError {
    ///     /// This window is not known by the window.
    ///     UnknownWindow(Window),
    ///     // Feel free to add more
    ///     ...
    /// }
    /// // Manually implement the `Display` trait.
    /// use std::fmt;
    /// impl fmt::Display for MyWMError {
    ///     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    ///         match *self {
    ///             MyWMError::UnknownWindow(ref window) =>
    ///                 write!(f, "Unknown window: {}", window),
    ///             ...
    ///         }
    ///     }
    /// }
    /// // Now implement the `Error` trait.
    /// use std::error;
    /// impl error::Error for MyWMError {
    ///     fn description(&self) -> &'static str {
    ///         match *self {
    ///             // Because we must return a `&'static str`, we can't
    ///             // include the window in it, because that creates a
    ///             // `String`.
    ///             MyWMError::UnknownWindow(_) => "Unknown window",
    ///         }
    ///     }
    /// }
    /// // We can now use `MyWMError` as the `Error` type of the `MyWM` window
    /// // manager.
    /// impl WindowManager for MyWM {
    ///     type Error = MyWMError;
    ///     ...
    /// }
    /// ```
    ///
    /// Note that you can use the same error type in multiple window managers.
    type Error: error::Error + 'static;

    /// The constructor of a window manager.
    ///
    /// The sole argument is the `Screen`, useful to determine the size of
    /// fullscreen windows or to center a window.
    fn new(screen: Screen) -> Self;

    /// Return the current desired window layout.
    ///
    /// For more information about this, see the assignment and the
    /// documentation for [`WindowLayout`](../types/struct.WindowLayout.html).
    fn get_window_layout(&self) -> WindowLayout;

    /// Check whether a window is managed by this window manager.
    ///
    /// **Invariant**: `is_managed(w) == true` for some window `w` iff the
    /// vector returned by the `get_windows` method contains `w`.
    ///
    /// A default implementation is provided in terms of `get_windows`.
    /// Override this implementation if you have a more efficient one.
    fn is_managed(&self, window: Window) -> bool {
        self.get_windows().contains(&window)
    }

    /// Return a vector of all the windows managed by the window manager,
    /// visible or not.
    ///
    /// The order of the windows in the vector does not matter.
    ///
    /// **Invariant**: `get_windows()` must not contain duplicates.
    fn get_windows(&self) -> Vec<Window>;

    /// Return the window that is currently focused according to the window
    /// manager.
    ///
    /// If no window should be focused, return `None`.
    ///
    /// **Invariant**: `get_focused_window() ==
    /// get_window_layout().focused_window`.
    ///
    /// **Invariant**: `get_focused_window() == Some(w)` then `is_managed(w)
    /// == true`.
    ///
    /// A default implementation is provided in terms of `get_window_layout`
    /// and its `focused_window` field. Override this implementation if you
    /// have a more efficient one.
    fn get_focused_window(&self) -> Option<Window> {
        self.get_window_layout().focused_window
    }

    /// Add a new window along with its information, e.g. its `Geometry`.
    ///
    /// This is called whenever a new window is created.
    ///
    /// **Invariant**: `is_managed` must return true for the given window
    /// after `add_window` was called with the given window.
    ///
    /// **Invariant**: after adding a window using `add_window`, it must be
    /// focused according to `get_focused_window`.
    ///
    /// A window manager that implements
    /// [`FloatSupport`](trait.FloatSupport.html) should float the window when
    /// the `float_or_tile` field of `WindowWithInfo` is set to `Float`.
    ///
    /// A window manager that implements
    /// [`FullscreenSupport`](trait.FullscreenSupport.html) should make the
    /// window fullscreen when the `fullscreen` field of `WindowWithInfo` is
    /// set to `true`.
    ///
    /// This function is *allowed* to return an appropriate error when the
    /// window is already managed by the window manager.
    fn add_window(&mut self, window_with_info: WindowWithInfo) -> Result<(), Self::Error>;

    /// Remove the given window from the window manager.
    ///
    /// This is called whenever a window is destroyed/killed.
    ///
    /// The same window remains focused, unless the focused window has been
    /// removed.
    ///
    /// **Invariant**: `is_managed` must return false for the given window
    /// after `remove_window` was called with the given window.
    ///
    /// This function is *allowed* to return an appropriate error when the
    /// window is not managed by the window manager.
    fn remove_window(&mut self, window: Window) -> Result<(), Self::Error>;

    /// Focus the given window, or when passed `None`, focus nothing.
    ///
    /// This is called when the user clicks or hovers on(to) a window, or
    /// changes the focus using the keyboard.
    ///
    /// **Invariant**: when `focus_window` succeeds, `get_focused_window` must
    /// return the same argument.
    ///
    /// This function is *allowed* to return an appropriate error when the
    /// window is not managed by the window manager.
    fn focus_window(&mut self, window: Option<Window>) -> Result<(), Self::Error>;

    /// Focus the previous or next window.
    ///
    /// Do nothing when there are no windows. When there is only one window,
    /// focus it if currently no window is focused, otherwise do nothing.
    ///
    /// When no window is focused, any window may become focused.
    ///
    /// Cycling the focus back and forth shouldn't change the focused window.
    fn cycle_focus(&mut self, dir: PrevOrNext);

    /// Get the info (`WindowWithInfo`) belonging to the given window. It
    /// should reflect the current state (location/size, floating or tiled,
    /// fullscreen or not) of the window.
    ///
    /// This method will come in handy when implementing later assignments by
    /// defining wrappers around existing window managers.
    ///
    /// This function *should* return an appropriate error when the window is
    /// not managed by the window manager.
    fn get_window_info(&self, window: Window) -> Result<WindowWithInfo, Self::Error>;

    /// Return the screen managed by the window manager.
    fn get_screen(&self) -> Screen;

    /// Resize the screen according to the given `Screen`.
    ///
    /// This is called whenever the resolution of the screen is changed.
    ///
    /// Note that we do not support multiple monitors.
    ///
    /// **Invariant**: after `resize_screen` is called with a screen,
    /// `get_screen()` must return the same screen.
    fn resize_screen(&mut self, screen: Screen);
}

/// A window manager that supports *tiling*.
///
/// The idea of *tiling* is explained in the assignment. While most tiling
/// window managers have many possible layouts to (dynamically) choose from,
/// we limit ourselves to one simple layout here. In an optional assignment
/// you will be asked to implement a different layout algorithm.
///
/// # Layout algorithm
///
/// This window manager follows the following the tiling layout: we start out
/// with no windows. When the first window is added, the window is maximised
/// as in the ASCII diagram below.
///
/// ```
/// +---------------------+
/// |                     |
/// |                     |
/// |          1          |
/// |                     |
/// |                     |
/// +---------------------+
/// ```
///
/// When a second window is added, the screen is split in two tiles: a left
/// tile for window 1 and a right tile for window 2.
///
/// ```
/// +----------+----------+
/// |          |          |
/// |          |          |
/// |    1     |    2     |
/// |          |          |
/// |          |          |
/// +----------+----------+
/// ```
///
/// When a third window is added, the right tile will split in two tiles: a
/// top tile for window 2 and a bottom tile for window 3.
///
/// ```
/// +---------------------+
/// |          |          |
/// |          |    2     |
/// |    1     +----------+
/// |          |          |
/// |          |    3     |
/// +----------+----------+
/// ```
///
/// The left tile will never be split, we call this the *master tile*. The
/// user typically places his/her main application in this tile, e.g., the
/// browser or editor. Additional windows, e.g. a terminal, a file manager, or
/// a chat window are displayed in the side tiles. Note that even when the
/// master tile is focused, new windows will not appear in the master tile,
/// but in a new side tile.
///
/// When a fourth window is added, an additional tile is created on the right
/// side and the new window is placed in the bottom tile.
///
/// ```
/// +----------+----------+
/// |          |    2     |
/// |          +----------+
/// |    1     |    3     |
/// |          +----------+
/// |          |    4     |
/// +----------+----------+
/// ```
///
/// When window 2, 3, or 4 is closed, the corresponding tile disappears and we
/// go back to the previous layout. When window 1 is closed, the first side
/// window (2) is chosen to be displayed in the master tile. The promoted
/// window's previous tile disappears.
///
/// **Invariant**: at all times there must be as many tiles as there are
/// visible windows. Note that this will not hold when there are floating
/// ([`FloatSupport`](trait.FloatSupport.html)) or minimised windows
/// ([`MinimiseSupport`](trait.MinimiseSupport.html)).
///
/// # How to implement it
///
/// This trait contains some useful methods to move around windows. It is not
/// sufficient to implement these methods, the actual tiling logic happens
/// when windows are added or removed, and in `get_window_layout`.
/// Consequently, you must change the appropriate implementations of the
/// methods of the `WindowManager` trait to take care of tiling.
pub trait TilingSupport: WindowManager {
    /// Return the window displayed in the master tile.
    ///
    /// If there are no windows, return `None`.
    ///
    /// **Invariant**: `get_master_window() == Some(w)`, then `w` must occur
    /// in the vector returned by `get_windows()`.
    ///
    /// **Invariant**: if the vector returned by `get_windows()` is empty =>
    /// `get_master_window() == None`. The other direction of the arrow must
    /// not hold, e.g., there could floating windows (see `FloatSupport`), but
    /// no tiled windows.
    ///
    fn get_master_window(&self) -> Option<Window>;

    /// Swap the given window with the window in the master tile.
    ///
    /// After the function has succeeded, the master window should be focused.
    ///
    /// If the given window is already in the master tile, no windows have to
    /// be swapped, but the master window should be focused.
    ///
    ///
    /// **Invariant**: if `swap_with_master(w)` succeeds, `get_master_window()
    /// == Some(w)`.
    ///
    /// This function is *allowed* to return an appropriate error when the
    /// window is not managed by the window manager.
    fn swap_with_master(&mut self, window: Window) -> Result<(), Self::Error>;

    /// Swap the focused window with the one in the next or previous tile.
    ///
    /// Do nothing when there are no windows, when there is only one window,
    /// or when no window is focused.
    ///
    /// If there were two tiles and the swap happened, the same window will be
    /// focused, but the other tile will be focused.
    ///
    /// **Invariant**: calling `swap_windows(dir)` for any `dir` will not
    /// change the focused window, even if no window was focused.
    ///
    /// **Invariant**: calling `swap_windows(dir)` and then
    /// `swap_windows(dir.opposite())` will not change the window layout.
    fn swap_windows(&mut self, dir: PrevOrNext);
}

/// A window manager that supports floating windows.
///
/// Some windows are not suitable for *tiling*, e.g. dialogs, popups, video,
/// etc. Instead of tiling them, they should *float* above the tiled windows.
/// Floating windows can be moved around and resized with the mouse.
///
/// The backend will detect that some windows should float from the start and
/// will set the `float_or_tile` field of `WindowWithInfo` to `Float` in these
/// cases. A window manager implementing this trait should implement the
/// `add_window` method of `WindowManager` such that these windows will float
/// after adding them.
///
/// The user can choose to *float* any tiled window or to *sink* (make it
/// tiled again) any floating window.
///
/// **Invariant**: every floating window `w` (`is_floating(w) == true`) must
/// be placed above the tiled windows in the window layout returned by
/// `get_window_layout()`.
pub trait FloatSupport: WindowManager {
    /// Return a vector of all the visible floating windows.
    ///
    /// The order of the windows in the vector does not matter.
    fn get_floating_windows(&self) -> Vec<Window>;

    /// Return true if the given window is floating.
    ///
    /// This function must always return false when the given window is not
    /// managed by the window manager.
    ///
    /// **Invariant**: if `is_floating(w) == true` for some window `w`, then
    /// `is_managed(w) == true`.
    ///
    /// **Invariant**: `is_floating(w) == true` for some window `w`, iff the
    /// vector returned by the `get_floating_windows` method contains `w`.
    ///
    /// A default implementation is provided in terms of
    /// `get_floating_windows()`. Override this implementation if you have a
    /// more efficient one.
    fn is_floating(&self, window: Window) -> bool {
        self.get_floating_windows().contains(&window)
    }

    /// If the given window is floating, let it *sink*, if it is not floating,
    /// let it *float*.
    ///
    /// When a non-floating window starts to float, its original geometry
    /// (passed to `add_window`) should be restored.
    ///
    /// **Invariant**: if calling `toggle_floating(w)` with a tiled window `w`
    /// succeeds, `is_floating(w)` must return `true`.
    ///
    /// **Invariant**: if calling `toggle_floating(w)` with a floating window
    /// `w` succeeds, `is_floating(w)` must return `false`.
    ///
    /// **Invariant**: the result of `is_floating(w)` must be the same before
    /// and after calling `toggle_floating(w)` twice.
    ///
    /// This function is *allowed* to return an appropriate error when the
    /// window is not managed by the window manager.
    fn toggle_floating(&mut self, window: Window) -> Result<(), Self::Error>;

    /// Resize/move the given floating window according to the given geometry.
    ///
    /// This function is called when the user moves or resizes a window using
    /// the mouse, but can also be called by custom user commands.
    ///
    /// The window layout should reflect the geometry change of the floating
    /// window.
    ///
    /// This function is *allowed* to return an appropriate error when the
    /// window is not managed by the window manager *or* when the window is
    /// not floating.
    fn set_window_geometry(&mut self,
                           window: Window,
                           new_geometry: Geometry)
                           -> Result<(), Self::Error>;
}

/// A window manager that supports (un)minimising windows.
///
/// Remember that a tiling window manager displays each window in a different
/// tile. So when you have opened ten applications, the screen will be split
/// in ten tiles. Sometimes you want to open an application without having to
/// look at it the whole time
/// ([`MultiWorkspaceSupport`](trait.MultiWorkspaceSupport.html) is also
/// useful for this scenario). In that case, *minimising* the window is the
/// solution: you can temporarily hide the window and regain the screen space
/// without having to close the window.
///
/// Naturally, there is also a way to reveal these windows again by
/// *unminimising* them.
///
/// **Note**: methods of other traits like `focus_window`, `focus_window`,
/// `toggle_fullscreen`, ... called with a minimised window as argument should
/// first unminimise the window.
///
/// **Hint**: you can use `remove_window` and `add_window` to hide and reveal
/// windows.
pub trait MinimiseSupport: WindowManager {
    /// Return a vector of all the minimised windows.
    ///
    /// The order of the windows in the vector *does* matter.
    ///
    /// The windows must occur in the order they were minimised: the window
    /// that was minimised first must occur first in the vector, the window
    /// that was minimised last must occur last. This makes it easy to define
    /// a function that unminimises the last minimised window.
    fn get_minimised_windows(&self) -> Vec<Window>;

    /// Return `true` if the given window is minimised.
    ///
    /// This function must always return false when the given window is not
    /// managed by the window manager.
    ///
    /// **Invariant**: if `is_minimised(w) == true` for some window `w`, then
    /// `is_managed(w) == true`.
    ///
    /// **Invariant**: `is_minimised(w) == true` for some window `w`, iff the
    /// vector returned by the `get_minised_windows` method contains `w`.
    ///
    /// A default implementation is provided in terms of
    /// `get_minimised_windows()`. Override this implementation if you have a
    /// more efficient one.
    fn is_minimised(&self, window: Window) -> bool {
        self.get_minimised_windows().contains(&window)
    }

    /// Minimise the given window, or when it is already minimised, unminise
    /// it.
    ///
    /// When a minimised floating window is unminimised, it should float again
    /// and have the same geometry as before. Hint: you could use the
    /// `float_or_tile` field of `WindowWithInfo`. Analogously for fullscreen
    /// windows.
    ///
    /// **Invariant**: if calling `toggle_minimised(w)` with an unminimised
    /// window `w` succeeds, `w` may no longer be visible according to
    /// `get_window_layout` and `is_minimised(w)` must return `true`.
    ///
    /// **Invariant**: if calling `toggle_minimised(w)` with an already
    /// minimised window `w` succeeds, `w` must be visible according to
    /// `get_window_layout` and `is_minimised(w)` must return `false`.
    ///
    /// The window layout before and after minimising and directly
    /// unminimising the currently focused window should be the same. This
    /// cannot hold for a window manager that implements
    /// [`TilingSupport`](trait.TilingSupport.html). Try to figure out why.
    fn toggle_minimised(&mut self, window: Window) -> Result<(), Self::Error>;
}

/// A window manager that supports fullscreen windows.
///
/// Users wishing to watch a video fullscreen, to play a game fullscreen, or
/// to view any application fullscreen, can make any window become fullscreen
/// using this trait.
///
/// There can at most one window be fullscreen at a time.
///
/// The backend will detect that some windows want to be fullscreen from the
/// start and will set the `fullscreen` field of `WindowWithInfo` to `true` in
/// these cases. A window manager implementing this trait should implement the
/// `add_window` method of `WindowManager` such that these windows will be
/// fullscreen after adding them.
///
/// A user can make any window fullscreen by using the `toggle_fullscreen`.
///
/// Think carefully about the interaction between a fullscreen window and the
/// other traits: when an action is performed on a fullscreen window, should
/// it stop being fullscreen? What when an action is performed on another
/// window which is not the fullscreen one? In some cases it might make sense
/// to keep the window fullscreen (e.g., when another window is removed), in
/// some cases maybe not (e.g., when a non-fullscreen window is added). Try to
/// make reasonable and consistent choices (keep the invariants of this *and*
/// the other traits in mind). Document the choices and write test cases for
/// them.
pub trait FullscreenSupport: WindowManager {
    /// Return the current fullscreen, if any.
    ///
    /// **Invariant**: if `get_fullscreen_window() == Some(w)`, then
    /// `is_managed(w) == true`.
    ///
    /// **Invariant**: if `get_fullscreen_window() == Some(w)`, then
    /// `get_focused_window() == Some(w)`.
    fn get_fullscreen_window(&self) -> Option<Window>;

    /// Make the given window fullscreen, or when it is already fullscreen,
    /// undo it.
    ///
    /// When called on a window that is already fullscreen, it should restore
    /// the window to the state before, e.g. float at the same place.
    /// **Hint**: you could use the `float_or_tile` field of `WindowWithInfo`.
    ///
    /// **Invariant**: if calling `toggle_fullscreen(w)` with a window `w`
    /// that is not yet fullscreen, `w` should be the only visible window
    /// according to `get_window_layout`, its geometry should be the same size
    /// as the screen, and `get_fullscreen_window(w) == Some(w)`.
    ///
    /// The window layout before and after calling `toggle_fullscreen` twice
    /// with the currently focused should be the same. This cannot hold for a
    /// window manager that implements
    /// [`TilingSupport`](trait.TilingSupport.html). Try to figure out why.
    fn toggle_fullscreen(&mut self, window: Window) -> Result<(), Self::Error>;
}

/// A window manager that supports gaps between tiles.
///
/// The user can configure the gap size at run-time. The gaps are only shown
/// between tiled windows. Floating windows and the fullscreen window are
/// unaffected. Initially the gap is 0.
///
/// For example the user has the following tiles:
///
/// ```
/// +----------+----------+
/// |          |          |
/// |          |          |
/// |    1     |    2     |
/// |          |          |
/// |          |          |
/// +----------+----------+
/// ```
///
/// When the gap is set to 5, it is as if every window gets an invisible 5
/// pixel border of empty space. Windows don't share the gap in between them.
/// Every tiled window is moved 5 pixels down and to 5 pixels to the right.
/// The width and height of every tiled window is also shrunk by 10 pixels.
///
/// ```
/// +---------------------+
/// |+--------+ +--------+|
/// ||        | |        ||
/// ||    1   | |   2    ||
/// ||        | |        ||
/// |+--------+ +--------+|
/// +---------------------+
/// ```
///
/// Even when there is only a single tile there should be a gap around it.
///
/// You may ignore scenarios in which the gap size is so large that one of the
/// windows might become invisible.
///
/// Implementors of this trait must adapt their implementation of the
/// `get_window_layout()` method accordingly.
pub trait GapSupport: WindowManager {
    /// Return the current gap size.
    ///
    /// Initially 0.
    fn get_gap(&self) -> GapSize;

    /// Set the gap size.
    ///
    /// **Invariant**: after setting `set_gap(g)` with some gap size `g`,
    /// `get_gap() == g`.
    fn set_gap(&mut self, GapSize);
}


/// A window manager that has multiple workspaces.
///
/// For a small introduction to workspaces, see the first section of the
/// assignment.
///
/// The idea is that there are a number of different workspaces
/// ([`MAX_WORKSPACE_INDEX`] + 1), each represented by a different copy of a
/// single type of window manager. Initially the first (index 0) workspace is
/// active. The user interacts with this workspace's window manager, e.g.,
/// adding/removing windows, focusing a window, etc. When the user then
/// switches to another workspace, all the windows will be hidden. The user
/// can then interact with this workspace as if it is a new one. When the user
/// switches back to the first workspace, the windows previously opened in
/// this workspace are shown again. When the user switches back to the other
/// workspace, the windows previously opened in this workspace are shown
/// again.
///
/// [`MAX_WORKSPACE_INDEX`]: ../types/static.MAX_WORKSPACE_INDEX.html
///
/// This is useful when multitasking: one workspace is for surfing the web,
/// another for working on a CPL assignment, a third one for working on your
/// thesis, etc.
///
/// Think carefully about the interaction between this trait and the others.
/// Most actions must only be executed on the current workspace, e.g.,
/// `add_window`, but some must be executed on all workspaces, e.g., `set_gap`
/// or `resize_screen`. Even the getters are non-obvious: should `get_windows`
/// return the windows of all workspaces or only the current one? What about
/// `get_floating_windows`? Try to make reasonable and consistent choices
/// (keep the invariants of this *and* the other traits in mind). Document the
/// choices and write test cases for them.
pub trait MultiWorkspaceSupport<WM: WindowManager>: WindowManager {
    /// Return the current workspace index.
    ///
    /// When creating a new workspace this will return 0.
    ///
    /// **Invariant**: `0 <= get_current_workspace_index() <=
    /// MAX_WORKSPACE_INDEX`.
    fn get_current_workspace_index(&self) -> WorkspaceIndex;

    /// Get an immutable borrow of the workspace at the given index.
    ///
    /// This function *should* return an appropriate error when `0 <= index <=
    /// MAX_WORKSPACE_INDEX` is not true.
    fn get_workspace(&self, index: WorkspaceIndex) -> Result<&WM, Self::Error>;

    /// Get a mutable borrow of the workspace at the given index.
    ///
    /// This function *should* return an appropriate error when `0 <= index <=
    /// MAX_WORKSPACE_INDEX` is not true.
    fn get_workspace_mut(&mut self, index: WorkspaceIndex) -> Result<&mut WM, Self::Error>;

    /// Switch to the workspace at the given index.
    ///
    /// If `index == get_current_workspace_index()`, do nothing.
    ///
    /// **Invariant**: the window layout after switching to another workspace
    /// and then switching back to the original workspace should be the same
    /// as before.
    ///
    /// This function *should* return an appropriate error when `0 <= index <=
    /// MAX_WORKSPACE_INDEX` is not true.
    fn switch_workspace(&mut self, index: WorkspaceIndex) -> Result<(), Self::Error>;
}
