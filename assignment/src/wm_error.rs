use std::error;
use std::fmt;

use cplwm_api::types::{Window, WorkspaceIndex};

/// The errors that this window manager can return.
///
/// For more information about why you need this, read the documentation of
/// the associated [Error] type of the `WindowManager` trait.
///
/// In the code below, we would like to return an error when we are asked to
/// do something with a window that we do not manage, so we define an enum
/// `WMError` with the first variant: `UnknownWindow`.
///
/// The second one is used when an already managed window is being added.
#[derive(Debug)]
pub enum WMError {
    /// This window is not known by the window manager.
    UnknownWindow(Window),
    /// This windows is already managed by the window manager.
    AlreadyManagedWindow(Window),
    /// The workspace index is not valid.
    WorkspaceIndexNotValid(WorkspaceIndex),
}

// This code is explained in the documentation of the associated [Error] type
// of the `WindowManager` trait.
impl fmt::Display for WMError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            WMError::UnknownWindow(ref window) => write!(f, "Unknown window: {}", window),
            WMError::AlreadyManagedWindow(ref window) => {
                write!(f, "Already managed window: {}", window)
            } 
            WMError::WorkspaceIndexNotValid(ref index) => {
                write!(f, "The workspace index is not valid: {}", index)
            }
        }
    }
}

// This code is explained in the documentation of the associated [Error] type
// of the `WindowManager` trait.
impl error::Error for WMError {
    fn description(&self) -> &'static str {
        match *self {
            WMError::UnknownWindow(_) => "Unknown window",
            WMError::AlreadyManagedWindow(_) => "Already managed window",
            WMError::WorkspaceIndexNotValid(_) => "Workspace index not valid", 
        }
    }
}
