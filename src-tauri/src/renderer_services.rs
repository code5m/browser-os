//! Ordinary-tab-only native IPC services, reached after parent identity,
//! route and sequence checks. No remote command exposes these services.
use crate::grid_ipc::{GridCmd, MAX_RENDERER_CAPTURE_BYTES, RENDERER_CAPTURE_CHUNK_BYTES};
use base64::Engine as _;
use tauri::{AppHandle, Manager};
use tauri_plugin_browser_tabs::{CaptureCancellation, CaptureDocument, TabManagerState};

struct Captured {
    ticket: String,
    document: CaptureDocument,
    bytes: Vec<u8>,
}
#[derive(Default)]
pub(crate) struct CaptureBuffer(Option<Captured>);

fn valid_ticket(ticket: &str) -> bool {
    ticket.len() == 36 && uuid::Uuid::parse_str(ticket).is_ok_and(|id| id.get_version_num() == 4)
}

fn capture_start(app: &AppHandle, buffer: &mut CaptureBuffer, id: &String, ticket: &str)
    -> Result<Option<String>, String>
{
    if !valid_ticket(ticket) { return Err("CAPTURE_INVALID_TICKET".into()); }
    // The main capture owner serializes start requests. A lost release reply
    // must not permanently strand a completed buffer on the same document.
    // The synchronous capture reader still cannot enter a second Start while
    // the first native capture is running.
    buffer.0 = None;
    let manager = app.state::<TabManagerState>();
    let document = manager.capture_document(id).map_err(|_| "CAPTURE_DOCUMENT_STALE")?;
    let cancellation = CaptureCancellation::new();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    let dispatched = manager.capture_bound_document(id, document.clone(), cancellation.clone(), move |result| {
        let _ = send.send(result);
    });
    if dispatched.is_err() && cancellation.revoke_pending_native() {
        return Err("CAPTURE_NATIVE_DISPATCH_REJECTED".into());
    }
    // Do not release a capture merely because its requester timed out.
    let result = receive.recv().map_err(|_| "CAPTURE_RENDERER_OUTCOME_UNKNOWN")??;
    if !manager.is_current_capture_document(id, &document) {
        return Err("CAPTURE_DOCUMENT_STALE".into());
    }
    if result.is_empty() || result.len() > MAX_RENDERER_CAPTURE_BYTES {
        return Err("CAPTURE_IMAGE_LIMIT".into());
    }
    crate::isolated_capture::checked_png(&result)?;
    let length = result.len();
    buffer.0 = Some(Captured { ticket: ticket.to_owned(), document, bytes: result });
    Ok(Some(length.to_string()))
}

fn capture_read(app: &AppHandle, buffer: &CaptureBuffer, id: &String, ticket: &str, offset: usize)
    -> Result<Option<String>, String>
{
    let capture = buffer.0.as_ref().filter(|value| value.ticket == ticket)
        .ok_or("CAPTURE_STALE_TICKET")?;
    if !app.state::<TabManagerState>().is_current_capture_document(id, &capture.document) {
        return Err("CAPTURE_DOCUMENT_STALE".into());
    }
    if offset >= capture.bytes.len() || offset % RENDERER_CAPTURE_CHUNK_BYTES != 0 {
        return Err("CAPTURE_CHUNK_INVALID".into());
    }
    let end = offset.saturating_add(RENDERER_CAPTURE_CHUNK_BYTES).min(capture.bytes.len());
    Ok(Some(base64::engine::general_purpose::STANDARD.encode(&capture.bytes[offset..end])))
}

fn fill(app: &AppHandle, id: &String, origin: &str, username: &str, password: &str)
    -> Result<Option<String>, String>
{
    if username.len() > 8192 || password.len() > 8192 || origin.len() > 8192
        || crate::bridge::credential_origin(origin).as_deref() != Some(origin)
    {
        return Err("CREDENTIAL_FILL_INVALID".into());
    }
    let manager = app.state::<TabManagerState>();
    let document = manager.capture_document(id).map_err(|_| "TAB_RENDERER_DOCUMENT_STALE")?;
    let js = crate::bridge::build_autofill_script(origin, username, password);
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    manager.eval_result_document(id, &document, js, move |value| { let _ = send.send(value); })
        .map_err(|_| "CREDENTIAL_FILL_OUTCOME_UNKNOWN")?;
    let code = receive.recv()
        .map_err(|_| "CREDENTIAL_FILL_OUTCOME_UNKNOWN")?
        .map_err(|_| "CREDENTIAL_FILL_OUTCOME_UNKNOWN")?;
    if !manager.is_current_capture_document(id, &document) {
        return Err("CREDENTIAL_FILL_OUTCOME_UNKNOWN".into());
    }
    if !crate::bridge::AUTOFILL_CODES.contains(&code.as_str()) {
        return Err("CREDENTIAL_FILL_FAILED".into());
    }
    Ok(Some(code))
}

pub(crate) fn dispatch(app: &AppHandle, buffer: &mut CaptureBuffer, cmd: &GridCmd)
    -> Option<Result<Option<String>, String>>
{
    Some(match cmd {
        GridCmd::FillCredential { id, origin, username, password } => fill(app, id, origin, username, password),
        GridCmd::CaptureStart { id, ticket } => capture_start(app, buffer, id, ticket),
        GridCmd::CaptureRead { id, ticket, offset } => capture_read(app, buffer, id, ticket, *offset),
        GridCmd::CaptureRelease { id, ticket } => {
            if buffer.0.as_ref().is_some_and(|value| value.ticket == *ticket) {
                let capture = buffer.0.take().expect("same owned capture");
                if app.state::<TabManagerState>().is_current_capture_document(id, &capture.document) {
                    Ok(None)
                } else {
                    Err("CAPTURE_DOCUMENT_STALE".into())
                }
            } else { Err("CAPTURE_STALE_TICKET".into()) }
        }
        _ => return None,
    })
}
