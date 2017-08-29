//! Floating Windows
//!
//! Extend your window manager with support for floating windows, i.e. windows
//! that do not tile but that you move around and resize with the mouse. These
//! windows will *float* above the tiles, e.g. dialogs, popups, video players,
//! etc. See the documentation of the [`FloatSupport`] trait for the precise
//! requirements.
//!
//! Either make a copy of the tiling window manager you developed in the
//! previous assignment and let it implement the [`FloatSupport`] trait as
//! well, or implement the [`FloatSupport`] trait by building a wrapper around
//! your tiling window manager. This way you won't have to copy paste code.
//! Note that this window manager must still implement the [`TilingSupport`]
//! trait.
//!
//! [`FloatSupport`]: ../../cplwm_api/wm/trait.FloatSupport.html
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
use cplwm_api::wm::{FloatSupport, TilingSupport, WindowManager};
use b_tiling_wm::TilingWM;

use wm_error::WMError;

use std::collections::HashMap;

/// The name of the Window Manager
pub type WMName = FloatingWM;

/// The FloatingWM struct
#[derive(RustcDecodable, RustcEncodable, Debug, Clone)]
pub struct FloatingWM {
    /// A wrapped Tiling Window Manager
    pub tiling_wm: TilingWM,

    /// An HashMap of floating windows <Window, Geometry>
    pub floats: HashMap<Window, Geometry>,
}


impl WindowManager for FloatingWM {
    /// We use 'WMError` as our `Error` type.
    type Error = WMError;

    /// The constructor is straightforward.
    ///
    fn new(screen: Screen) -> FloatingWM {
        FloatingWM {
            // initialize the wrapped WM
            tiling_wm: TilingWM::new(screen),
            floats: HashMap::new(),
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

    /// To add a window first call the old the `add_window` function from the
    /// TilingWM and check if whether or not the function returns an error.
    ///
    /// If the window is Float the wrapped `add_window` does not managed it,
    /// then add the window and its geometry to the floats `HashMap`.
    fn add_window(&mut self, window_with_info: WindowWithInfo) -> Result<(), Self::Error> {
        // get the return value of the add_window funciton
        try!(self.tiling_wm.add_window(window_with_info));
        // if the window is float, add the window and its gemometry to the floats vec
        if window_with_info.float_or_tile == FloatOrTile::Float {
            self.floats.insert(window_with_info.window, window_with_info.geometry);
        }
        Ok(())
    }

    /// First we try to call the wrapped function, if there is an error we return it.
    ///
    /// If there is no error and the window is float, we remove it from the `floats` vec
    fn remove_window(&mut self, window: Window) -> Result<(), Self::Error> {
        try!(self.tiling_wm.remove_window(window));
        if self.floats.contains_key(&window) {
            // if there are no more windows, then there is no focus
            if self.tiling_wm.windows.is_empty() {
                // remove also the window for the floats Vec
                let w = 0 as u64;
                self.floats.remove(&w);
                self.tiling_wm.is_focus = false;
            } else {
                // if the window is tiled remove it from the floats
                self.floats.remove(&window);
            }
            Ok(())
        } else {
            Ok(())
        }
    }


    /// The function concatenates the tiled windows returned by the TilingWM
    /// with the floating windows layout respecting the order of the focus.
    fn get_window_layout(&self) -> WindowLayout {
        let mut layout = self.tiling_wm.get_window_layout();
        // for each window in the windows `VecDeque` if the window is floating, concatenate it to
        // the windows layout (to maintain the order of the focus)
        for i in 0..self.tiling_wm.windows.len() {
            let window = self.tiling_wm.windows[i];
            // if the window is floating
            if self.is_floating(window) {
                let geom = self.floats.get(&window);
                // workaround for minimised windows
                if geom.is_some() {
                    layout.windows.push((window, *geom.unwrap()));
                }
            }
        }
        layout
    }

    /// Focus the given window, or when passed None, focus nothing.
    fn focus_window(&mut self, window: Option<Window>) -> Result<(), Self::Error> {
        self.tiling_wm.focus_window(window)
    }

    /// Focus the previous or next window.
    fn cycle_focus(&mut self, dir: PrevOrNext) {
        self.tiling_wm.cycle_focus(dir);
    }

    /// Get the info (WindowWithInfo) belonging to the given window.
    ///
    /// If the window is tiled call the wrapped function.
    /// Otherwise return the WindowWithInfo from the `HashMap`.
    fn get_window_info(&self, window: Window) -> Result<WindowWithInfo, Self::Error> {
        match self.tiling_wm.windows_info.get(&window) {
            None => Err(WMError::UnknownWindow(window)),
            Some(window_with_info) => {
                match window_with_info.float_or_tile {
                    FloatOrTile::Float => self.tiling_wm.get_window_info(window), 
                    FloatOrTile::Tile => Ok(*window_with_info),
                }
            }
        }
    }

    /// Return the screen managed by the window manager.
    fn get_screen(&self) -> Screen {
        self.tiling_wm.get_screen()
    }

    /// Resize the screen according to the given Screen.
    fn resize_screen(&mut self, screen: Screen) {
        self.tiling_wm.resize_screen(screen);
    }
}

// Now we start implementing the methods fot the TilingSupport trait

impl TilingSupport for FloatingWM {
    /// Return the window displayed in the master tile.
    fn get_master_window(&self) -> Option<Window> {
        self.tiling_wm.get_master_window()
    }

    /// Swap the given window with the window in the master tile.
    fn swap_with_master(&mut self, window: Window) -> Result<(), Self::Error> {
        self.tiling_wm.swap_with_master(window)
    }

    /// Swap the focused window with the one in the next or previous tile.
    ///
    /// If the focused window is Tiled call the wrapped function otherwise do nothing.
    fn swap_windows(&mut self, dir: PrevOrNext) {
        if self.tiling_wm.is_focus {
            let focused = self.get_focused_window().unwrap();
            match self.get_window_info(focused).unwrap().float_or_tile {
                FloatOrTile::Float => (), 
                FloatOrTile::Tile => self.tiling_wm.swap_windows(dir),
            }
        }
    }
}

impl FloatSupport for FloatingWM {
    /// Return the list of floating windows
    fn get_floating_windows(&self) -> Vec<Window> {
        self.floats
            .keys()
            .map(|w| *w)
            .collect::<Vec<_>>()
    }

    /// If the given window is floating, let it sink, if it is not floating, let it float.
    ///
    /// If the window is floating, remove it from the floats, push it on the tiles, and change its
    /// `WindowWithInfo`.
    /// If the windows is tiled, remove it from the tiles, add it to the floats retrieving its
    /// original geometry from the windows_info `HashMap`.
    fn toggle_floating(&mut self, window: Window) -> Result<(), Self::Error> {
        match self.tiling_wm.windows_info.get_mut(&window) {
            // if the window is not managed return an error
            None => Err(WMError::UnknownWindow(window)), 
            Some(window_with_info) => {
                // check whether the windows is float or tile
                match window_with_info.float_or_tile {
                    FloatOrTile::Float => {
                        // remove the window from the floats
                        self.floats.remove(&window);
                        // insert the window in the tile VecDeque
                        self.tiling_wm.tiles.push_back(window);
                        // update the window info of the window
                        window_with_info.float_or_tile = FloatOrTile::Tile;
                        Ok(())
                    } 
                    FloatOrTile::Tile => {
                        let j = self.tiling_wm.tiles.iter().position(|t| *t == window).unwrap();
                        // remove the window from the tiles
                        self.tiling_wm.tiles.remove(j);
                        // retrieve the old geometry
                        let geom = window_with_info.geometry;
                        // push window + geometry to floats
                        self.floats.insert(window, geom);
                        // update the window info of the window
                        window_with_info.float_or_tile = FloatOrTile::Float;
                        Ok(())
                    } 
                }
            }
        }
    }

    /// Resize/move the given floating window according to the given geometry.
    fn set_window_geometry(&mut self,
                           window: Window,
                           new_geometry: Geometry)
                           -> Result<(), Self::Error> {
        match self.tiling_wm.windows_info.get_mut(&window) {
            None => Err(WMError::UnknownWindow(window)), 
            Some(window_with_info) => {
                if self.floats.contains_key(&window) {
                    // update the window geometry in the window info
                    window_with_info.geometry = new_geometry;
                    // update also the geometry here
                    self.floats.insert(window, new_geometry);
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::FloatingWM;
    use cplwm_api::wm::{FloatSupport, TilingSupport, WindowManager};
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
        let mut wm = FloatingWM::new(SCREEN);

        // Initially the window layout should be empty.
        assert_eq!(WindowLayout::new(), wm.get_window_layout());

        // Let's add a window
        wm.add_window(WindowWithInfo::new_tiled(1, SOME_GEOM)).unwrap();

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

        let two_tiles_layout: Vec<(Window, Geometry)> = vec![(1,
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

        // let's add two other windows
        // a tile window
        wm.add_window(WindowWithInfo::new_tiled(2, SOME_GEOM)).unwrap();
        // and a float
        wm.add_window(WindowWithInfo::new_float(3, SOME_GEOM)).unwrap();

        let mixed_layout: Vec<(Window, Geometry)> = vec![(1,
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
                                                         }),
                                                         (3, SOME_GEOM)];
        let wl4 = wm.get_window_layout();
        // let's compare the given layout with the returned one
        assert_eq!(mixed_layout, wl4.windows);
        // let's remove the second window
        wm.remove_window(2).unwrap();

        let two_windows_layout: Vec<(Window, Geometry)> = vec![(1,
                                                                Geometry {
                                                                   x: 0,
                                                                   y: 0,
                                                                   width: 800,
                                                                   height: 600,
                                                               }),
                                                               (3, SOME_GEOM)];
        assert_eq!(vec![1, 3], wm.get_windows());
        let wl5 = wm.get_window_layout();
        // let's compare the given layout with the new one
        assert_eq!(two_windows_layout, wl5.windows);
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
        let mut wm = FloatingWM::new(SCREEN);

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
        wm.add_window(WindowWithInfo::new_float(2, SOME_GEOM)).unwrap();
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

        // Let's remove the float window
        wm.remove_window(2).unwrap();
        assert_eq!(vec![1, 3], wm.get_windows());
        // Now the focus should be on the 3rd window
        // Let's change the focus on the next window
        wm.cycle_focus(PrevOrNext::Next);
        // Now the focus should be on the 1st window
        assert_eq!(wm.get_focused_window(), Some(1));
        // Let's remove the first windows
        wm.remove_window(1).unwrap();
        assert_eq!(vec![3], wm.get_windows());
        // Now the focus should be on the last one
        assert_eq!(wm.get_focused_window(), Some(3));
    }

    #[test]
    fn test_swapping_some_windows() {
        // Let's make a new `TilingWM` with `SCREEN` as screen.
        let mut wm = FloatingWM::new(SCREEN);

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

    #[test]
    fn test_floating_windows() {
        let mut wm = FloatingWM::new(SCREEN);
        // let's add a window
        wm.add_window(WindowWithInfo::new_tiled(1, SOME_GEOM)).unwrap();
        // let's add another window
        wm.add_window(WindowWithInfo::new_float(2, SOME_GEOM)).unwrap();
        // let's check if the second window is float
        assert!(wm.is_floating(2));
        assert_eq!(vec![2], wm.get_floating_windows());
        // the first window should be tiled
        assert!(!wm.is_floating(1));
        // let's make the 1st window float
        wm.toggle_floating(1).unwrap();
        // now the first window should be also float
        assert!(wm.is_floating(1));
        // the order depends of the implementation
        assert_eq!(2, wm.get_floating_windows().len());
        // let's toggle the 2nd
        wm.toggle_floating(2).unwrap();
        // now only the 1st is float
        assert_eq!(vec![1], wm.get_floating_windows());
        // and the 2nd is the master window
        assert_eq!(Some(2), wm.get_master_window());
        // the function should return an error, the window is not managed
        assert!(wm.set_window_geometry(0, SOME_GEOM).is_err());
        // let's make the 2nd window float
        wm.toggle_floating(2).unwrap();
        // let's resize the 2nd window
        wm.set_window_geometry(2, SCREEN_GEOM).unwrap();
        let wl1 = wm.get_window_layout();
        let two_windows_layout: Vec<(Window, Geometry)> = vec![(1, SOME_GEOM), (2, SCREEN_GEOM)];
        assert_eq!(two_windows_layout, wl1.windows);

    }
    // To run these tests, run the command `cargo test` in the `solution`
    // directory.
}
