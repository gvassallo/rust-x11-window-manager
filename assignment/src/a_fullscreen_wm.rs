//! Fullscreen Window Manager
//!
//! Implement the [`WindowManager`] trait by writing a simple window manager
//! that displays every window fullscreen. When a new window is added, the
//! last window that was visible will become invisible.
//!
//! [`WindowManager`]: ../../cplwm_api/wm/trait.WindowManager.html
//!
//! Now have a look at the source code of this file, it contains a tutorial to
//! help you write the fullscreen window manager.
//!
//! You are free to remove the documentation in this file that is only part of
//! the tutorial or no longer matches the code after your changes.
//!
//! # Status
//!
//! COMPLETED: YES
//!
//! COMMENTS:
//!
//! ...
//!

// Import some types and the WindowManager trait from the cplwm_api crate
// (defined in the api folder).
use cplwm_api::types::{PrevOrNext, Screen, Window, WindowLayout, WindowWithInfo};
use cplwm_api::wm::WindowManager;

use wm_error::WMError;

use std::collections::VecDeque;

/// You are free to choose the name for your window manager. As we will use
/// automated tests when grading your assignment, indicate here the name of
/// your window manager data type so we can just use `WMName` instead of
/// having to manually figure out your window manager name.
pub type WMName = FullscreenWM;


/// The FullscreenWM struct
#[derive(RustcDecodable, RustcEncodable, Debug, Clone)]
pub struct FullscreenWM {
    /// A VecDeque of windows, the first one is on the front, the last one is
    /// on back, and also the only visible window.
    pub windows: VecDeque<Window>,
    /// We need to know which size the fullscreen window must be.
    pub screen: Screen,
    /// Boolean variable to indicate if there is a focused window
    pub is_focus: bool,
}

// Now we start implementing our window manager
impl WindowManager for FullscreenWM {
    /// We use `WMError` as our `Error` type.
    type Error = WMError;

    /// The constructor is straightforward.
    ///
    /// Track the given screen and make a new empty `VecDeque`.
    fn new(screen: Screen) -> FullscreenWM {
        FullscreenWM {
            windows: VecDeque::new(),
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

    /// The last window in the VecDeque `windows` is the focused one.
    fn get_focused_window(&self) -> Option<Window> {
        // if there is focus return the last window of the VecDeque
        if self.is_focus {
            // if there is no window in the vec, back() function returns None
            self.windows.back().map(|x| *x)
        }
        // otherwise return None
        else {
            None
        }
    }

    /// To add a window, just push it onto the end the `windows` `VecDeque`.
    ///
    /// The function returns an error if the window is already managed by the
    /// window manager.
    fn add_window(&mut self, window_with_info: WindowWithInfo) -> Result<(), Self::Error> {
        if !self.is_managed(window_with_info.window) {
            self.windows.push_back(window_with_info.window);
            // set the boolean is_focus to true
            self.is_focus = true;
            Ok(())
        } else {
            // if the window is already managed return the error
            Err(WMError::AlreadyManagedWindow(window_with_info.window))
        }
    }

    /// To remove a window, just remove it from the `windows` `VecDeque`.
    ///
    /// First we look up the position (or index) of the window in `windows`,
    /// and then remove it unless the window does not occur in the `VecDeque`, in
    /// which case we return an error.
    fn remove_window(&mut self, window: Window) -> Result<(), Self::Error> {
        match self.windows.iter().position(|w| *w == window) {
            None => Err(WMError::UnknownWindow(window)),
            Some(i) => {
                self.windows.remove(i);
                // if there are no more windows, then there is no focus
                if self.windows.is_empty() {
                    self.is_focus = false;
                }
                Ok(())
            }
        }
    }

    /// Now the most important part: calculating the `WindowLayout`.
    ///
    /// First we build a `Geometry` for a fullscreen window using the
    /// `to_geometry` method: it has the same width and height as the screen.
    ///
    /// * When the `Option` contains `Some(w)`, we know that there was at
    ///   least one window, and `w`, being the last window in the `Vec` should
    ///   be focused. As the other windows will not be visible, the `windows`
    ///   field of `WindowLayout` can just be a `Vec` with one element: the
    ///   one window along with the fullscreen `Geometry`.
    ///
    /// * When the `Option` is `None`, we know that there are no windows, so
    ///   we can just return an empty `WindowLayout`.
    ///
    fn get_window_layout(&self) -> WindowLayout {
        let fullscreen_geometry = self.screen.to_geometry();
        match self.windows.back() {
            // If there is at least one window.
            Some(w) => {
                let mut focused = Some(*w);
                if !self.is_focus {
                    focused = None;
                }
                WindowLayout {
                    // focued window, otherwise None
                    focused_window: focused,
                    // ... and should fill the screen. The other windows are
                    // simply hidden.
                    windows: vec![(*w, fullscreen_geometry)],
                }
            }
            // Otherwise, return an empty WindowLayout
            None => WindowLayout::new(),
        }
    }

    /// Focus the given window, or when passed None, focus nothing.
    fn focus_window(&mut self, window: Option<Window>) -> Result<(), Self::Error> {
        match window {
            // if None is passed, set also the boolean is_focus to false
            None => {
                self.is_focus = false;
                Ok(())
            } 
            Some(_) => {
                match self.windows.iter().position(|w| *w == window.unwrap()) {
                    None => Err(WMError::UnknownWindow(window.unwrap())),
                    Some(i) => {
                        self.is_focus = true;
                        // move the given window to the last position on the VecDeque
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
    /// If there are no windows do nothing.
    /// If there is only 1 window set only is_focus to true.
    /// If there are two window swap them and set the focus.
    /// If there are more then 2 window swap the last window in the VecDeque with the previous/next
    /// one(first in the buffer).
    fn cycle_focus(&mut self, dir: PrevOrNext) {
        let len = self.windows.len();
        match len {
            0 => return,
            1 => {} 
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
        // at this point one window is focused
        self.is_focus = true;
    }

    /// Get the info (WindowWithInfo) belonging to the given window.
    /// Return a tiled window, with the screen geometry.
    fn get_window_info(&self, window: Window) -> Result<WindowWithInfo, Self::Error> {
        match self.windows.iter().position(|w| *w == window) {
            None => Err(WMError::UnknownWindow(window)),
            Some(_) => Ok(WindowWithInfo::new_tiled(window, self.screen.to_geometry())),
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

// Here we define a submodule, called `tests`, that will contain the unit
// tests of this module.
//
// The `#[cfg(test)]` annotation means that this code is only compiled when
// we're testing the code.
#[cfg(test)]
mod tests {

    // We have to import `FullscreenWM` from the super module.
    use super::FullscreenWM;
    // We have to repeat the imports we did in the super module.
    use cplwm_api::wm::WindowManager;
    use cplwm_api::types::*;

    // We define a static variable for the screen we will use in the tests.
    // You can just as well define it as a local variable in your tests.
    static SCREEN: Screen = Screen {
        width: 800,
        height: 600,
    };

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
        let mut wm = FullscreenWM::new(SCREEN);

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
        // and fullscreen.
        assert_eq!(vec![(2, SCREEN_GEOM)], wl2.windows);

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
        // Now let's resize the screen
        wm.resize_screen(RESIZED);
        // Get the new screen from wv
        let screen = wm.get_screen();
        // screen should be equal to RESIZED screen
        assert_eq!(RESIZED, screen);


    }

    #[test]
    fn test_focusing_some_windows() {
        // Let's make a new `FullscreenWM` with `SCREEN` as screen.
        let mut wm = FullscreenWM::new(SCREEN);

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

    // To run these tests, run the command `cargo test` in the `solution`
    // directory.
}
