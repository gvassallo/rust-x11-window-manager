//! Minimising Windows
//!
//! Extend your window manager with support for (un)minimising windows. i.e.
//! the ability to temporarily hide windows and to reveal them again later.
//! See the documentation of the [`MinimiseSupport`] trait for the precise
//! requirements.
//!
//! Either make a copy of the tiling window manager with support for floating
//! windows you developed in the previous assignment and let it implement the
//! [`MinimiseSupport`] trait as well, or implement this trait by building a
//! wrapper around the previous window manager. Note that this window manager
//! must still implement all the traits from previous assignments.
//!
//! [`MinimiseSupport`]: ../../cplwm_api/wm/trait.MinimiseSupport.html
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
use cplwm_api::wm::{FloatSupport, MinimiseSupport, TilingSupport, WindowManager};
use c_floating_windows::FloatingWM;
use wm_error::WMError;

/// The name of the Window Manger
pub type WMName = MinimiseWM;

/// Window Manager that supports (un)minimising windows
#[derive(RustcDecodable, RustcEncodable, Debug, Clone)]
pub struct MinimiseWM {
    /// A wrapper of the Tiling Window Manager
    pub floating_wm: FloatingWM,
    /// A Vec containing all the minimised windows
    pub minimised: Vec<Window>,
}

impl WindowManager for MinimiseWM {
    /// We use `WMError` as our `Error` type
    type Error = WMError;
    /// The constructor wraps the constructor of the FloatingWM
    /// it also declares a `Vec` of minimised windows.
    fn new(screen: Screen) -> MinimiseWM {
        MinimiseWM {
            floating_wm: FloatingWM::new(screen),
            minimised: Vec::new(),
        }
    }

    /// The function wraps the old `get_windows`
    fn get_windows(&self) -> Vec<Window> {
        self.floating_wm.get_windows()
    }

    /// The function wraps the old `get_focused_window`
    fn get_focused_window(&self) -> Option<Window> {
        self.floating_wm.get_focused_window()
    }

    /// The function wraps the old `add_window`
    fn add_window(&mut self, window_with_info: WindowWithInfo) -> Result<(), Self::Error> {
        self.floating_wm.add_window(window_with_info)
    }

    /// Remove the given window from the window manager.
    /// If it's minimied, unminimised it and call the wrapped function.
    fn remove_window(&mut self, window: Window) -> Result<(), Self::Error> {
        if self.is_managed(window) && self.is_minimised(window) {
            self.toggle_minimised(window).unwrap();
        }
        self.floating_wm.remove_window(window)
    }

    /// Call the wrapped function.
    fn get_window_layout(&self) -> WindowLayout {
        self.floating_wm.get_window_layout()
    }

    /// Focus the given window, or when passed None, focus nothing.
    ///
    /// If the windwos is `Some`, managed and minimised, unminimise it.
    /// Then call the wrapped function.
    fn focus_window(&mut self, window: Option<Window>) -> Result<(), Self::Error> {
        // if the window is not none, is managed and minimised
        if window.is_some() && self.is_managed(window.unwrap()) &&
           self.is_minimised(window.unwrap()) {
            // unminimised it
            self.toggle_minimised(window.unwrap()).unwrap();
        }
        // and call the wrapped function
        self.floating_wm.focus_window(window)
    }

    /// Focus the previous or next window.
    ///
    /// Call the wrapped function, if the next/prev window is minimised, unminimised it.
    /// (Like Windows Window Manager)
    fn cycle_focus(&mut self, dir: PrevOrNext) {
        // focus the next/prev windows using the wrapped function
        if self.get_windows().len() == 0 {
            return;
        }
        self.floating_wm.cycle_focus(dir);
        // we now we have focus
        let window = self.get_focused_window().unwrap();
        // if the current focused window is minimised
        if self.is_minimised(window) {
            // unminimised it
            self.toggle_minimised(window).unwrap();
        }
    }

    /// Get the info (WindowWithInfo) belonging to the given window.
    /// If it's not minimised call the wrapped function, otherwise return the WindowWithInfo from
    /// the windows_info `HashMap`. For the tiles the geometry is the one specified at the creation
    /// phase.
    fn get_window_info(&self, window: Window) -> Result<WindowWithInfo, Self::Error> {
        if !self.is_managed(window) {
            Err(WMError::UnknownWindow(window))
            // } else if !self.is_minimised(window) {
            // self.floating_wm.get_window_info(window)
        } else {
            Ok(*(self.floating_wm.tiling_wm.windows_info.get(&window).unwrap()))
        }
    }

    /// Return the screen managed by the window manager.
    fn get_screen(&self) -> Screen {
        self.floating_wm.get_screen()
    }

    /// Resize the screen according to the given Screen.
    fn resize_screen(&mut self, screen: Screen) {
        self.floating_wm.resize_screen(screen);
    }
}

impl TilingSupport for MinimiseWM {
    /// Return the window displayed in the master tile.
    fn get_master_window(&self) -> Option<Window> {
        self.floating_wm.get_master_window()
    }

    /// Swap the given window with the window in the master tile.
    /// If the window is tiled and minimised, unminimised it first, then call the wrapped function.
    fn swap_with_master(&mut self, window: Window) -> Result<(), Self::Error> {
        if !self.is_managed(window) {
            return Err(WMError::UnknownWindow(window));
        } else if self.get_window_info(window).unwrap().float_or_tile == FloatOrTile::Float {
            return Ok(());
            // if the window is tiled and minimised
        } else if self.is_minimised(window) {
            // unminimised it first
            self.toggle_minimised(window).unwrap();
        }
        // then call the wrapped function
        self.floating_wm.swap_with_master(window)
    }

    /// Swap the focused window with the one in the next or previous tile.
    /// Call the wrapped function.
    fn swap_windows(&mut self, dir: PrevOrNext) {
        self.floating_wm.swap_windows(dir);
    }
}

impl FloatSupport for MinimiseWM {
    /// returns the list of all VISIBLE floating windows
    fn get_floating_windows(&self) -> Vec<Window> {
        self.floating_wm.get_floating_windows()
    }

    /// If the given window is floating, let it sink, if it is not floating, let it float.
    /// If the windows is minimised, unminimise it first, then call the wrapped function.
    fn toggle_floating(&mut self, window: Window) -> Result<(), Self::Error> {
        if self.is_managed(window) && self.is_minimised(window) {
            self.toggle_minimised(window).unwrap();
        }
        self.floating_wm.toggle_floating(window)
    }

    /// Resize/move the given floating window according to the given geometry.
    /// If the window is minimised and floating, unminimised it, then call the wrapped funciton.
    fn set_window_geometry(&mut self,
                           window: Window,
                           new_geometry: Geometry)
                           -> Result<(), Self::Error> {
        if !self.is_managed(window) {
            return Err(WMError::UnknownWindow(window));
        }
        if self.is_minimised(window) {
            // I decided to unminimise the window and resize it to be consistent with the previous
            // function
            if self.get_window_info(window).unwrap().float_or_tile == FloatOrTile::Float {
                self.toggle_minimised(window).unwrap();
            }
        }
        self.floating_wm.set_window_geometry(window, new_geometry)
    }
}

impl MinimiseSupport for MinimiseWM {
    /// Return the minimised `Vec`.
    fn get_minimised_windows(&self) -> Vec<Window> {
        self.minimised.clone()
    }
    /// If the window is minimised:
    ///
    /// * If it's float add it to the floats `HashMap`, otherwise to the tiles `VecDeque`
    /// * Then remove it from the minimised `Vec`
    /// * Focus it
    ///
    /// If the window is unminimised:
    ///
    /// * Remove it from the floats `HashMap` or from the tiles `VecDeque`
    /// * Add it to the minimised `Vec`
    /// * If there was no focus, or focus to another window return
    /// * Otherwise focus the previous unminimised window
    fn toggle_minimised(&mut self, window: Window) -> Result<(), Self::Error> {
        if !self.is_managed(window) {
            return Err(WMError::UnknownWindow(window));
        }
        let window_with_info = self.get_window_info(window).unwrap();

        if self.is_minimised(window) {
            match window_with_info.float_or_tile {
                FloatOrTile::Float => {
                    self.floating_wm
                        .floats
                        .insert(window_with_info.window, window_with_info.geometry);
                } 
                FloatOrTile::Tile => {
                    self.floating_wm
                        .tiling_wm
                        .tiles
                        .push_back(window_with_info.window);
                }
            }
            let i = self.minimised.iter().position(|w| *w == window).unwrap();
            self.minimised.remove(i);
            self.focus_window(Some(window)).unwrap();
        } else {
            // add the window to the Vec of minimised windows
            self.minimised.push(window);
            match window_with_info.float_or_tile {
                FloatOrTile::Float => {
                    self.floating_wm
                        .floats
                        .remove(&window);
                } 
                FloatOrTile::Tile => {
                    let i = self.floating_wm
                        .tiling_wm
                        .tiles
                        .iter()
                        .position(|w| *w == window)
                        .unwrap();
                    self.floating_wm
                        .tiling_wm
                        .tiles
                        .remove(i);
                }
            }
            let focus = self.get_focused_window();

            if focus.is_none() || focus != Some(window) {
                return Ok(());
            }
            // number of unminimised window
            let len = self.get_windows().len() - self.minimised.len();
            // if the focus was on the minimised window let's change the focus
            // let's focus the first previous window that is not minimised
            match len {
                0 => {
                    self.focus_window(None).unwrap();
                } 
                _ => {
                    let windows = self.get_windows();
                    // scan the windows in reverse order
                    for i in (0..(windows.len() - 1)).rev() {
                        // the first window that is not minimised
                        if self.minimised.iter().position(|w| *w == windows[i]).is_none() {
                            // receives the focus
                            self.focus_window(Some(windows[i])).unwrap();
                            break;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::WMName;
    use cplwm_api::wm::{FloatSupport, MinimiseSupport, TilingSupport, WindowManager};
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
        // Let's make a new `MinimiseWM` with `SCREEN` as screen.
        let mut wm = WMName::new(SCREEN);

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

    #[test]
    fn test_floating_windows() {
        let mut wm = WMName::new(SCREEN);
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

    #[test]
    fn minimise_some_windows() {
        let mut wm = WMName::new(SCREEN);

        wm.add_window(WindowWithInfo::new_float(1, SOME_GEOM)).unwrap();
        // let's get the one window layout
        let wl1 = wm.get_window_layout();
        // let's add a new float
        wm.add_window(WindowWithInfo::new_float(2, SOME_GEOM)).unwrap();
        // let's try to minimise a window not managed
        let res = wm.toggle_minimised(3);
        // it should return an error
        assert!(res.is_err());
        // let's get the two windows layout
        let wl2 = wm.get_window_layout();
        // let's minimise the second window
        wm.toggle_minimised(2).unwrap();
        // the minimised window should be only the second
        assert_eq!(vec![2], wm.get_minimised_windows());
        // let's check if the layout is the only one window layout
        assert_eq!(wl1, wm.get_window_layout());
        // let's unminimise the second window
        wm.toggle_minimised(2).unwrap();
        // let's check if the layout is the same as the one before the minimisation
        assert_eq!(wl2, wm.get_window_layout());

        // let's remove the second window
        wm.remove_window(2).unwrap();
        // let's minimise the first
        wm.toggle_minimised(1).unwrap();
        // there should not be focus
        assert_eq!(wm.get_focused_window(), None);
        // unminimise the first
        wm.toggle_minimised(1).unwrap();
        assert_eq!(wl1, wm.get_window_layout());

        wm.toggle_minimised(1).unwrap();
        // let's remove a minimised window
        wm.remove_window(1).unwrap();
        assert_eq!(0, wm.get_windows().len());

        wm.add_window(WindowWithInfo::new_tiled(1, SOME_GEOM)).unwrap();
        wm.add_window(WindowWithInfo::new_tiled(2, SOME_GEOM)).unwrap();
        // let's save the two tiled layout
        let wl3 = wm.get_window_layout();
        wm.add_window(WindowWithInfo::new_tiled(3, SOME_GEOM)).unwrap();
        // minimise a tile
        wm.toggle_minimised(3).unwrap();
        // should be equal to the previous
        assert_eq!(wl3, wm.get_window_layout());
        // let's add a float
        wm.add_window(WindowWithInfo::new_float(4, SOME_GEOM)).unwrap();
        // let's save the layout
        let wl4 = wm.get_window_layout();
        // let's minimise 4th
        wm.toggle_minimised(4).unwrap();
        // should be still equal to the previous
        assert_eq!(wl3, wm.get_window_layout());
        // the order should be 3, 4
        assert_eq!(vec![3, 4], wm.get_minimised_windows());
        // let's focus a minimised window
        wm.focus_window(Some(4)).unwrap();
        // now the window should be visible
        assert_eq!(wl4, wm.get_window_layout());
        // let's focus the previous window
        wm.cycle_focus(PrevOrNext::Prev);
        // let's focus the previous window, that is minimised
        wm.cycle_focus(PrevOrNext::Prev);
        assert_eq!(Some(3), wm.get_focused_window());

    }

    #[test]
    fn swap_minimised_tiled_windows() {
        let mut wm = WMName::new(SCREEN);
        wm.add_window(WindowWithInfo::new_tiled(1, SOME_GEOM)).unwrap();
        wm.add_window(WindowWithInfo::new_tiled(2, SOME_GEOM)).unwrap();
        let wl1 = wm.get_window_layout();
        // minimize the master
        wm.toggle_minimised(1).unwrap();
        // swap the minimized with the master
        wm.swap_with_master(1).unwrap();
        // focus the second
        wm.focus_window(Some(2)).unwrap();
        // now should be equal to wl1
        assert_eq!(wl1, wm.get_window_layout());
    }

    #[test]
    fn minimise_floating_windows() {
        let mut wm = WMName::new(SCREEN);

        wm.add_window(WindowWithInfo::new_float(1, SOME_GEOM)).unwrap();
        wm.add_window(WindowWithInfo::new_tiled(2, SOME_GEOM)).unwrap();

        // let's save the layout
        let wl1 = wm.get_window_layout();
        // let's make float the 2nd
        wm.toggle_floating(2).unwrap();
        // let's minimise it
        wm.toggle_minimised(2).unwrap();
        // now only 1st should be returned from the function
        assert_eq!(vec![1], wm.get_floating_windows());
        // let the minimised window sink
        wm.toggle_floating(2).unwrap();
        // focus the 2nd window
        wm.focus_window(Some(2)).unwrap();
        assert_eq!(wl1, wm.get_window_layout());
    }

    // To run these tests, run the command `cargo test` in the `solution`
    // directory.
}
