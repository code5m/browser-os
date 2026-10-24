fn main() {
    // Vite uses hashed asset names. Track the directory recursively so a new
    // frontend build cannot silently reuse a binary with stale embedded assets.
    println!("cargo:rerun-if-changed=../dist");
    tauri_build::build();
}
