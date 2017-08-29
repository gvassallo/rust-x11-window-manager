//! Optional: Fullscreen Windows
//!
//! Extend your window manager with support for fullscreen windows, i.e. the
//! ability to temporarily make a window take up the whole screen, thereby
//! obscuring all other windows. See the documentation of the
//! [`FullscreenSupport`] trait for the precise requirements. Don't confuse
//! this with the first assignment, in which you built a window manager that
//! displayed all windows fullscreen.
//!
//! Like in the previous assignments, either make a copy of, or define a
//! wrapper around your previous window manager to implement the
//! [`FullscreenSupport`] trait as well. Note that this window manager must
//! still implement all the traits from previous assignments.
//!
//! [`FullscreenSupport`]: ../../cplwm_api/wm/trait.FullscreenSupport.html
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
use cplwm_api::wm::{FloatSupport, FullscreenSupport, MinimiseSupport, TilingSupport, WindowManager};
use d_minimising_windows::MinimiseWM;
use wm_error::WMError;

/// The name of the Window Manger
pub type WMName = FullWM;

/// Window Manager that supports fullscreen windows
#[derive(RustcDecodable, RustcEncodable, Debug, Clone)]
pub struct FullWM {
    /// Wrap of the Minimise Window Manager
    pub minimise_wm: MinimiseWM,
    /// The current fullscreen window
    pub fullscreen_window: Option<Window>,
}

impl WindowManager for FullWM {
    type Error = WMError;
    /// The constructor wraps the constructor of the FloatingWM
    /// it also declares the fullscreen_window Option
    fn new(screen: Screen) -> FullWM {
        FullWM {
            minimise_wm: MinimiseWM::new(screen),
            fullscreen_window: None,
        }
    }

    /// The function wraps the old `get_windows`
    fn get_windows(&self) -> Vec<Window> {
        self.minimise_wm.get_windows()
    }

    /// The function wraps the old `get_focused_window`
    fn get_focused_window(&self) -> Option<Window> {
        self.minimise_wm.get_focused_window()
    }

    /// The function return an error if the window is already managed
    ///
    /// Check whether there is a fullscreen window:
    ///
    /// * if there isn't add the window using the wrapped function and if the added window is
    /// fullscreen toggle it
    /// * if there is toggle it, add the window and if the new window is fullscreen toggle it
    fn add_window(&mut self, window_with_info: WindowWithInfo) -> Result<(), Self::Error> {
        // if the window is already managed
        if self.is_managed(window_with_info.window) {
            // return the error from the wrapped function
            return self.minimise_wm.add_window(window_with_info);
        }
        // If there isn't a fullscreen window
        if self.fullscreen_window.is_none() {
            // call the wrapped function
            self.minimise_wm.add_window(window_with_info).unwrap();
            // if the added window is fullscreen
            if window_with_info.fullscreen {
                // toggle it
                self.toggle_fullscreen(window_with_info.window).unwrap();
            }
            return Ok(());
        }
        let fullscreen = self.fullscreen_window.unwrap();
        // toggle the fullscreen window
        self.toggle_fullscreen(fullscreen).unwrap();
        // call the wrapped function
        self.minimise_wm.add_window(window_with_info).unwrap();
        // it the added window is fullscreen
        if window_with_info.fullscreen {
            // make it fullscreen
            self.toggle_fullscreen(window_with_info.window).unwrap();
        }
        Ok(())
    }

    /// Remove the given window from the window manager.
    /// If the window is managed and is fullscreen, first toggle it and then remove the window
    /// calling the wrapped function and set the current fullscreen_window as `None`.
    fn remove_window(&mut self, window: Window) -> Result<(), Self::Error> {
        // if the window is managed and fullscreen
        if self.is_managed(window) && self.fullscreen_window == Some(window) {
            // untoggle the window
            self.toggle_fullscreen(window).unwrap();
            // use the wrapped function to remove it
            self.minimise_wm.remove_window(window).unwrap();
            // set None as the fullscreen window
            self.fullscreen_window = None;
            return Ok(());
        }
        // otherwise remove the window using the wrapped function
        self.minimise_wm.remove_window(window)
    }

    /// If there is a fullscreen window return the layout containing only that window with the
    /// geometry of the screen. Otherwise call the wrapped function.
    fn get_window_layout(&self) -> WindowLayout {
        let fullscreen = self.fullscreen_window;
        if fullscreen.is_none() {
            return self.minimise_wm.get_window_layout();
        }
        // if there is a fullscreen window the layout should contain
        // only that window and as geometry the screen geometry
        WindowLayout {
            focused_window: fullscreen,
            windows: vec![(fullscreen.unwrap(), self.get_screen().to_geometry())],
        }
    }

    /// Focus the given window, or when passed None, focus nothing.
    /// If the window passed is `None` toggle the fullscreen window if exists.
    /// If it is not managed call the wrapped function to return the error.
    /// It is a managed window: if there is a fullscreen window toggle it.
    /// At the end call the wrapped function.
    fn focus_window(&mut self, window: Option<Window>) -> Result<(), Self::Error> {
        if window.is_none() {
            // if the window is none and there is a fullscreen window
            if self.fullscreen_window.is_some() {
                // toggle the fullscreen window
                let fullscreen = self.fullscreen_window.unwrap();
                self.toggle_fullscreen(fullscreen).unwrap();
            }
            // call the wrapped function
            self.minimise_wm.focus_window(window)
        } else if !self.is_managed(window.unwrap()) {
            self.minimise_wm.focus_window(window)
        } else {
            let fullscreen = self.fullscreen_window;
            // if there is a fullscreen window
            if fullscreen.is_some() && fullscreen.unwrap() != window.unwrap() {
                // toggle it
                self.toggle_fullscreen(fullscreen.unwrap()).unwrap();
            }
            // call the wrapped function
            self.minimise_wm.focus_window(window)
        }
    }

    /// Focus the previous or next window.
    /// If the current focused window is fullscreen there can be two cases:
    ///
    /// * There can be only that window, in that case leave the situation as is
    /// * Otherwise toggles the window and call the wrapped function.
    fn cycle_focus(&mut self, dir: PrevOrNext) {
        // if there is a fullscreen window and it's not the only window
        if self.fullscreen_window.is_some() {
            if self.get_windows().len() > 1 {
                let fullscreen = self.fullscreen_window.unwrap();
                // toggle it and proceed
                self.toggle_fullscreen(fullscreen).unwrap();
            } else {
                return;
            }
        }
        // call the wrapped function
        self.minimise_wm.cycle_focus(dir);
    }

    /// Get the info (WindowWithInfo) belonging to the given window.
    fn get_window_info(&self, window: Window) -> Result<WindowWithInfo, Self::Error> {
        self.minimise_wm.get_window_info(window)
    }

    /// Return the screen managed by the window manager.
    fn get_screen(&self) -> Screen {
        self.minimise_wm.get_screen()
    }

    /// Resize the screen according to the given Screen.
    fn resize_screen(&mut self, screen: Screen) {
        self.minimise_wm.resize_screen(screen);
    }
}

impl TilingSupport for FullWM {
    /// Return the window displayed in the master tile.
    /// Call the wrapped function.
    fn get_master_window(&self) -> Option<Window> {
        self.minimise_wm.get_master_window()
    }

    /// Swap the given window with the window in the master tile.
    ///
    /// If the window passed is a tile and is also the fullscreen one, toggle it and call the
    /// wrapped function.
    /// Otherwise if the master tile is the fullscreen one, toggle it and call the wrapped
    /// function.
    /// In the other cases do the swapping in 'background'.
    fn swap_with_master(&mut self, window: Window) -> Result<(), Self::Error> {
        if self.is_managed(window) && self.fullscreen_window.is_some() {
            if self.fullscreen_window.unwrap() == window {
                let master = self.get_master_window();
                if master.is_some() && master.unwrap() == window {
                    return Ok(());
                }
                match self.get_window_info(window).unwrap().float_or_tile {
                    FloatOrTile::Float => return Ok(()), 
                    FloatOrTile::Tile => self.toggle_fullscreen(window).unwrap(),
                };
            } else {
                let master = self.get_master_window();
                if master == self.fullscreen_window {
                    self.toggle_fullscreen(master.unwrap()).unwrap();
                }
            }
        }
        self.minimise_wm.swap_with_master(window)
    }

    /// Swap the focused window with the one in the next or previous tile.
    /// if the focus window is a tile and fullscreen toggle it and call the wrapped function.
    fn swap_windows(&mut self, dir: PrevOrNext) {
        if self.fullscreen_window.is_some() {
            let fullscreen = self.fullscreen_window.unwrap();
            if self.get_window_info(fullscreen).unwrap().float_or_tile == FloatOrTile::Tile {
                self.toggle_fullscreen(fullscreen).unwrap();
            }
        }
        self.minimise_wm.swap_windows(dir)
    }
}

impl FloatSupport for FullWM {
    /// Return the list of all VISIBLE floating windows.
    ///
    /// * If there is a fullscreen window and is floating return only that
    /// * If the fullscreen window is tiled return an empty Vec
    /// * Otherwise call the wrapped function
    fn get_floating_windows(&self) -> Vec<Window> {
        if self.fullscreen_window.is_some() {
            let mut floating: Vec<Window> = Vec::new();
            let full_info = self.get_window_info(self.fullscreen_window.unwrap()).unwrap();
            if full_info.float_or_tile == FloatOrTile::Float {
                floating.push(self.fullscreen_window.unwrap());
            }
            return floating;
        }
        self.minimise_wm.get_floating_windows()
    }

    /// If the given window is floating, let it sink, if it is not floating, let it float.
    /// If there is a fullscreen window toggle it (to maintain the invariants)
    fn toggle_floating(&mut self, window: Window) -> Result<(), Self::Error> {
        if self.is_managed(window) && self.fullscreen_window.is_some() {
            self.toggle_fullscreen(window).unwrap();
        }
        self.minimise_wm.toggle_floating(window)
    }

    /// Resize/move the given floating window according to the given geometry.
    /// If the given window is the fullscreen one and it is float, toggle it.
    /// Otherwise do the resize in 'background'.
    fn set_window_geometry(&mut self,
                           window: Window,
                           new_geometry: Geometry)
                           -> Result<(), Self::Error> {
        let res = try!(self.minimise_wm.set_window_geometry(window, new_geometry));

        if self.fullscreen_window.is_some() && self.fullscreen_window.unwrap() == window {
            if self.is_floating(window) {
                self.toggle_fullscreen(window).unwrap();
            }
        }
        Ok(res)
    }
}

impl MinimiseSupport for FullWM {
    /// Call the wrapped function
    fn get_minimised_windows(&self) -> Vec<Window> {
        self.minimise_wm.get_minimised_windows()
    }
    /// If the given window is unminimised and the fullscreen one, toggle it and call the wrapped
    /// function
    fn toggle_minimised(&mut self, window: Window) -> Result<(), Self::Error> {
        if self.is_managed(window) {
            if !self.is_minimised(window) {
                // if it's the fullscreen window the one to minimise
                if self.fullscreen_window.is_some() && self.fullscreen_window.unwrap() == window {
                    self.toggle_fullscreen(window).unwrap();
                    let mut window_with_info = self.get_window_info(window).unwrap();
                    window_with_info.fullscreen = true;
                }
            } else {
                // if it's minimised
                let window_with_info = self.get_window_info(window).unwrap();
                if window_with_info.fullscreen {
                    if self.fullscreen_window.is_some() {
                        let fullscreen = self.fullscreen_window;
                        self.toggle_fullscreen(fullscreen.unwrap()).unwrap();
                    }
                    self.toggle_fullscreen(window).unwrap();
                }
            }
        }
        self.minimise_wm.toggle_minimised(window)
    }
}

impl FullscreenSupport for FullWM {
    /// Return the fullscreen_window `Option`.
    fn get_fullscreen_window(&self) -> Option<Window> {
        self.fullscreen_window
    }
    /// Make the given window fullscreen, or when it is already fullscreen, undo it.
    /// If there is a fullscreen window already:
    ///
    /// * if it's the current window remove it as fullscreen window and modify its info
    /// * otherwise change the info of the old one assigning false to the fullscreen attribute of
    ///   the WindowWithInfo, do the opposite for the given window and assign the given window to the
    ///   `fullscreen_window` of the WM.
    ///
    /// If there isn't already a fullscreen window, assign the current one.
    fn toggle_fullscreen(&mut self, window: Window) -> Result<(), Self::Error> {
        if !self.is_managed(window) {
            return Err(WMError::UnknownWindow(window));
        }
        let fullscreen = self.fullscreen_window;
        // if there is a fullscreen window
        if fullscreen.is_some() {
            // if it's the current one
            if fullscreen.unwrap() == window {
                let mut window_with_info = self.get_window_info(fullscreen.unwrap()).unwrap();
                // modify the fullscreen info of the struct
                window_with_info.fullscreen = false;
                self.fullscreen_window = None;
            } else {
                // if there is already a fullscreen window
                let mut old_window_with_info = self.get_window_info(fullscreen.unwrap()).unwrap();
                old_window_with_info.fullscreen = false;
                let mut window_with_info = self.get_window_info(window).unwrap();
                window_with_info.fullscreen = true;
                // assign the new fullscreen window
                self.fullscreen_window = Some(window);
                // focus it
                self.focus_window(Some(window)).unwrap();
            }
        } else {
            // if there isn't a fullscreen window
            let mut window_with_info = self.get_window_info(window).unwrap();
            window_with_info.fullscreen = true;
            // assign the current one
            self.fullscreen_window = Some(window);
            // focus it
            self.focus_window(Some(window)).unwrap();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::WMName;
    use cplwm_api::wm::{FloatSupport, FullscreenSupport, MinimiseSupport, TilingSupport,
                        WindowManager};
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

    #[test]
    fn focus_fullscreen_windows() {
        let mut wm = WMName::new(SCREEN);

        // add a new fullscreen window
        wm.add_window(WindowWithInfo::new_fullscreen(1, SOME_GEOM)).unwrap();
        let wl1 = wm.get_window_layout();
        // the geometry of the window should be the SCREEN_GEOM
        assert_eq!(wl1.windows, vec![(1, SCREEN_GEOM)]);
        // toggle the window
        wm.toggle_fullscreen(1).unwrap();
        // now the window should be tiled, and the master window
        assert_eq!(wm.get_master_window().unwrap(), 1);
        // make the window fullscreen again
        wm.toggle_fullscreen(1).unwrap();
        // add another fullscreen window
        wm.add_window(WindowWithInfo::new_fullscreen(2, SOME_GEOM)).unwrap();
        // now the 2nd should be the fullscreen one
        assert_eq!(wm.get_fullscreen_window(), Some(2));
        wm.toggle_fullscreen(2).unwrap();
        // now there should not be a fullscreen window
        assert_eq!(wm.get_fullscreen_window(), None);
        wm.toggle_fullscreen(2).unwrap();
        // remove the fullscreen window
        wm.remove_window(2).unwrap();
        // now there should not be a fullscreen window
        assert_eq!(wm.get_fullscreen_window(), None);

        // add a new fullscreen window
        wm.add_window(WindowWithInfo::new_fullscreen(2, SOME_GEOM)).unwrap();
        // cycle the focus
        wm.cycle_focus(PrevOrNext::Prev);
        // no window should be fullscreen
        assert_eq!(wm.get_fullscreen_window(), None);

        // make the 2nd fullscreen again
        wm.toggle_floating(2).unwrap();
        // focus the 1st
        wm.focus_window(Some(1)).unwrap();
        // no window should be fullscreen
        assert_eq!(wm.get_fullscreen_window(), None);

        // make the 2nd fullscreen again
        wm.toggle_floating(2).unwrap();
        // remove the focus
        wm.focus_window(None).unwrap();
        // no window should be fullscreen
        assert_eq!(wm.get_fullscreen_window(), None);
    }

    #[test]
    fn swap_fullscreen_windows() {
        let mut wm = WMName::new(SCREEN);

        // add a fullscreen
        wm.add_window(WindowWithInfo::new_fullscreen(1, SOME_GEOM)).unwrap();
        // add a window
        wm.add_window(WindowWithInfo::new_tiled(2, SOME_GEOM)).unwrap();
        // no fullscreen after adding a non fullscreen window
        assert_eq!(wm.get_fullscreen_window(), None);

        // make 2nd fullscreen
        wm.toggle_fullscreen(2).unwrap();
        // swap it with master
        wm.swap_with_master(2).unwrap();
        assert_eq!(wm.get_master_window(), Some(2));
        // no fullscreen window
        assert_eq!(wm.get_fullscreen_window(), None);

        wm.toggle_fullscreen(2).unwrap();
        // swap the master with the master
        wm.swap_with_master(2).unwrap();
        // nothing should happen
        assert_eq!(wm.get_fullscreen_window(), Some(2));

        wm.toggle_fullscreen(2).unwrap();
        wm.swap_windows(PrevOrNext::Prev);
        assert_eq!(wm.get_fullscreen_window(), None);
    }

    #[test]
    fn float_fullscreen_windows() {
        let mut wm = WMName::new(SCREEN);

        wm.add_window(WindowWithInfo::new_tiled(1, SOME_GEOM)).unwrap();
        wm.add_window(WindowWithInfo::new_float(2, SOME_GEOM)).unwrap();
        // 1st tile fullscreen
        wm.toggle_fullscreen(1).unwrap();
        // no floating window should returned
        assert_eq!(wm.get_floating_windows(), Vec::new());
        // make 1st floating
        wm.toggle_floating(1).unwrap();
        // make 1st fullscreen again
        wm.toggle_fullscreen(1).unwrap();
        // it should return only the fullscreen one
        assert_eq!(wm.get_floating_windows(), vec![1]);
        // resize the fullscreen window
        wm.set_window_geometry(1, SOME_GEOM).unwrap();
        // the window should not be fullscreen anymore
        assert_eq!(wm.get_fullscreen_window(), None);
    }

    #[test]
    fn minimise_fullscreen_windows() {
        let mut wm = WMName::new(SCREEN);
        // let's add a new fullscreen window
        wm.add_window(WindowWithInfo::new_fullscreen(1, SOME_GEOM)).unwrap();
        // let's minimise it
        wm.toggle_minimised(1).unwrap();
        // there should not be a fullscreen window
        assert_eq!(wm.get_fullscreen_window(), None);
        // let's make the minimised window fullscreen
        wm.toggle_fullscreen(1).unwrap();
        // 1 should be the fullscreen one
        assert_eq!(wm.get_fullscreen_window(), Some(1));

        // minimise the fullscreen window
        wm.toggle_minimised(1).unwrap();
        // unminimise it
        wm.toggle_minimised(1).unwrap();
        // the window should be fullscreen again
        assert_eq!(wm.get_fullscreen_window(), Some(1));

        // add a new fullscreen window
        wm.add_window(WindowWithInfo::new_fullscreen(2, SOME_GEOM)).unwrap();
        assert_eq!(wm.get_fullscreen_window(), Some(2));
        // minimise the fullscreen window
        wm.toggle_minimised(2).unwrap();
        // toggle the 1st window
        wm.toggle_fullscreen(1).unwrap();
        // unminimise the 2nd window again
        wm.toggle_minimised(2).unwrap();
        // it should return fullscreen
        assert_eq!(wm.get_fullscreen_window(), Some(2));
    }
    // To run these tests, run the command `cargo test` in the `solution`
    // directory.
}
