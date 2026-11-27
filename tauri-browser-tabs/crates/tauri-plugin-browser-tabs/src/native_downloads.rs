//! UI-thread-owned engine transfer objects. No URL or full destination path
//! crosses this native-only adapter. Counts remain an independent fallback.
use serde_json::{json, Value};
#[cfg(target_os = "linux")]
use std::{cell::Cell, rc::Rc};
use std::{cell::RefCell, collections::HashMap};
#[cfg(target_os = "linux")]
use webkit2gtk::{DownloadExt, WebContextExt, WebViewExt};
#[cfg(target_os = "windows")]
use webview2_com::Microsoft::Web::WebView2::Win32::*;

const LIMIT: usize = 64;
#[derive(Clone)]
struct NativeTransfer {
    #[cfg(target_os = "linux")]
    download: webkit2gtk::Download,
    #[cfg(target_os = "linux")]
    state: Rc<Cell<&'static str>>,
    #[cfg(target_os = "windows")]
    download: ICoreWebView2DownloadOperation,
}
#[derive(Clone)]
struct Entry {
    id: String,
    native: NativeTransfer,
}
#[cfg(target_os = "linux")]
type Registration = (webkit2gtk::WebContext, gtk::glib::SignalHandlerId);
#[cfg(target_os = "windows")]
type Registration = (ICoreWebView2_4, i64);
struct Registry {
    registration: Option<Registration>,
    next: u64,
    entries: Vec<Entry>,
    limited: bool,
}
thread_local! {
    static TRANSFERS: RefCell<HashMap<u64, Registry>> = RefCell::new(HashMap::new());
}

fn basename(value: &str) -> Option<String> {
    let result: String = value
        .rsplit(['/', '\\'])
        .next()?
        .chars()
        .filter(|c| {
            !c.is_control() && !matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
        .take(180)
        .collect();
    (!result.is_empty() && result != "." && result != "..").then_some(result)
}
fn exact_count(value: u64) -> Option<String> {
    Some(value.to_string())
}

impl NativeTransfer {
    #[cfg(target_os = "linux")]
    fn observe(&self, id: &str) -> Value {
        let state = self.state.get();
        let name = self.download.destination().and_then(|uri| {
            tauri::Url::parse(uri.as_str())
                .ok()?
                .to_file_path()
                .ok()
                .and_then(|path| basename(&path.file_name()?.to_string_lossy()))
        });
        let progress = self.download.estimated_progress();
        json!({"transfer_id":id,"name":name,"state":state,
            "received_bytes":exact_count(self.download.received_data_length()),"total_bytes":null,
            "progress":if progress.is_finite() && (0.0..=1.0).contains(&progress) {Some(progress)} else {None},
            "controls":if state == "in_progress" {vec!["cancel"]} else {vec![]}})
    }

    #[cfg(target_os = "windows")]
    fn observe(&self, id: &str) -> Value {
        use windows::core::PWSTR;
        let mut state = COREWEBVIEW2_DOWNLOAD_STATE_IN_PROGRESS;
        let mut reason = COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON::default();
        let mut can_resume = Default::default();
        let mut received = -1i64;
        let mut total = -1i64;
        let mut path = PWSTR::null();
        let (state, can_resume, received, total, name) = unsafe {
            let state = if self.download.State(&mut state).is_err() {
                "unknown"
            } else if state == COREWEBVIEW2_DOWNLOAD_STATE_COMPLETED {
                "completed"
            } else if state == COREWEBVIEW2_DOWNLOAD_STATE_IN_PROGRESS {
                "in_progress"
            } else if state == COREWEBVIEW2_DOWNLOAD_STATE_INTERRUPTED {
                if self.download.InterruptReason(&mut reason).is_err() {
                    "interrupted"
                } else if reason == COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_USER_PAUSED {
                    "paused"
                } else if reason == COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_USER_CANCELED {
                    "cancelled"
                } else {
                    "interrupted"
                }
            } else {
                "unknown"
            };
            let can_resume = self.download.CanResume(&mut can_resume).is_ok() && can_resume.0 != 0;
            let received = self
                .download
                .BytesReceived(&mut received)
                .ok()
                .and_then(|_| u64::try_from(received).ok());
            let total = self
                .download
                .TotalBytesToReceive(&mut total)
                .ok()
                .and_then(|_| u64::try_from(total).ok());
            let path_read = self.download.ResultFilePath(&mut path).is_ok();
            let path = webview2_com::take_pwstr(path);
            (
                state,
                can_resume,
                received,
                total,
                if path_read { basename(&path) } else { None },
            )
        };
        let controls = match state {
            "in_progress" => vec!["pause", "cancel"],
            "paused" | "interrupted" if can_resume => vec!["resume", "cancel"],
            "paused" => vec!["cancel"],
            _ => vec![],
        };
        let progress = received
            .zip(total)
            .and_then(|(r, t)| (t > 0 && r <= t).then_some(r as f64 / t as f64));
        json!({"transfer_id":id,"name":name,"state":state,
            "received_bytes":received.map(|v|v.to_string()),"total_bytes":total.map(|v|v.to_string()),
            "progress":progress,"controls":controls})
    }
    fn control(&self, id: &str, operation: &str) -> Result<(), String> {
        // Reobserve the exact native object's current state at admission.
        if !self.observe(id)["controls"]
            .as_array()
            .is_some_and(|items| items.iter().any(|v| v.as_str() == Some(operation)))
        {
            return Err("DOWNLOAD_CONTROL_UNAVAILABLE_OR_STALE".into());
        }
        #[cfg(target_os = "linux")]
        {
            if operation != "cancel" {
                return Err("DOWNLOAD_CONTROL_UNSUPPORTED".into());
            }
            self.download.cancel();
            Ok(())
        }
        #[cfg(target_os = "windows")]
        unsafe {
            match operation {
                "pause" => self.download.Pause(),
                "resume" => self.download.Resume(),
                "cancel" => self.download.Cancel(),
                _ => return Err("DOWNLOAD_CONTROL_UNSUPPORTED".into()),
            }
            .map_err(|_| "DOWNLOAD_CONTROL_OUTCOME_UNKNOWN".into())
        }
    }
}
fn existing(instance: u64) -> Vec<Entry> {
    TRANSFERS.with(|all| {
        all.borrow()
            .get(&instance)
            .map(|r| r.entries.clone())
            .unwrap_or_default()
    })
}
fn track(instance: u64, native: NativeTransfer) {
    // Engine calls may reenter callbacks. Never hold the registry borrow
    // across such a call. Evict only a confirmed terminal observation.
    let entries = existing(instance);
    if entries
        .iter()
        .any(|entry| entry.native.download == native.download)
    {
        return;
    }
    let terminal = if entries.len() >= LIMIT {
        entries
            .iter()
            .find(|entry| {
                matches!(
                    entry.native.observe(&entry.id)["state"].as_str(),
                    Some("completed" | "failed" | "cancelled")
                )
            })
            .map(|entry| entry.id.clone())
    } else {
        None
    };
    TRANSFERS.with(|all| {
        let mut all = all.borrow_mut();
        let Some(registry) = all.get_mut(&instance) else {
            return;
        };
        if registry.entries.len() >= LIMIT {
            registry.limited = true;
            if let Some(id) = terminal {
                registry.entries.retain(|entry| entry.id != id);
            }
        }
        if registry.entries.len() >= LIMIT {
            return;
        }
        let Some(next) = registry.next.checked_add(1) else {
            registry.limited = true;
            return;
        };
        let id = format!("{instance}:{}", registry.next);
        registry.next = next;
        registry.entries.push(Entry { id, native });
    });
}
fn create(instance: u64) {
    TRANSFERS.with(|all| {
        all.borrow_mut().insert(
            instance,
            Registry {
                registration: None,
                next: 1,
                entries: Vec::new(),
                limited: false,
            },
        );
    });
}
fn registered(instance: u64, registration: Registration) {
    TRANSFERS.with(|all| {
        if let Some(registry) = all.borrow_mut().get_mut(&instance) {
            registry.registration = Some(registration);
        }
    });
}

#[cfg(target_os = "linux")]
pub(super) fn attach_linux(instance: u64, webview: &webkit2gtk::WebView) {
    use gtk::prelude::ObjectExt;
    let Some(context) = webview.context() else {
        return;
    };
    create(instance);
    let weak = webview.downgrade();
    let token = context.connect_download_started(move |_, download| {
        let Some(view) = weak.upgrade() else {
            return;
        };
        if download.web_view().as_ref() != Some(&view) {
            return;
        }
        let state = Rc::new(Cell::new("in_progress"));
        let failed = Rc::clone(&state);
        download.connect_failed(move |_, error| {
            failed.set(
                if error.matches(webkit2gtk::DownloadError::CancelledByUser) {
                    "cancelled"
                } else {
                    "failed"
                },
            );
        });
        let finished = Rc::clone(&state);
        download.connect_finished(move |_| {
            if finished.get() == "in_progress" {
                finished.set("completed");
            }
        });
        track(
            instance,
            NativeTransfer {
                download: download.clone(),
                state,
            },
        );
    });
    registered(instance, (context, token));
}

#[cfg(target_os = "windows")]
pub(super) fn attach_windows(instance: u64, core: ICoreWebView2) {
    use windows::core::Interface;
    let Ok(core) = core.cast::<ICoreWebView2_4>() else {
        return;
    };
    create(instance);
    let handler = webview2_com::DownloadStartingEventHandler::create(Box::new(move |_, args| {
        if let Some(args) = args {
            if let Ok(download) = unsafe { args.DownloadOperation() } {
                track(instance, NativeTransfer { download });
            }
        }
        Ok(())
    }));
    let mut token = 0i64;
    if unsafe { core.add_DownloadStarting(&handler, &mut token) }.is_ok() {
        registered(instance, (core, token));
    } else {
        detach(instance);
    }
}

pub(super) fn detach(instance: u64) {
    let registry = TRANSFERS.with(|all| all.borrow_mut().remove(&instance));
    if let Some((native, token)) = registry.and_then(|registry| registry.registration) {
        #[cfg(target_os = "linux")]
        {
            use gtk::prelude::ObjectExt;
            native.disconnect(token);
        }
        #[cfg(target_os = "windows")]
        {
            let _ = unsafe { native.remove_DownloadStarting(token) };
        }
    }
}

pub(super) fn request(instance: u64, control: Option<(&str, &str)>) -> Result<Value, String> {
    let (registered, limited, entries) = TRANSFERS
        .with(|all| {
            all.borrow().get(&instance).map(|registry| {
                (
                    registry.registration.is_some(),
                    registry.limited,
                    registry.entries.clone(),
                )
            })
        })
        .ok_or("DOWNLOAD_NATIVE_TRACKING_UNAVAILABLE")?;
    if !registered {
        return Err("DOWNLOAD_NATIVE_TRACKING_UNAVAILABLE".into());
    }
    let mut dispatched = false;
    if let Some((id, operation)) = control {
        let entry = entries
            .iter()
            .find(|entry| entry.id == id)
            .ok_or("DOWNLOAD_TRANSFER_STALE")?;
        entry.native.control(id, operation)?;
        dispatched = true;
    }
    let rows = existing(instance)
        .iter()
        .map(|entry| entry.native.observe(&entry.id))
        .collect::<Vec<_>>();
    Ok(
        json!({"native_instance":instance.to_string(),"coverage":"since_registration",
        "platform":std::env::consts::OS,"limited":limited,"limit":LIMIT,
        "control_dispatched":dispatched,"items":rows}),
    )
}
