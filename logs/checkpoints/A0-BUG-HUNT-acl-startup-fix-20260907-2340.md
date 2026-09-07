# A0 BUG-HUNT ACL and Startup Fix

## Trigger

Native-client screenshots showed `list_bookmarks`, `term_spawn_channel`, and `list_tools` rejected from the local `main` webview. The same client also exposed the documented `open_tool` ACL gap.

## Delivered

- `run-gui.sh` now builds the native client by default. `--no-build` is explicitly opt-in and fails when no binary exists. This keeps the Vite frontend and Tauri's build-time embedded capability manifest in sync.
- `open_tool` is now allowed by the main-window `default-commands` permission.
- Command-set consistency no longer allow-lists `open_tool`; tool policy now detects a missing `open_tool` ACL entry; startup smoke checks the fresh-build invariant.

## Verification

- `bash scripts/check-dev-startup.sh`: PASS (24/24)
- `cargo test --manifest-path src-tauri/Cargo.toml tools::tests`: PASS (4/4)
- `python3 scripts/check-command-set-consistency.py`: PASS (main ACL 135/135 registered commands)
- `python3 scripts/check-tools-policy.py --self-test`: PASS (17 bad samples detected)
- `npm run build`: PASS
- `bash scripts/pre-merge.sh`: `PRE_MERGE_RESULT=ALL_PASS`

## User Retest

Close any already-running client, then start `bash run-gui.sh` without `--no-build`. Verify bookmarks, terminal, and toolbox again. The old process keeps its previous in-memory manifest and cannot be repaired without restart.
