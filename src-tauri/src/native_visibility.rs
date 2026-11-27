//! Read-only native window-system evidence, executed on the existing child
//! IPC worker after layout/show/hide. No page JS, pixels or desktop capture.
//! X11 viewability and a DWM barrier do not establish webpage paint or
//! unobscured monitor scanout. Keep that final GUI acceptance separate.
use crate::grid_ipc::{IpcRect, NativeWindowReceipt, NativeWindowSource};
use std::time::{Duration, Instant};

/// Retry observations only, never issue a second layout/show/hide operation.
/// The parent still owns the IPC deadline and retirement on an unknown result.
/// The local window-system round trip itself may stall on a broken server.
fn observe_until(
    expected: Option<IpcRect>,
    mut observe: impl FnMut() -> Result<NativeWindowReceipt, String>,
) -> Result<NativeWindowReceipt, String> {
    let deadline = Instant::now() + Duration::from_millis(400);
    loop {
        let receipt = observe()?;
        match receipt.validate_for(expected) {
            Ok(()) => return Ok(receipt),
            Err(error) if Instant::now() >= deadline => return Err(error),
            Err(_) => std::thread::sleep(Duration::from_millis(20)),
        }
    }
}

#[cfg(target_os = "linux")]
fn x11_visibility_for_request(state: x11rb::protocol::xproto::MapState, show: bool) -> bool {
    use x11rb::protocol::xproto::MapState;
    if show {
        state == MapState::VIEWABLE
    } else {
        state != MapState::UNMAPPED
    }
}

#[cfg(all(test, target_os = "linux"))]
mod x11_tests {
    use super::*;
    use x11rb::protocol::xproto::MapState;
    #[test]
    fn mapped_but_unviewable_is_not_an_explicit_hide() {
        assert!(x11_visibility_for_request(MapState::UNVIEWABLE, false));
        assert!(!x11_visibility_for_request(MapState::UNVIEWABLE, true));
        assert!(!x11_visibility_for_request(MapState::UNMAPPED, false));
        assert!(x11_visibility_for_request(MapState::VIEWABLE, true));
    }
}

#[cfg(target_os = "linux")]
pub(crate) fn confirm(
    window: &tauri::Window,
    expected: Option<IpcRect>,
) -> Result<NativeWindowReceipt, String> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use x11rb::protocol::xproto::{ConnectionExt, MapState};
    let handle = window
        .window_handle()
        .map_err(|_| "GRID_X11_HANDLE_UNAVAILABLE")?;
    let xid = match handle.as_raw() {
        RawWindowHandle::Xlib(value) => {
            u32::try_from(value.window).map_err(|_| "GRID_X11_HANDLE_INVALID")?
        }
        RawWindowHandle::Xcb(value) => value.window.get(),
        _ => return Err("GRID_X11_BACKEND_UNSUPPORTED".into()),
    };
    if xid == 0 {
        return Err("GRID_X11_HANDLE_INVALID".into());
    }
    // The app forces GDK_BACKEND=x11 for both the main and child hosts.
    // Use an independent X connection to observe the server's actual state,
    // not GTK's requested visibility or a cached client property.
    let (connection, _) = x11rb::connect(None).map_err(|_| "GRID_X11_CONNECTION_UNAVAILABLE")?;
    observe_until(expected, || {
        let attrs = connection
            .get_window_attributes(xid)
            .map_err(|_| "GRID_X11_VISIBILITY_UNAVAILABLE")?
            .reply()
            .map_err(|_| "GRID_X11_VISIBILITY_UNAVAILABLE")?;
        // UNVIEWABLE means this window is still mapped, but an ancestor
        // is not viewable. A hide receipt requires our own window to be
        // unmapped, not merely obscured by an ancestor's lifecycle.
        // A mismatched observation is retried by observe_until and never
        // published as a successful receipt.
        let visible = x11_visibility_for_request(attrs.map_state, expected.is_some());
        let rect = if attrs.map_state == MapState::VIEWABLE {
            let geometry = connection
                .get_geometry(xid)
                .map_err(|_| "GRID_X11_GEOMETRY_UNAVAILABLE")?
                .reply()
                .map_err(|_| "GRID_X11_GEOMETRY_UNAVAILABLE")?;
            let origin = connection
                .translate_coordinates(xid, geometry.root, 0, 0)
                .map_err(|_| "GRID_X11_GEOMETRY_UNAVAILABLE")?
                .reply()
                .map_err(|_| "GRID_X11_GEOMETRY_UNAVAILABLE")?;
            if !origin.same_screen {
                return Err("GRID_X11_SCREEN_MISMATCH".into());
            }
            Some(IpcRect {
                x: f64::from(origin.dst_x),
                y: f64::from(origin.dst_y),
                w: f64::from(geometry.width),
                h: f64::from(geometry.height),
            })
        } else {
            None
        };
        Ok(NativeWindowReceipt {
            visible,
            source: NativeWindowSource::X11Server,
            rect,
            compositor_barrier: false,
        })
    })
}

#[cfg(target_os = "windows")]
pub(crate) fn confirm(
    window: &tauri::Window,
    expected: Option<IpcRect>,
) -> Result<NativeWindowReceipt, String> {
    use windows_sys::Win32::{
        Foundation::{HWND, RECT},
        Graphics::Dwm::{DwmFlush, DwmGetWindowAttribute, DWMWA_CLOAKED},
        UI::WindowsAndMessaging::{
            GetWindowRect, GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible,
        },
    };
    let hwnd = window.hwnd().map_err(|_| "GRID_DWM_HANDLE_UNAVAILABLE")?.0 as HWND;
    if hwnd.is_null() {
        return Err("GRID_DWM_HANDLE_INVALID".into());
    }
    // A process-owned HWND is required before querying DWM. The barrier is
    // intentionally on the child IPC thread, never the GTK/Win32 event loop.
    unsafe {
        let mut pid = 0u32;
        if GetWindowThreadProcessId(hwnd, &mut pid) == 0 || pid != std::process::id() {
            return Err("GRID_DWM_HANDLE_OWNER_MISMATCH".into());
        }
    }
    let sample = || unsafe {
        let mut pid = 0u32;
        if IsWindow(hwnd) == 0
            || GetWindowThreadProcessId(hwnd, &mut pid) == 0
            || pid != std::process::id()
        {
            return Err("GRID_DWM_HANDLE_OWNER_MISMATCH".into());
        }
        let visible = IsWindowVisible(hwnd) != 0;
        // A minimized/cloaked HWND cannot attest to a requested show. For a
        // requested hide require the actual WS_VISIBLE state to be cleared,
        // rather than accepting a still-shown but incidentally cloaked window.
        if visible && expected.is_some() {
            let mut cloaked = 0u32;
            let code = DwmGetWindowAttribute(
                hwnd,
                DWMWA_CLOAKED as u32,
                (&mut cloaked as *mut u32).cast(),
                std::mem::size_of::<u32>() as u32,
            );
            if code < 0 {
                return Err("GRID_DWM_CLOAK_UNCONFIRMED".into());
            }
            if cloaked != 0 || IsIconic(hwnd) != 0 {
                return Ok(NativeWindowReceipt {
                    visible: false,
                    source: NativeWindowSource::WindowsDwm,
                    rect: None,
                    compositor_barrier: true,
                });
            }
        }
        let rect = if visible {
            let mut bounds: RECT = std::mem::zeroed();
            if GetWindowRect(hwnd, &mut bounds) == 0 {
                return Err("GRID_DWM_GEOMETRY_UNAVAILABLE".into());
            }
            Some(IpcRect {
                x: f64::from(bounds.left),
                y: f64::from(bounds.top),
                w: f64::from(bounds.right) - f64::from(bounds.left),
                h: f64::from(bounds.bottom) - f64::from(bounds.top),
            })
        } else {
            None
        };
        Ok(NativeWindowReceipt {
            visible,
            source: NativeWindowSource::WindowsDwm,
            rect,
            compositor_barrier: true,
        })
    };
    // First observe the actual window state and bounds. Then wait for the
    // compositor barrier and re-observe the SAME HWND. A change during
    // the barrier cannot be returned as the requested geometry.
    observe_until(expected, &sample)?;
    if unsafe { DwmFlush() } < 0 {
        return Err("GRID_DWM_BARRIER_UNCONFIRMED".into());
    }
    let receipt = sample()?;
    receipt.validate_for(expected)?;
    Ok(receipt)
}
