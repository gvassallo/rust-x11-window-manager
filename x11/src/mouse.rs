//! Mouse-related methods.

use std::os::raw::{c_int, c_uint};

use super::*;

use cplwm_api::types::{Geometry, Window};
use cplwm_api::wm::{FloatSupport, FullscreenSupport, MinimiseSupport, WindowManager};

use x11_dl::xlib;

/// Mouse-related methods.
impl<WM> X11Backend<WM>
    where WM: WindowManager + FloatSupport + FullscreenSupport + MinimiseSupport
{
    /// Return the absolute pointer position on the screen.
    ///
    /// Pass the focused window as `window`.
    ///
    /// Uses [`XQueryPointer`].
    ///
    /// [`XQueryPointer`]: https://tronche.com/gui/x/xlib/window-information/XQueryPointer.html
    pub fn get_pointer_position(&self, window: Window) -> (c_int, c_int) {
        let mut root_return = 0;
        let mut child_return = 0;
        let mut root_x_return = 0;
        let mut root_y_return = 0;
        let mut win_x_return = 0;
        let mut win_y_return = 0;
        let mut mask_return = 0;
        unsafe {
            (self.xlib.XQueryPointer)(self.display,
                                      window,
                                      &mut root_return,
                                      &mut child_return,
                                      &mut root_x_return,
                                      &mut root_y_return,
                                      &mut win_x_return,
                                      &mut win_y_return,
                                      &mut mask_return);
        }
        trace!("GET: {},{}", win_x_return, win_y_return);
        (root_x_return, root_y_return)
    }

    /// Set the absolute pointer position on the screen.
    ///
    /// Pass the focused window as `window`.
    ///
    /// This function generates motion events.
    ///
    /// Uses [`XWarpPointer`].
    ///
    /// [`XWarpPointer`]: https://tronche.com/gui/x/xlib/input/XWarpPointer.html
    pub fn set_pointer_position(&self, window: Window, x: c_int, y: c_int) {
        trace!("SET: {},{}", x, y);
        unsafe {
            (self.xlib.XWarpPointer)(self.display, 0, window, 0, 0, 0, 0, x, y);
        }
    }


    /// Start dragging the mouse.
    ///
    /// The `while_dragging` function will be repeatedly executed until the
    /// user releases the mouse button.
    fn mouse_drag(&mut self, while_dragging: Box<WhileDragging<WM>>) {
        if self.dragging.is_none() {
            let mask = (xlib::ButtonReleaseMask | xlib::PointerMotionMask) as c_uint;
            unsafe {
                (self.xlib.XGrabPointer)(self.display,
                                         self.root_window,
                                         xlib::False,
                                         mask,
                                         xlib::GrabModeAsync,
                                         xlib::GrabModeAsync,
                                         0,
                                         0,
                                         xlib::CurrentTime)
            };
            self.dragging = Some(while_dragging);
        }
    }

    /// Move the given window with the mouse.
    ///
    /// Does nothing when the given window is not floating.
    ///
    /// The pointer position determines the new position of the window, until
    /// the user releases the pressed mouse button.
    ///
    /// Use this function in a binding for a mouse button.
    pub fn mouse_move_window(&mut self, window: Window) -> X11Result<()>
        where WM: FloatSupport
    {
        if self.get_wm().is_floating(window) {
            let orig_geometry = try!(self.get_window_geometry(window));
            let (start_x, start_y) = self.get_pointer_position(window);
            let while_dragging = move |backend: &mut X11Backend<WM>, moved_x, moved_y| {
                let new_geometry = Geometry {
                    x: orig_geometry.x + (moved_x - start_x),
                    y: orig_geometry.y + (moved_y - start_y),
                    width: orig_geometry.width,
                    height: orig_geometry.height,
                };
                try!(backend.get_wm_mut().set_window_geometry(window, new_geometry));
                Ok(())
            };
            self.mouse_drag(Box::new(while_dragging));
        }
        Ok(())
    }

    /// Resize the given window with the mouse.
    ///
    /// Does nothing when the given window is not floating.
    ///
    /// First, the mouse pointer is moved to the bottom right corner of the
    /// window. From then on, the pointer position determines the new size of
    /// the window, until the user releases the pressed mouse button.
    ///
    /// Use this function in a binding for a mouse button.
    pub fn mouse_resize_window(&mut self, window: Window) -> X11Result<()>
        where WM: FloatSupport
    {
        if self.get_wm().is_floating(window) {
            let orig_geometry = try!(self.get_window_geometry(window));
            self.set_pointer_position(window,
                                      orig_geometry.width as c_int,
                                      orig_geometry.height as c_int);
            let (start_x, start_y) = self.get_pointer_position(window);
            let orig_width = orig_geometry.width as c_int;
            let orig_height = orig_geometry.height as c_int;
            let while_dragging = move |backend: &mut X11Backend<WM>, moved_x, moved_y| {
                let new_geometry = Geometry {
                    x: orig_geometry.x,
                    y: orig_geometry.y,
                    width: (orig_width + (moved_x - start_x)) as c_uint,
                    height: (orig_height + (moved_y - start_y)) as c_uint,
                };
                try!(backend.get_wm_mut().set_window_geometry(window, new_geometry));
                Ok(())
            };
            self.mouse_drag(Box::new(while_dragging));
        }
        Ok(())
    }
}
