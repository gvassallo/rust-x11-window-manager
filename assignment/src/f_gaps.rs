//! Optional: Gaps
//!
//! Extend your window manager with support for gaps, i.e. the ability to add
//! some space between the different tiles. See the documentation of the
//! [`GapSupport`] trait for the precise requirements.
//!
//! Make a copy of your tiling window manager from assignment B and let it
//! implement the [`GapSupport`] trait. You are not required to let this
//! window manager implement all the previous traits.
//!
//! [`GapSupport`]: ../../cplwm_api/wm/trait.GapSupport.html
//!
//! # Status
//!
//! COMPLETED: YES
//!
//! COMMENTS:
//!
//! ...
//!


use cplwm_api::types::*;
use cplwm_api::wm::{GapSupport, TilingSupport, WindowManager};
use b_tiling_wm::TilingWM;
use wm_error::WMError;
/// The name of the Window Manger
pub type WMName = GapsWM;

/// Window Manager that supports gaps
#[derive(RustcDecodable, RustcEncodable, Debug, Clone)]
pub struct GapsWM {
    /// A wrapper of the Fullscreen Windows Window Manager
    pub tiling_wm: TilingWM,
    /// The value of the gap, initially 0
    pub gap: GapSize,
}

impl WindowManager for GapsWM {
    type Error = WMError;

    /// The constructor wraps the constructor of the FloatingWM
    /// it also declares the fullscreen_window Option
    fn new(screen: Screen) -> GapsWM {
        GapsWM {
            tiling_wm: TilingWM::new(screen),
            gap: (0 as GapSize),
        }
    }

    /// The function wraps the old `get_windows`
    fn get_windows(&self) -> Vec<Window> {
        self.tiling_wm.get_windows()
    }

    /// The function wraps the old `get_focused_window`
    fn get_focused_window(&self) -> Option<Window> {
        self.tiling_wm.get_focused_window()
    }

    /// The function return an error if the window is already managed
    fn add_window(&mut self, window_with_info: WindowWithInfo) -> Result<(), Self::Error> {
        self.tiling_wm.add_window(window_with_info)
    }

    /// Remove the given window from the window manager.
    fn remove_window(&mut self, window: Window) -> Result<(), Self::Error> {
        self.tiling_wm.remove_window(window)
    }

    /// Return the current window layout.
    ///
    /// This is the only function that changes, if there is no gap return the layout from the
    /// wrapped WM.
    /// Otherwise add the gap to the x and y coordinates of each window geometry and remove gap*2
    /// from the width and height of each window geometry.
    fn get_window_layout(&self) -> WindowLayout {
        let mut layout = self.tiling_wm.get_window_layout();
        if layout.windows.len() > 0 && self.gap > 0 {
            for i in 0..layout.windows.len() {
                layout.windows[i].1.x += self.gap as i32;
                layout.windows[i].1.y += self.gap as i32;
                layout.windows[i].1.width -= (self.gap * 2) as u32;
                layout.windows[i].1.height -= (self.gap * 2) as u32;
            }
            return layout;
        }
        layout
    }

    /// Focus the given window, or when passed None, focus nothing.
    fn focus_window(&mut self, window: Option<Window>) -> Result<(), Self::Error> {
        self.tiling_wm.focus_window(window)
    }

    /// Focus the previous or next window.
    fn cycle_focus(&mut self, dir: PrevOrNext) {
        self.tiling_wm.cycle_focus(dir)
    }

    /// Get the info (WindowWithInfo) belonging to the given window.
    fn get_window_info(&self, window: Window) -> Result<WindowWithInfo, Self::Error> {
        self.tiling_wm.get_window_info(window)
    }

    /// Return the screen managed by the window manager.
    fn get_screen(&self) -> Screen {
        self.tiling_wm.get_screen()
    }

    /// Resize the screen according to the given Screen.
    fn resize_screen(&mut self, screen: Screen) {
        self.tiling_wm.resize_screen(screen)
    }
}

impl TilingSupport for GapsWM {
    /// Return the window displayed in the master tile.
    fn get_master_window(&self) -> Option<Window> {
        self.tiling_wm.get_master_window()
    }

    /// Swap the given window with the window in the master tile.
    fn swap_with_master(&mut self, window: Window) -> Result<(), Self::Error> {
        self.tiling_wm.swap_with_master(window)
    }

    /// Swap the focused window with the one in the next or previous tile.
    fn swap_windows(&mut self, dir: PrevOrNext) {
        self.tiling_wm.swap_windows(dir)
    }
}

impl GapSupport for GapsWM {
    /// Return the current gap size. Initially 0.
    fn get_gap(&self) -> GapSize {
        self.gap
    }
    /// Set the gap size.
    fn set_gap(&mut self, gapsize: GapSize) {
        self.gap = gapsize;
    }
}

#[cfg(test)]
mod tests {

    use super::WMName;
    use cplwm_api::wm::{GapSupport, TilingSupport, WindowManager};
    use cplwm_api::types::*;

    // We define a static variable for the screen we will use in the tests.
    // You can just as well define it as a local variable in your tests.
    static SCREEN: Screen = Screen {
        width: 800,
        height: 600,
    };
    // We define a new Screen to resize the firs one.
    static RESIZED: Screen = Screen {
        width: 1024,
        height: 768,
    };

    // We define a static variable for the geometry of a fullscreen window.
    // Note that it matches the dimensions of `SCREEN`.
    static SCREEN_GEOM: Geometry = Geometry {
        x: 0,
        y: 0,
        width: 800,
        height: 600,
    };

    // We define a static variable for some random geometry that we will use
    // when adding windows to a window manager.
    static SOME_GEOM: Geometry = Geometry {
        x: 10,
        y: 10,
        width: 100,
        height: 100,
    };

    // Now let's write our test.
    //
    // Note that tests are annotated with `#[test]`, and cannot take arguments
    // nor return anything.
    #[test]
    fn test_adding_and_removing_some_windows() {
        // Let's make a new `FullscreenWM` with `SCREEN` as screen.
        let mut wm = WMName::new(SCREEN);

        // Initially the window layout should be empty.
        assert_eq!(WindowLayout::new(), wm.get_window_layout());
        // `assert_eq!` is a macro that will check that the second argument,
        // the actual value, matches first value, the expected value.

        // Let's add a window
        wm.add_window(WindowWithInfo::new_tiled(1, SOME_GEOM)).unwrap();
        // Because `add_window` returns a `Result`, we use `unwrap`, which
        // tries to extract the `Ok` value from the result, but will panic
        // (crash) when it is an `Err`. You must be very careful when using
        // `unwrap` in your code. Here we can use it because we know for sure
        // that an `Err` won't be returned, and even if that were the case,
        // the panic will simply cause the test to fail.

        // The window should now be managed by the WM
        assert!(wm.is_managed(1));
        // and be present in the `Vec` of windows.
        assert_eq!(vec![1], wm.get_windows());
        // According to the window layout
        let wl1 = wm.get_window_layout();
        // it should be focused
        assert_eq!(Some(1), wl1.focused_window);
        // and fullscreen.
        assert_eq!(vec![(1, SCREEN_GEOM)], wl1.windows);

        // Let's add another window.
        wm.add_window(WindowWithInfo::new_tiled(2, SOME_GEOM)).unwrap();
        // It should now be managed by the WM.
        assert!(wm.is_managed(2));
        // The `Vec` of windows should now contain both windows 1 and 2.
        assert_eq!(vec![1, 2], wm.get_windows());
        // According to the window layout
        let wl2 = wm.get_window_layout();
        // window 2 should be focused
        assert_eq!(Some(2), wl2.focused_window);

        let two_tiles_layout_with_gaps: Vec<(Window, Geometry)> = vec![(1,
                                                                        Geometry {
                                                                           x: 5,
                                                                           y: 5,
                                                                           width: 390,
                                                                           height: 590,
                                                                       }),
                                                                       (2,
                                                                        Geometry {
                                                                           x: 405,
                                                                           y: 5,
                                                                           width: 390,
                                                                           height: 590,
                                                                       })];
        // let's set a gap of 5px
        wm.set_gap(5);
        // let's compare the given layout with the returned one
        assert_eq!(two_tiles_layout_with_gaps, wm.get_window_layout().windows);

        let mut two_tiles_layout: Vec<(Window, Geometry)> = vec![(1,
                                                                  Geometry {
                                                                     x: 0,
                                                                     y: 0,
                                                                     width: 400,
                                                                     height: 600,
                                                                 }),
                                                                 (2,
                                                                  Geometry {
                                                                     x: 400,
                                                                     y: 0,
                                                                     width: 400,
                                                                     height: 600,
                                                                 })];
        wm.set_gap(0);
        // Now let's remove window 2
        wm.remove_window(2).unwrap();
        // It should no longer be managed by the WM.
        assert!(!wm.is_managed(2));
        // The `Vec` of windows should now just contain window 1.
        assert_eq!(vec![1], wm.get_windows());
        // According to the window layout
        let wl3 = wm.get_window_layout();
        // window 1 should be focused again
        assert_eq!(Some(1), wl3.focused_window);
        // and fullscreen.
        assert_eq!(vec![(1, SCREEN_GEOM)], wl3.windows);

        // let's get the window info of the 1st window
        let wwi = wm.get_window_info(1);
        // It should be equal to the window generated by the constructor 'new_tiled(1, SCREEN_GEOM)'
        assert_eq!(WindowWithInfo::new_tiled(1, SCREEN_GEOM), wwi.unwrap());
        // let's add two other windows
        wm.add_window(WindowWithInfo::new_tiled(2, SCREEN_GEOM)).unwrap();
        wm.add_window(WindowWithInfo::new_tiled(3, SCREEN_GEOM)).unwrap();

        let three_tiles_layout: Vec<(Window, Geometry)> = vec![(1,
                                                                Geometry {
                                                                   x: 0,
                                                                   y: 0,
                                                                   width: 400,
                                                                   height: 600,
                                                               }),
                                                               (2,
                                                                Geometry {
                                                                   x: 400,
                                                                   y: 0,
                                                                   width: 400,
                                                                   height: 300,
                                                               }),
                                                               (3,
                                                                Geometry {
                                                                   x: 400,
                                                                   y: 300,
                                                                   width: 400,
                                                                   height: 300,
                                                               })];
        let wl4 = wm.get_window_layout();
        // let's compare the given layout with the returned one
        assert_eq!(three_tiles_layout, wl4.windows);
        let geom = Geometry {
            x: 400,
            y: 0,
            width: 400,
            height: 300,
        };
        let window_with_info = WindowWithInfo::new_tiled(2, geom);
        // let's test the WindowWithInfo returned by the get_window_info function
        assert_eq!(window_with_info, wm.get_window_info(2).unwrap());
        // let's remove the second window
        wm.remove_window(2).unwrap();
        two_tiles_layout[1].0 = 3;
        let wl5 = wm.get_window_layout();
        // let's compare the given layout with the new one
        assert_eq!(two_tiles_layout, wl5.windows);
        // Now let's resize the screen
        wm.resize_screen(RESIZED);
        // Get the new screen from wv
        let screen = wm.get_screen();
        // screen should be equal to RESIZED screen
        assert_eq!(RESIZED, screen);

    }

    #[test]
    fn test_focusing_some_windows() {
        // Let's make a new `WMName` with `SCREEN` as screen.
        let mut wm = WMName::new(SCREEN);

        // Let's add a window
        wm.add_window(WindowWithInfo::new_tiled(1, SOME_GEOM)).unwrap();

        // Focus nothing
        let res1 = wm.focus_window(None);
        // There should be no errors
        assert!(res1.is_ok());
        // There should be no focus on any window
        assert_eq!(wm.get_focused_window(), None);

        // Let's try to focus a new unknown window
        let res2 = wm.focus_window(Some(2));
        // There should be an error
        assert!(res2.is_err());
        // Let's try to cycle the focus on the first window
        wm.cycle_focus(PrevOrNext::Prev);
        // Now the first window sould be focused
        assert_eq!(wm.get_focused_window(), Some(1));

        // Let's add a second window
        wm.add_window(WindowWithInfo::new_tiled(2, SOME_GEOM)).unwrap();
        // Let's change the focus on the previous window
        wm.cycle_focus(PrevOrNext::Prev);
        // Now the first window sould be focused
        assert_eq!(wm.get_focused_window(), Some(1));
        // Let's try to focus again the second window
        wm.cycle_focus(PrevOrNext::Prev);
        // Now the second window sould be focused
        assert_eq!(wm.get_focused_window(), Some(2));

        // Let's add an other window
        wm.add_window(WindowWithInfo::new_tiled(3, SOME_GEOM)).unwrap();
        // Let's change the focus on the previous window
        wm.cycle_focus(PrevOrNext::Prev);
        // Now the second window sould be focused
        assert_eq!(wm.get_focused_window(), Some(2));
        // Let's change the focus on the next window
        wm.cycle_focus(PrevOrNext::Next);
        // Now the third window sould be focused
        assert_eq!(wm.get_focused_window(), Some(3));
    }

    #[test]
    fn test_swapping_some_windows() {
        // Let's make a new `WMName` with `SCREEN` as screen.
        let mut wm = WMName::new(SCREEN);

        // Let's add a window
        wm.add_window(WindowWithInfo::new_tiled(1, SOME_GEOM)).unwrap();
        // Let's retrieve the master window
        let master1 = wm.get_master_window();
        // The master windows should be the fist one
        assert_eq!(master1, Some(1));
        // Let's add a second window
        wm.add_window(WindowWithInfo::new_tiled(2, SOME_GEOM)).unwrap();
        let master2 = wm.get_master_window();
        // The master windows should be the first one
        assert_eq!(master2, Some(1));
        // Let's swap the 2nd window with the master
        wm.swap_with_master(2).unwrap();
        let master3 = wm.get_master_window();
        // The master windows should be the second one
        assert_eq!(master3, Some(2));
        assert_eq!(wm.get_focused_window(), Some(2));
        // Let's try to swap a not managed window
        assert!(wm.swap_with_master(3).is_err());

        // Let's add a second window
        wm.add_window(WindowWithInfo::new_tiled(3, SOME_GEOM)).unwrap();
        // Let's remove the focus
        wm.focus_window(None).unwrap();
        // Now first tile is the master, and it should be also focused
        wm.swap_with_master(1).unwrap();
        assert_eq!(wm.get_focused_window(), Some(1));
        // Let's focus the second window
        wm.focus_window(Some(2)).unwrap();
        // Let's swap the focused window, the 2nd, with the prev
        wm.swap_windows(PrevOrNext::Prev);
        //  _ _ _ _ _ _
        // |     |  2  |
        // |  1  |_ f _|
        // |     |  3  |
        // |_ _ _|_ _ _|
        let master4 = wm.get_master_window();
        assert_eq!(master4, Some(2));
        //  _ _ _ _ _ _
        // |     |  1  |
        // |  2  |_ _ _|
        // |  f  |  3  |
        // |_ _ _|_ _ _|
        assert_eq!(wm.get_focused_window(), Some(2));
        wm.swap_windows(PrevOrNext::Next);
        let master5 = wm.get_master_window();
        assert_eq!(master5, Some(1));

        // Let's remove the focus
        wm.focus_window(None).unwrap();
        // Let's try to swap the focused window
        wm.swap_windows(PrevOrNext::Prev);
        // There should be no focus
        assert_eq!(wm.get_focused_window(), None);
        let master6 = wm.get_master_window();
        // The master should remain the first window
        assert_eq!(master6, Some(1));
    }

    // To run these tests, run the command `cargo test` in the `solution`
    // directory.
}
