//! Optional: Multiple Workspaces
//!
//! Extend your window manager with support for multiple workspaces. The
//! concept of workspaces is described in the first section of the assignment.
//! See the documentation of the [`MultiWorkspaceSupport`] trait for the precise
//! requirements.
//!
//! *Unlike* the previous assignments, you are not allowed to make a copy of
//! your previous window manager. You *have* to define a wrapper implementing
//! the [`MultiWorkspaceSupport`] trait. This wrapper can take any existing
//! window manager and uses it to create the different workspaces. This
//! wrapper must also implement all the traits you have implemented in the
//! other assignments, you can forward them to the window manager of the
//! current workspace.
//!
//! [`MultiWorkspaceSupport`]: ../../cplwm_api/wm/trait.MultiWorkspaceSupport.html
//!
//! # Status
//!
//! COMPLETED: YES
//!
//! COMMENTS:
//!
//! ...
//!

// Add imports here

use cplwm_api::types::*;
use cplwm_api::wm::{FloatSupport, FullscreenSupport, MinimiseSupport, MultiWorkspaceSupport,
                    TilingSupport, WindowManager};
use e_fullscreen_windows::FullWM;
use wm_error::WMError;

/// Name of the WM
pub type WMName = MultiWorkspaceWM;
/// Window Manager to extend
pub type WM = FullWM;

/// Window Manager that supports multi workspaces
#[derive(RustcDecodable, RustcEncodable, Debug, Clone)]
pub struct MultiWorkspaceWM {
    /// `Vec` of the different workspaces
    workspaces: Vec<WM>,
    /// Index of the current workspaces
    index: WorkspaceIndex,
}

impl WindowManager for MultiWorkspaceWM {
    type Error = WMError;

    /// create MAX_WORKSPACE_INDEX + 1 workspaces, then initialize the WM with 0 as current workspace index
    fn new(screen: Screen) -> MultiWorkspaceWM {
        let mut wms: Vec<WM> = Vec::new();
        let mut i = 0;
        loop {
            wms.push(WM::new(screen));
            if i == MAX_WORKSPACE_INDEX {
                break;
            }
            i += 1;
        }
        MultiWorkspaceWM {
            workspaces: wms,
            index: 0,
        }
    }

    /// Return all the windows managed by the current workspace.
    /// Retrieve all the windows from each workspace.
    fn get_windows(&self) -> Vec<Window> {
        let mut windows: Vec<Window> = Vec::new();
        let mut partial: Vec<Window>;
        for i in 0..MAX_WORKSPACE_INDEX {
            partial = self.workspaces[i].get_windows().clone();
            windows.append(&mut partial);
        }
        windows
    }

    /// Get the focused window of the current workspace.
    fn get_focused_window(&self) -> Option<Window> {
        self.workspaces[self.index].get_focused_window()
    }

    /// Add a window to the WM. First check if its already managed, if not add it.
    fn add_window(&mut self, window_with_info: WindowWithInfo) -> Result<(), Self::Error> {
        if self.is_managed(window_with_info.window) {
            return Err(WMError::AlreadyManagedWindow(window_with_info.window));
        }
        self.workspaces[self.index].add_window(window_with_info)
    }

    /// If the window is managed find it in the different workspaces and remove it from the
    /// workspaces.
    fn remove_window(&mut self, window: Window) -> Result<(), Self::Error> {
        if !self.is_managed(window) {
            return Err(WMError::UnknownWindow(window));
        }

        for i in 0..MAX_WORKSPACE_INDEX {
            if self.workspaces[i].is_managed(window) {
                self.workspaces[i].remove_window(window).unwrap();
                break;
            }
        }
        Ok(())
    }

    /// Return the WindowLayout of the current workspace.
    fn get_window_layout(&self) -> WindowLayout {
        self.workspaces[self.index].get_window_layout()
    }

    /// If the given window is `None` remove the focus from the given workspace
    /// If it is `Some`, if it's not managed return an error.
    /// Otherwise find the workspace that contains it and call focus_window on that workspace.
    /// If it's not the current one switch to that workspace.
    fn focus_window(&mut self, window: Option<Window>) -> Result<(), Self::Error> {
        if window.is_none() {
            return self.workspaces[self.index].focus_window(window);
        }
        if !self.is_managed(window.unwrap()) {
            return Err(WMError::UnknownWindow(window.unwrap()));
        }

        for i in 0..MAX_WORKSPACE_INDEX {
            if self.workspaces[i].is_managed(window.unwrap()) {
                self.workspaces[i].focus_window(window).unwrap();
                if i != self.index {
                    self.switch_workspace(i).unwrap();
                    break;
                }
            }
        }
        Ok(())
    }

    /// Cycle the focus on the current workspace
    fn cycle_focus(&mut self, dir: PrevOrNext) {
        self.workspaces[self.index].cycle_focus(dir)
    }

    /// Get the info (WindowWithInfo) belonging to the given window.
    fn get_window_info(&self, window: Window) -> Result<WindowWithInfo, Self::Error> {
        if !self.is_managed(window) {
            return Err(WMError::UnknownWindow(window));
        }
        let index = self.find_index(window);
        self.workspaces[index].get_window_info(window)
    }

    /// Return the screen managed by the window manager.
    fn get_screen(&self) -> Screen {
        self.workspaces[self.index].get_screen()
    }

    /// Resize the screen according to the given Screen in every workspace.
    fn resize_screen(&mut self, screen: Screen) {
        for i in 0..(MAX_WORKSPACE_INDEX + 1) {
            self.workspaces[i].resize_screen(screen);
        }
    }
}

impl TilingSupport for MultiWorkspaceWM {
    /// Return the master window of the current workspace.
    fn get_master_window(&self) -> Option<Window> {
        self.workspaces[self.index].get_master_window()
    }

    /// Call `swap_with_master` on the current workspace.
    fn swap_with_master(&mut self, window: Window) -> Result<(), Self::Error> {
        self.workspaces[self.index].swap_with_master(window)
    }
    /// Call `swap_windows` on the current workspace.
    fn swap_windows(&mut self, dir: PrevOrNext) {
        self.workspaces[self.index].swap_windows(dir);
    }
}

impl FloatSupport for MultiWorkspaceWM {
    /// Return the VISIBLE floating windows of all the workspaces.
    fn get_floating_windows(&self) -> Vec<Window> {
        let mut floats: Vec<Window> = Vec::new();
        for i in 0..MAX_WORKSPACE_INDEX {
            let mut current = self.workspaces[i].get_floating_windows().clone();
            floats.append(&mut current);
        }
        floats
    }

    /// If the window is managed call `toggle_floating` on the workspace that manages it.
    fn toggle_floating(&mut self, window: Window) -> Result<(), Self::Error> {
        if !self.is_managed(window) {
            return Err(WMError::UnknownWindow(window));
        }
        let index = self.find_index(window);
        self.workspaces[index].toggle_floating(window)
    }
    /// If the window is managed call `set_window_geometry` on the workspaces that manages it.
    fn set_window_geometry(&mut self,
                           window: Window,
                           new_geometry: Geometry)
                           -> Result<(), Self::Error> {

        if !self.is_managed(window) {
            return Err(WMError::UnknownWindow(window));
        }
        let index = self.find_index(window);
        self.workspaces[index].set_window_geometry(window, new_geometry)
    }
}

impl MinimiseSupport for MultiWorkspaceWM {
    /// Return all the minimised window.
    fn get_minimised_windows(&self) -> Vec<Window> {
        let mut min: Vec<Window> = Vec::new();
        for i in 0..MAX_WORKSPACE_INDEX {
            let mut current = self.workspaces[i].get_minimised_windows().clone();
            min.append(&mut current);
        }
        min
    }

    /// Call `toggle_minimised` on the workspaces that contains the window.
    /// If the window is going to be unminimised and the workspace that contains it is not the
    /// current one, switch to that workspace.
    fn toggle_minimised(&mut self, window: Window) -> Result<(), Self::Error> {
        if !self.is_managed(window) {
            return Err(WMError::UnknownWindow(window));
        }
        let index = self.find_index(window);
        if index != self.index && self.is_minimised(window) {
            self.switch_workspace(index).unwrap();
        }

        self.workspaces[index].toggle_minimised(window)
    }
}

impl FullscreenSupport for MultiWorkspaceWM {
    /// Return the fullscreen window of the current workspace.
    fn get_fullscreen_window(&self) -> Option<Window> {
        self.workspaces[self.index].get_fullscreen_window()
    }
    /// If the window is not managed return an error.
    /// If it's the current fullscreen window, toggle it.
    /// Otherwise find the workspace that manages it and call the `toggle_fullscreen` on that
    /// workspace. If the workspace is different from the current one swith to that workspace.
    fn toggle_fullscreen(&mut self, window: Window) -> Result<(), Self::Error> {
        if !self.is_managed(window) {
            return Err(WMError::UnknownWindow(window));
        } else if self.get_fullscreen_window() == Some(window) {
            self.workspaces[self.index].toggle_fullscreen(window)
        } else {
            let index = self.find_index(window);
            if index != self.index {
                self.switch_workspace(index).unwrap();
            }
            self.workspaces[index].toggle_fullscreen(window)
        }
    }
}

impl MultiWorkspaceSupport<WM> for MultiWorkspaceWM {
    /// Return the index of the current workspace.
    fn get_current_workspace_index(&self) -> WorkspaceIndex {
        self.index
    }

    /// Return an error if index exceeds the `MAX_WORKSPACE_INDEX`.
    /// Otherwise return the current workspace.
    fn get_workspace(&self, index: WorkspaceIndex) -> Result<&WM, Self::Error> {
        if index > MAX_WORKSPACE_INDEX {
            return Err(WMError::WorkspaceIndexNotValid(index));
        }
        Ok(&self.workspaces[index])
    }

    /// Return an error if index exceeds the `MAX_WORKSPACE_INDEX`.
    /// Otherwise return the current workspace as mutable.
    fn get_workspace_mut(&mut self, index: WorkspaceIndex) -> Result<&mut WM, Self::Error> {
        if index > MAX_WORKSPACE_INDEX {
            return Err(WMError::WorkspaceIndexNotValid(index));
        }
        Ok(&mut self.workspaces[index])
    }

    /// Return an error if index exceeds the `MAX_WORKSPACE_INDEX`.
    /// If the workspace is the same as the current one, do nothing.
    /// If is different, if there is a fullscreen window, toggle it (to respect the invariant).
    fn switch_workspace(&mut self, index: WorkspaceIndex) -> Result<(), Self::Error> {
        if index > MAX_WORKSPACE_INDEX {
            return Err(WMError::WorkspaceIndexNotValid(index));
        } else if index == self.index {
            return Ok(());
        }
        if self.get_fullscreen_window().is_some() {
            let fullscreen = self.get_fullscreen_window().unwrap();
            self.toggle_fullscreen(fullscreen).unwrap();
        }
        self.index = index;
        Ok(())
    }
}

impl MultiWorkspaceWM {
    /// Helper function to find the index of the workspace that contain the given window.
    fn find_index(&self, window: Window) -> WorkspaceIndex {
        let mut index = 0;
        for i in 0..MAX_WORKSPACE_INDEX {
            if self.workspaces[i].is_managed(window) {
                index = i;
                break;
            }
        }
        return index;
    }
}

#[cfg(test)]
mod tests {

    use super::WMName;
    use cplwm_api::wm::{FloatSupport, FullscreenSupport, MinimiseSupport, MultiWorkspaceSupport,
                        WindowManager};
    use cplwm_api::types::*;

    // We define a static variable for the screen we will use in the tests.
    // You can just as well define it as a local variable in your tests.
    static SCREEN: Screen = Screen {
        width: 800,
        height: 600,
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
        let mut wm = WMName::new(SCREEN);

        // current ws should be 0
        assert_eq!(wm.get_current_workspace_index(), 0);
        // let's switch to a new workspace
        wm.switch_workspace(1).unwrap();
        // current ws should be 1
        assert_eq!(wm.get_current_workspace_index(), 1);
        assert!(wm.switch_workspace(MAX_WORKSPACE_INDEX + 1).is_err());
        assert!(wm.get_workspace_mut(MAX_WORKSPACE_INDEX + 1).is_err());

        // add a new window to the current workspace
        wm.add_window(WindowWithInfo::new_tiled(1, SOME_GEOM)).unwrap();
        assert_eq!(wm.get_focused_window(), Some(1));
        // switch to workspace 0
        wm.switch_workspace(0).unwrap();
        // there should not be a focused window
        assert_eq!(wm.get_focused_window(), None);
        // switch to workspace 1 again
        wm.switch_workspace(1).unwrap();
        // the focused window should be 1
        assert_eq!(wm.get_focused_window(), Some(1));

        wm.switch_workspace(0).unwrap();
        // add window 2 to workspace 0
        wm.add_window(WindowWithInfo::new_tiled(2, SOME_GEOM)).unwrap();
        // all the windows should be returned
        assert_eq!(wm.get_windows(), vec![2, 1]);
        // switch to 1
        wm.switch_workspace(1).unwrap();
        // try to add window 2 to a different workspace
        let res = wm.add_window(WindowWithInfo::new_tiled(2, SOME_GEOM));
        // the fn should return an error
        assert!(res.is_err());

        // remove 2 from the WM
        wm.remove_window(2).unwrap();
        assert!(!wm.is_managed(2));
        // remove 1 from the WM
        wm.remove_window(1).unwrap();
        assert!(!wm.is_managed(1));
    }

    #[test]
    fn test_focusing_some_windows() {
        let mut wm = WMName::new(SCREEN);
        // add a new window
        let wi = WindowWithInfo::new_float(1, SOME_GEOM);
        wm.add_window(wi).unwrap();
        // switch to 1
        wm.switch_workspace(1).unwrap();
        // window_with_info should be as wi
        assert_eq!(wm.get_window_info(1).unwrap(), wi);
        // add a new window on a workspace 1
        wm.add_window(WindowWithInfo::new_float(2, SOME_GEOM)).unwrap();
        // focus 1 that is on the workspace 0
        wm.focus_window(Some(1)).unwrap();
        // 1 should be focused
        assert_eq!(wm.get_focused_window(), Some(1));
        // workspace should change
        assert_eq!(wm.get_current_workspace_index(), 0);
        // remove focus
        wm.focus_window(None).unwrap();
        // No focused window
        assert_eq!(wm.get_focused_window(), None);
        // workspace remains the same
        assert_eq!(wm.get_current_workspace_index(), 0);
    }

    #[test]
    fn test_floating_windows() {
        let mut wm = WMName::new(SCREEN);

        // add a floating window 1
        wm.add_window(WindowWithInfo::new_float(1, SOME_GEOM)).unwrap();
        // change workspace
        wm.switch_workspace(1).unwrap();
        // add a floating window 2
        wm.add_window(WindowWithInfo::new_float(2, SOME_GEOM)).unwrap();
        // check all the floating windows
        assert_eq!(wm.get_floating_windows(), vec![1, 2]);
        // toggle floating the 1st window
        wm.toggle_floating(1).unwrap();
        // now 1 should be tiled
        assert!(!wm.is_floating(1));
        // change the geometry of 2
        wm.set_window_geometry(2, SCREEN_GEOM).unwrap();
        // check the geometry
        assert_eq!(wm.get_window_info(2).unwrap().geometry, SCREEN_GEOM);
    }

    #[test]
    fn minimise_some_windows() {
        let mut wm = WMName::new(SCREEN);
        wm.add_window(WindowWithInfo::new_float(1, SOME_GEOM)).unwrap();
        wm.switch_workspace(1).unwrap();
        // add window 2 to workspace 1
        wm.add_window(WindowWithInfo::new_float(2, SOME_GEOM)).unwrap();
        // minimise 1 & 2
        wm.toggle_minimised(1).unwrap();
        wm.toggle_minimised(2).unwrap();
        // minimised windows should be 1 & 2
        assert_eq!(wm.get_minimised_windows(), vec![1, 2]);
        // No focus
        assert_eq!(wm.get_focused_window(), None);

        // Unminimise 1
        wm.toggle_minimised(1).unwrap();
        // workspace should change
        assert_eq!(wm.get_current_workspace_index(), 0);
        // Focus on 1
        assert_eq!(wm.get_focused_window(), Some(1));
    }

    #[test]
    fn focus_fullscreen_windows() {
        let mut wm = WMName::new(SCREEN);

        // add a fulscreen window on ws 0
        wm.add_window(WindowWithInfo::new_fullscreen(1, SOME_GEOM)).unwrap();
        // toggle it
        wm.toggle_fullscreen(1).unwrap();
        // no fullscreen window
        assert_eq!(wm.get_fullscreen_window(), None);

        // change workspace to 1
        wm.switch_workspace(1).unwrap();
        // toggle 1 on ws 0
        wm.toggle_fullscreen(1).unwrap();
        // 1 should be focused
        assert_eq!(wm.get_focused_window(), Some(1));
        // 1 should be the fullscreen window
        assert_eq!(wm.get_fullscreen_window(), Some(1));
        // 0 should be the ws
        assert_eq!(wm.get_current_workspace_index(), 0);

        // add a fullscreen window 2 on ws 0
        wm.add_window(WindowWithInfo::new_fullscreen(2, SOME_GEOM)).unwrap();
        // 2 should be the focused window
        assert_eq!(wm.get_focused_window(), Some(2));
        // minimise 2
        wm.toggle_minimised(2).unwrap();
        // Now the focused window should be 1
        assert_eq!(wm.get_focused_window(), Some(1));
        // change ws to1
        wm.switch_workspace(1).unwrap();
        // unminimise the fullscreen window
        wm.toggle_minimised(2).unwrap();
        // now the fullscreen window should be 2
        assert_eq!(wm.get_fullscreen_window(), Some(2));
    }

    // To run these tests, run the command `cargo test` in the `solution`
    // directory.
}
