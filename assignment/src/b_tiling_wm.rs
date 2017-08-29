//! Tiling Window Manager
//!
//! Write a more complex window manager that will *tile* its windows. Tiling
//! is described in the first section of the assignment. Your window manager
//! must implement both the [`WindowManager`] trait and the [`TilingSupport`]
//! trait. See the documentation of the [`TilingSupport`] trait for the
//! precise requirements and an explanation of the tiling layout algorithm.
//!
//! [`WindowManager`]: ../../cplwm_api/wm/trait.WindowManager.html
//! [`TilingSupport`]: ../../cplwm_api/wm/trait.TilingSupport.html
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
use cplwm_api::wm::{TilingSupport, WindowManager};
use wm_error::WMError;

use std::collections::{HashMap, VecDeque};

/// The name of the Window Manager
pub type WMName = TilingWM;

/// The TilingWM struct
#[derive(RustcDecodable, RustcEncodable, Debug, Clone)]
pub struct TilingWM {
    /// A VecDeque of windows, the first one is on the front, the last one is
    /// on back, the one in the back is the focused window
    pub windows: VecDeque<Window>,
    /// A HashMap to store the info associated to windows
    pub windows_info: HashMap<Window, WindowWithInfo>,
    /// An other VecDeque of windows, the one in the front is the master tile
    pub tiles: VecDeque<Window>,
    /// We need to know which size the fullscreen window must be.
    pub screen: Screen,
    /// Boolean variable to indicate if there is a focused window
    pub is_focus: bool,
}

// Now we start implementing our window manager
impl WindowManager for TilingWM {
    /// We use `WMError` as our `Error` type.
    type Error = WMError;

    /// The constructor is straightforward.
    ///
    fn new(screen: Screen) -> TilingWM {
        TilingWM {
            windows: VecDeque::new(),
            windows_info: HashMap::new(),
            tiles: VecDeque::new(),
            screen: screen,
            is_focus: false,
        }
    }

    /// The `windows` field contains all the windows we manage.
    fn get_windows(&self) -> Vec<Window> {
        let mut windows: Vec<Window> = Vec::new();

        for i in 0..self.windows.len() {
            windows.push(*self.windows.get(i).unwrap());
        }

        windows
    }

    /// The last window in the list is the focused one.
    fn get_focused_window(&self) -> Option<Window> {
        if self.is_focus {
            // if there is no window in the vec, back() function returns None
            self.windows.back().map(|x| *x)
        } else {
            None
        }
    }

    /// To add a window, just push it onto the end the `windows` `VecDeque`.
    /// Add the window also in the tiles `VecDeque`, and the WindowWithInfo in the `HashMap`.
    ///
    /// The function returns an error if the window is already managed by the window manager.
    fn add_window(&mut self, window_with_info: WindowWithInfo) -> Result<(), Self::Error> {
        if !self.is_managed(window_with_info.window) {
            self.windows.push_back(window_with_info.window);
            // insert the info in the hasmap
            self.windows_info.insert(window_with_info.window, window_with_info);
            // workaround
            if window_with_info.float_or_tile == FloatOrTile::Tile {
                self.tiles.push_back(window_with_info.window);
            }
            self.is_focus = true;
            Ok(())
        } else {
            Err(WMError::AlreadyManagedWindow(window_with_info.window))
        }
    }

    /// To remove a window, remove it from the `windows` `VecDeque`.
    /// Remove also the window from `tiles`, and it's associated `WindowWithInfo`.
    fn remove_window(&mut self, window: Window) -> Result<(), Self::Error> {
        match self.windows.iter().position(|w| *w == window) {
            None => Err(WMError::UnknownWindow(window)),
            Some(i) => {
                // workaround for FloatingWM
                if self.windows_info.get(&window).unwrap().float_or_tile == FloatOrTile::Float {
                    self.windows.remove(i);
                    self.windows_info.remove(&window);
                    return Ok(());
                }
                // remove from windows
                self.windows.remove(i);
                // remove WindowWithInfo
                self.windows_info.remove(&window);

                // if there are no more windows, then there is no focus
                if self.windows.is_empty() {
                    // remove also the window for the tiles VecDeque
                    self.tiles.remove(0);
                    self.is_focus = false;
                } else {
                    // if the window is tiled remove it from the tiles
                    let j = self.tiles.iter().position(|t| *t == window).unwrap();
                    self.tiles.remove(j);
                }
                Ok(())
            }
        }
    }

    /// Return the `WindowLayout` of the WindowManager.
    /// Calculate the geometry of each window, subdivining the space of the whole screen.
    fn get_window_layout(&self) -> WindowLayout {
        let fullscreen_geometry = self.screen.to_geometry();
        match self.windows.back() {

            // If there is at least one window.
            Some(w) => {
                let len = self.tiles.len();
                let mut focused = Some(*w);
                if !self.is_focus {
                    focused = None;
                }
                match len {
                    0 => {
                        WindowLayout {
                            focused_window: focused,
                            windows: Vec::new(),
                        }
                    } 
                    1 => {
                        WindowLayout {
                            focused_window: focused,
                            windows: vec![(*(self.tiles.back().unwrap()), fullscreen_geometry)],
                        }
                    } 
                    _ => {
                        let mut windows: Vec<(Window, Geometry)> = Vec::new();
                        let mut geometry = Geometry {
                            x: 0,
                            y: 0,
                            width: fullscreen_geometry.width / 2,
                            height: fullscreen_geometry.height,
                        };
                        windows.push((self.tiles[0], geometry));
                        let height = fullscreen_geometry.height / ((self.tiles.len() - 1) as u32);
                        geometry.x = geometry.width as i32;
                        geometry.height = height;
                        // workaround to not write also the case for the 2nd window
                        geometry.y -= height as i32;
                        for i in 1..self.tiles.len() {
                            geometry.y += height as i32;
                            windows.push((self.tiles[i], geometry));
                        }
                        WindowLayout {
                            focused_window: focused,
                            windows: windows,
                        }
                    }
                }
            }
            // Otherwise, return an empty WindowLayout
            None => WindowLayout::new(),
        }
    }

    /// Focus the given window, or when passed None, focus nothing.
    ///
    /// Move the new focused window in the last position of the windows `VecDeque`.
    fn focus_window(&mut self, window: Option<Window>) -> Result<(), Self::Error> {
        match window {
            None => {
                self.is_focus = false;
                Ok(())
            } 
            Some(_) => {
                match self.windows.iter().position(|w| *w == window.unwrap()) {
                    None => Err(WMError::UnknownWindow(window.unwrap())),
                    Some(i) => {
                        self.is_focus = true;
                        let w = self.windows.remove(i);
                        self.windows.push_back(w.unwrap());
                        Ok(())
                    }
                }
            }
        }
    }

    /// Focus the previous or next window.
    ///
    /// Behaves as the `cycle_focus` of the `FullscreenWM`.
    fn cycle_focus(&mut self, dir: PrevOrNext) {
        let len = self.windows.len();
        match len {
            0 => return,
            // When there is only one window, focus it if currently no window is focused. (redundant in the code)
            1 => self.is_focus = true, 
            // With two windows swap them.
            2 => self.windows.swap(0, 1),
            _ => {
                match dir {
                    // The windows vecDeque has to be seen as a circular buffer
                    // With Prev move the last element in the first position of the list
                    PrevOrNext::Prev => {
                        let w = self.windows.pop_back().unwrap();
                        self.windows.push_front(w);
                    }
                    // With Next move the first element in the last position of the list
                    PrevOrNext::Next => {
                        let w = self.windows.pop_front().unwrap();
                        self.windows.push_back(w);
                    }
                }
            }
        }
        self.is_focus = true;
    }

    /// Get the info (WindowWithInfo) belonging to the given window.
    ///
    /// Retrive it from the `WindowLayout`.
    fn get_window_info(&self, window: Window) -> Result<WindowWithInfo, Self::Error> {
        match self.windows.iter().position(|w| *w == window) {
            None => Err(WMError::UnknownWindow(window)),
            Some(_) => {
                let layout = self.get_window_layout().windows;
                let i = layout.iter().position(|w| (*w).0 == window).unwrap();
                Ok(WindowWithInfo::new_tiled(window, layout[i].1))
            }
        }
    }

    /// Return the screen managed by the window manager.
    fn get_screen(&self) -> Screen {
        self.screen
    }

    /// Resize the screen according to the given Screen.
    fn resize_screen(&mut self, screen: Screen) {
        self.screen = screen;
    }
}

// Now we start implementing the methods fot the TilingSupport trait

impl TilingSupport for TilingWM {
    /// Return the window displayed in the master tile.
    /// The master window is the one in the last position of the tiles `VecDeque`.
    fn get_master_window(&self) -> Option<Window> {
        match self.tiles.front() {
            None => None, 
            Some(w) => Some(*w),
        }
    }

    /// Swap the given window with the window in the master tile.
    ///
    /// Swap the given window, if exists, in the last position of the tiles `VecDeque`.
    /// Then focus the window.
    fn swap_with_master(&mut self, window: Window) -> Result<(), Self::Error> {
        if !self.is_managed(window) {
            Err(WMError::UnknownWindow(window))
        } else {
            match self.tiles.iter().position(|t| *t == window) {
                // it's float
                None => Ok(()), 
                Some(i) => {
                    self.tiles.swap(0, i);
                    self.focus_window(Some(window)).unwrap();
                    Ok(())
                }
            }
        }
    }

    /// Swap the focused window with the one in the next or previous tile.
    /// If there is no focus return.
    /// If there is only one tile do nothing.
    /// If there are 2 tiles, swap them.
    /// If there are more than 2 tiles swap the focused one with the previous/next one, considering
    /// the tiles `VecDeque` as circular.
    fn swap_windows(&mut self, dir: PrevOrNext) {
        // Do nothing when no window is focused.
        if !self.is_focus {
            return;
        }

        let len = self.tiles.len();
        // Do nothing when there are no windows and when there is only one window
        match len {
            // If there were two tiles and the swap happened, the same window will be focused, but the other tile will be focused.
            2 => {
                self.tiles.swap(0, 1);

            } 
            _ => {
                // get the index of the focused window in the tiles VecDeque
                // unwrap it, cause we're sure the tile exists
                let i = self.tiles
                    .iter()
                    .position(|t| *t == *self.windows.back().unwrap())
                    .unwrap();
                match dir {
                    PrevOrNext::Prev => {
                        // if the focused window is also the master tile, swap the master tile
                        // with the last tile
                        if i == 0 {
                            self.tiles.swap(i, len - 1);
                        }
                        // if the index is 1 swap the first 2 tiles
                        else if i == 1 {
                            self.tiles.swap(0, 1);
                        }
                        // otherwise swap the tile with the previous one
                        else {
                            self.tiles.swap(i, i - 1);
                        }
                    } 
                    PrevOrNext::Next => {
                        // swap the tile with the next one
                        self.tiles.swap(i, (i + 1) % len);
                    }
                }
            } 
        }
    }
}

// Here we define a submodule, called `tests`, that will contain the unit
// tests of this module.
//
// The `#[cfg(test)]` annotation means that this code is only compiled when
// we're testing the code.
#[cfg(test)]
mod tests {

    use super::TilingWM;
    use cplwm_api::wm::{TilingSupport, WindowManager};
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
        let mut wm = TilingWM::new(SCREEN);

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
        // let's compare the give layout with the returned one
        assert_eq!(two_tiles_layout, wl2.windows);

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
        // Let's make a new `TilingWM` with `SCREEN` as screen.
        let mut wm = TilingWM::new(SCREEN);

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
        // Let's make a new `TilingWM` with `SCREEN` as screen.
        let mut wm = TilingWM::new(SCREEN);

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
