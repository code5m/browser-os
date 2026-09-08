//! Debug-only integration smoke, requiring an isolated test data directory.
use tauri::{Listener, Manager};

pub fn start(app: tauri::AppHandle) -> Result<(), String> {
    let data = std::env::var("XDG_DATA_HOME").unwrap_or_default();
    if !data.starts_with("/tmp/workbench-native-") {
        return Err("Smoke requires isolated XDG_DATA_HOME".into());
    }
    let root = crate::workspace::workspace_dir(&app).join("smoke-vault");
    std::fs::create_dir_all(root.join("notes")).map_err(|e| e.to_string())?;
    std::fs::write(root.join("Home.md"), "# Home\n\n[[notes/Plan|Plan]]\n")
        .map_err(|e| e.to_string())?;
    std::fs::write(
        root.join("notes/Plan.md"),
        "# Plan\n\nNative smoke fixture\n\n[[Home]]",
    )
    .map_err(|e| e.to_string())?;
    let conn = rusqlite::Connection::open(root.join("data.db")).map_err(|e| e.to_string())?;
    conn.execute_batch("CREATE TABLE items(value INTEGER); WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x<100) INSERT INTO items SELECT x FROM n;").map_err(|e| e.to_string())?;
    let repo_id = "workbench-smoke";
    crate::workspace::save_repos(
        &app,
        &[crate::domain::RepoConfig {
            id: repo_id.into(),
            provider: crate::domain::RepoProvider::Git,
            name: "Smoke repository".into(),
            remote_url: "https://example.invalid/test.git".into(),
            branch: "master".into(),
            username: String::new(),
        }],
    )?;
    let repo_dir = crate::sync::repo_dir(&app, repo_id)?;
    let repo = git2::Repository::init(&repo_dir).map_err(|e| e.to_string())?;
    std::fs::write(repo_dir.join("README.md"), "fixture\n").map_err(|e| e.to_string())?;
    let mut index = repo.index().map_err(|e| e.to_string())?;
    index
        .add_path(std::path::Path::new("README.md"))
        .map_err(|e| e.to_string())?;
    index.write().map_err(|e| e.to_string())?;
    let oid = index.write_tree().map_err(|e| e.to_string())?;
    let tree = repo.find_tree(oid).map_err(|e| e.to_string())?;
    let sig = git2::Signature::now("Test", "test@example.invalid").map_err(|e| e.to_string())?;
    repo.commit(Some("HEAD"), &sig, &sig, "Smoke initial", &tree, &[])
        .map_err(|e| e.to_string())?;
    let done_app = app.clone();
    app.listen("workbench-smoke-finished", move |event| {
        let passed = serde_json::from_str::<serde_json::Value>(event.payload())
            .ok()
            .and_then(|v| v.get("pass").and_then(|b| b.as_bool()))
            .unwrap_or(false);
        eprintln!("WORKBENCH_NATIVE_REPORT={}", event.payload());
        done_app.exit(if passed { 0 } else { 1 });
    });
    let script = include_str!("../../scripts/workbench-native-smoke.js").replace(
        "__FIXTURE_ROOT__",
        &serde_json::to_string(&root.to_string_lossy()).unwrap(),
    );
    let timeout_app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(2));
        if let Some(view) = app.get_webview_window("main") {
            let _ = view.eval(&script);
        }
    });
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(60));
        eprintln!("WORKBENCH_NATIVE_TIMEOUT");
        timeout_app.exit(2);
    });
    Ok(())
}
