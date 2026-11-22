//! Receipt synchronization for the opt-in installed GUI regression driver.
//! Grid eval acknowledges dispatch, not the page's asynchronous HTTP request.
//! Only the local mock server writes this per-run JSONL evidence file.

use std::io::Read;
use std::path::Path;
use std::time::{Duration, Instant};

const MAX_RECEIPT_BYTES: usize = 256 * 1024;
const RECEIPT_TIMEOUT: Duration = Duration::from_secs(8);
const RECEIPT_POLL: Duration = Duration::from_millis(25);

pub(crate) struct ReceiptProbe {
    case_id: String,
    grid: String,
    phase: String,
    nonce: String,
    require_menu: bool,
}

impl ReceiptProbe {
    /// Dispatch exactly once. Keep sending and waiting separate so the four-grid
    /// scenario still dispatches to every child before waiting for any receipt.
    pub(crate) fn send(
        case_id: &str,
        grid: usize,
        phase: &str,
        require_menu: bool,
        dispatch: impl FnOnce(&str) -> Result<(), String>,
    ) -> Result<Self, String> {
        let probe = Self {
            case_id: case_id.to_string(),
            grid: grid.to_string(),
            phase: phase.to_string(),
            nonce: uuid::Uuid::new_v4().to_string(),
            require_menu,
        };
        dispatch(&probe.nonce)?;
        Ok(probe)
    }

    pub(crate) fn wait(self, path: &Path) -> Result<String, String> {
        self.wait_with(
            RECEIPT_TIMEOUT,
            || {
                let file =
                    std::fs::File::open(path).map_err(|_| "GUI_RECEIPT_READ_FAILED".to_string())?;
                let mut bytes = Vec::new();
                file.take((MAX_RECEIPT_BYTES + 1) as u64)
                    .read_to_end(&mut bytes)
                    .map_err(|_| "GUI_RECEIPT_READ_FAILED".to_string())?;
                Ok(bytes)
            },
            std::thread::sleep,
        )
    }

    fn wait_with(
        self,
        timeout: Duration,
        mut snapshot: impl FnMut() -> Result<Vec<u8>, String>,
        mut pause: impl FnMut(Duration),
    ) -> Result<String, String> {
        let started = Instant::now();
        let result: Result<(), String> = (|| loop {
            if self.confirmed(&snapshot()?)? {
                return Ok(());
            }
            let remaining = timeout.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                return Err("GUI_RECEIPT_TIMEOUT".to_string());
            }
            pause(remaining.min(RECEIPT_POLL));
        })();
        let identity = format!(
            "case={} grid={} phase={} nonce={}",
            self.case_id, self.grid, self.phase, self.nonce
        );
        result.map_err(|error| format!("{error}: {identity}"))?;
        let detail = format!(
            "receipt confirmed {identity} elapsed_ms={}",
            started.elapsed().as_millis()
        );
        eprintln!("[m0-6c-gui] {detail}");
        Ok(detail)
    }

    fn confirmed(&self, bytes: &[u8]) -> Result<bool, String> {
        if bytes.len() > MAX_RECEIPT_BYTES {
            return Err("GUI_RECEIPT_LIMIT".to_string());
        }
        // The server may be appending the final record, including part of a UTF-8
        // code point. Only newline-terminated records can acknowledge a probe.
        let Some(end) = bytes.iter().rposition(|&byte| byte == b'\n') else {
            return Ok(false);
        };
        let complete = std::str::from_utf8(&bytes[..=end])
            .map_err(|_| "GUI_RECEIPT_INVALID_UTF8".to_string())?;
        let mut matched = false;
        for line in complete.lines().filter(|line| !line.is_empty()) {
            let event: serde_json::Value =
                serde_json::from_str(line).map_err(|_| "GUI_RECEIPT_INVALID_RECORD".to_string())?;
            if !event.is_object() {
                return Err("GUI_RECEIPT_INVALID_RECORD".to_string());
            }
            if event.get("nonce").and_then(|value| value.as_str()) != Some(self.nonce.as_str()) {
                continue;
            }
            for (key, expected) in [
                ("case", self.case_id.as_str()),
                ("grid", self.grid.as_str()),
                ("phase", self.phase.as_str()),
            ] {
                if event.get(key).and_then(|value| value.as_str()) != Some(expected) {
                    return Err("GUI_RECEIPT_IDENTITY_MISMATCH".to_string());
                }
            }
            let cookie = event
                .get("cookie")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            if !cookie.split(';').any(|part| part.trim() == "m0_session=ok") {
                return Err("GUI_RECEIPT_COOKIE_MISSING".to_string());
            }
            if self.require_menu
                && event.get("menu").and_then(|value| value.as_str()) != Some("true")
            {
                return Err("GUI_RECEIPT_MENU_MISSING".to_string());
            }
            matched = true;
        }
        Ok(matched)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn probe(require_menu: bool) -> ReceiptProbe {
        ReceiptProbe {
            case_id: "7".into(),
            grid: "0".into(),
            phase: "after_crash_recovery".into(),
            nonce: "this-probe".into(),
            require_menu,
        }
    }

    fn event(probe: &ReceiptProbe) -> serde_json::Value {
        serde_json::json!({
            "case": probe.case_id,
            "grid": probe.grid,
            "phase": probe.phase,
            "nonce": probe.nonce,
            "cookie": "other=value; m0_session=ok",
            "menu": "true",
        })
    }

    fn record(event: &serde_json::Value) -> Vec<u8> {
        let mut bytes = serde_json::to_vec(event).unwrap();
        bytes.push(b'\n');
        bytes
    }

    #[test]
    fn delayed_receipt_blocks_progress_without_redispatch() {
        let sends = Cell::new(0);
        let probe = ReceiptProbe::send("7", 0, "after_crash_recovery", false, |_| {
            sends.set(sends.get() + 1);
            Ok(())
        })
        .unwrap();
        let complete = record(&event(&probe));
        let reads = Cell::new(0);
        let pauses = Cell::new(0);
        let detail = probe
            .wait_with(
                Duration::from_secs(1),
                || {
                    reads.set(reads.get() + 1);
                    if reads.get() == 1 {
                        Ok(complete[..complete.len() - 1].to_vec())
                    } else {
                        Ok(complete.clone())
                    }
                },
                |_| {
                    assert_eq!(sends.get(), 1);
                    assert_eq!(reads.get(), 1);
                    pauses.set(pauses.get() + 1);
                },
            )
            .unwrap();
        assert!(detail.contains("receipt confirmed case=7 grid=0 phase=after_crash_recovery"));
        assert_eq!(sends.get(), 1);
        assert_eq!(reads.get(), 2);
        assert_eq!(pauses.get(), 1);
    }

    #[test]
    fn receipt_identity_cookie_and_menu_are_exact() {
        let probe = probe(true);
        let valid = event(&probe);
        assert_eq!(probe.confirmed(&record(&valid)), Ok(true));
        for (key, value, expected) in [
            ("case", "8", "GUI_RECEIPT_IDENTITY_MISMATCH"),
            ("grid", "1", "GUI_RECEIPT_IDENTITY_MISMATCH"),
            ("phase", "loaded", "GUI_RECEIPT_IDENTITY_MISMATCH"),
            ("cookie", "m0_session=okay", "GUI_RECEIPT_COOKIE_MISSING"),
            (
                "cookie",
                "other_m0_session=ok",
                "GUI_RECEIPT_COOKIE_MISSING",
            ),
            ("cookie", "", "GUI_RECEIPT_COOKIE_MISSING"),
            ("menu", "false", "GUI_RECEIPT_MENU_MISSING"),
        ] {
            let mut wrong = valid.clone();
            wrong[key] = serde_json::Value::String(value.into());
            assert_eq!(probe.confirmed(&record(&wrong)), Err(expected.into()));
        }
        let mut stale = valid.clone();
        stale["nonce"] = serde_json::json!("previous-probe");
        assert_eq!(probe.confirmed(&record(&stale)), Ok(false));
        let mut missing_menu = valid;
        missing_menu.as_object_mut().unwrap().remove("menu");
        assert_eq!(
            probe.confirmed(&record(&missing_menu)),
            Err("GUI_RECEIPT_MENU_MISSING".into())
        );
    }

    #[test]
    fn partial_utf8_tail_waits_but_complete_corruption_fails() {
        let probe = probe(false);
        let complete = record(&event(&probe));
        let mut partial = b"{\"text\":\"".to_vec();
        partial.extend_from_slice(&[0xe4, 0xb8]);
        assert_eq!(probe.confirmed(&partial), Ok(false));
        let mut after_receipt = complete;
        after_receipt.extend_from_slice(&partial);
        assert_eq!(probe.confirmed(&after_receipt), Ok(true));
        partial.push(b'\n');
        assert_eq!(
            probe.confirmed(&partial),
            Err("GUI_RECEIPT_INVALID_UTF8".into())
        );
        assert_eq!(
            probe.confirmed(b"{broken}\n"),
            Err("GUI_RECEIPT_INVALID_RECORD".into())
        );
        assert_eq!(
            probe.confirmed(&vec![b'x'; MAX_RECEIPT_BYTES + 1]),
            Err("GUI_RECEIPT_LIMIT".into())
        );
    }

    #[test]
    fn missing_receipt_and_dispatch_failure_cannot_report_success() {
        let probe = probe(false);
        let mut stale = event(&probe);
        stale["nonce"] = serde_json::json!("previous-probe");
        let error = probe
            .wait_with(
                Duration::ZERO,
                || Ok(record(&stale)),
                |_| panic!("must time out"),
            )
            .unwrap_err();
        assert!(error.starts_with("GUI_RECEIPT_TIMEOUT: case=7 grid=0"));
        let sends = Cell::new(0);
        let result = ReceiptProbe::send("7", 0, "after_crash_recovery", false, |_| {
            sends.set(sends.get() + 1);
            Err("GUI_DISPATCH_FAILED".into())
        });
        assert!(matches!(result, Err(error) if error == "GUI_DISPATCH_FAILED"));
        assert_eq!(sends.get(), 1);
    }

    #[test]
    fn four_grid_probes_are_dispatched_before_receipts_are_waited() {
        let sends = Cell::new(0);
        let mut probes = Vec::new();
        for grid in 0..4 {
            probes.push(
                ReceiptProbe::send("2", grid, "concurrent_ai", false, |_| {
                    sends.set(sends.get() + 1);
                    Ok(())
                })
                .unwrap(),
            );
        }
        let nonces = probes
            .iter()
            .map(|probe| probe.nonce.clone())
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(nonces.len(), 4);
        for probe in probes {
            let bytes = record(&event(&probe));
            probe
                .wait_with(
                    Duration::ZERO,
                    || {
                        assert_eq!(sends.get(), 4);
                        Ok(bytes.clone())
                    },
                    |_| panic!("receipt is already present"),
                )
                .unwrap();
        }
    }
}
