fn prepare_windows_lsp_runtime() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    for input in [
        "../scripts/build-windows-lsp-runtime.ps1",
        "../vendor/windows-rust-analyzer/source.json",
        "../vendor/windows-rust-analyzer/metadata-stdin.patch",
        "../vendor/windows-rust-analyzer/Cargo.lock",
        "resources/windows-lsp/runtime.json",
        "resources/windows-lsp/rust-analyzer.exe",
        "resources/windows-lsp/Cargo.lock",
        "resources/windows-lsp/LICENSE-MIT",
        "resources/windows-lsp/LICENSE-APACHE",
    ] {
        println!("cargo:rerun-if-changed={input}");
    }
    let manifest = std::path::PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR").expect("Cargo provides the manifest directory"),
    );
    let repository = manifest.parent().expect("host crate belongs to BrowserOS");
    // Cargo OUT_DIR is <profile>/build/<package>/out. Validate that layout
    // before publishing the same verified bundle for cargo run --release.
    let output = std::path::PathBuf::from(
        std::env::var_os("OUT_DIR").expect("Cargo provides the build output directory"),
    );
    let build = output.parent().and_then(std::path::Path::parent);
    assert!(
        output.is_absolute()
            && output.file_name() == Some(std::ffi::OsStr::new("out"))
            && build.and_then(std::path::Path::file_name) == Some(std::ffi::OsStr::new("build")),
        "Unsupported Cargo output layout for Windows LSP resources"
    );
    let executable_directory = build
        .and_then(std::path::Path::parent)
        .expect("Cargo profile output directory");
    let script = repository.join("scripts/build-windows-lsp-runtime.ps1");
    let status = std::process::Command::new("pwsh")
        .args(["-NoProfile", "-File"])
        .arg(&script)
        .arg("-ExecutableDirectory")
        .arg(executable_directory)
        .current_dir(repository)
        .status()
        .expect("Windows LSP preparation requires PowerShell 7, Git and rustup");
    assert!(
        status.success(),
        "Windows LSP runtime preparation failed; run scripts/build-windows-lsp-runtime.ps1"
    );
}

fn main() {
    // Vite uses hashed asset names. Track the directory recursively so a new
    // frontend build cannot silently reuse a binary with stale embedded assets.
    println!("cargo:rerun-if-changed=../dist");
    // Dev, bare Cargo, CI and NSIS use the same idempotent preparation. The
    // nested Cargo build has an independent target directory and toolchain.
    prepare_windows_lsp_runtime();
    tauri_build::build();
}
