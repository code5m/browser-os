//! The executable used in the lowbox is shipped with BrowserOS. Its manifest is
//! compiled into the host, and both on-disk inputs stay pinned through staging.
use super::{check_operation, OperationBudget};
use browseros_platform_security::windows::PinnedReadFile;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

const SOURCE_CONTRACT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../vendor/windows-rust-analyzer/source.json"
));
const EMBEDDED_MANIFEST: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/windows-lsp/runtime.json"
));
const MAX_MANIFEST_BYTES: u64 = 32 * 1024;
const MAX_EXECUTABLE_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    source: Value,
    artifact: Artifact,
    build: Build,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    file: String,
    sha256: String,
    size: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Build {
    rustc_release: String,
    rustc_commit: String,
    rustc_sha256: String,
    target: String,
}

pub(super) struct VerifiedBundle {
    analyzer: PinnedReadFile,
    // Keep the source of the comparison pinned until the executable is staged.
    _manifest: PinnedReadFile,
}

impl VerifiedBundle {
    pub(super) fn path(&self) -> &Path {
        self.analyzer.path()
    }
}

fn digest_is_valid(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn decode_manifest(bytes: &[u8]) -> Result<Manifest, String> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_MANIFEST_BYTES {
        return Err("LSP_BUNDLE_MANIFEST_INVALID".into());
    }
    let manifest: Manifest =
        serde_json::from_slice(bytes).map_err(|_| "LSP_BUNDLE_MANIFEST_INVALID")?;
    let source: Value =
        serde_json::from_str(SOURCE_CONTRACT).map_err(|_| "LSP_BUNDLE_MANIFEST_INVALID")?;
    if manifest.schema_version != 1
        || manifest.source != source
        || manifest.artifact.file != "rust-analyzer.exe"
        || manifest.artifact.size == 0
        || manifest.artifact.size > MAX_EXECUTABLE_BYTES
        || !digest_is_valid(&manifest.artifact.sha256)
        || !digest_is_valid(&manifest.build.rustc_sha256)
        || source["rust_release"].as_str() != Some(manifest.build.rustc_release.as_str())
        || source["source_commit"].as_str() != Some(manifest.build.rustc_commit.as_str())
        || source["target"].as_str() != Some(manifest.build.target.as_str())
    {
        return Err("LSP_BUNDLE_MANIFEST_INVALID".into());
    }
    Ok(manifest)
}

fn bundle_directory() -> Result<PathBuf, String> {
    #[cfg(any(test, debug_assertions))]
    {
        Ok(Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/windows-lsp"))
    }
    #[cfg(not(any(test, debug_assertions)))]
    {
        let executable = std::env::current_exe().map_err(|_| "LSP_BUNDLE_DIRECTORY_UNAVAILABLE")?;
        Ok(executable
            .parent()
            .ok_or("LSP_BUNDLE_DIRECTORY_UNAVAILABLE")?
            .join("windows-lsp"))
    }
}

pub(super) fn open(operation: &OperationBudget) -> Result<VerifiedBundle, String> {
    check_operation(operation)?;
    open_directory(&bundle_directory()?, EMBEDDED_MANIFEST, operation)
}

// Only open() supplies the compiled manifest in production. Tests exercise the
// same native file/pin/hash path with small non-executable synthetic bytes.
fn open_directory(
    directory: &Path,
    expected_manifest: &[u8],
    operation: &OperationBudget,
) -> Result<VerifiedBundle, String> {
    check_operation(operation)?;
    let expected = decode_manifest(expected_manifest)?;
    let mut manifest = PinnedReadFile::open(&directory.join("runtime.json"), operation)
        .map_err(|_| "LSP_BUNDLE_MANIFEST_UNAVAILABLE")?;
    let mut observed = Vec::new();
    Read::take(&mut manifest, MAX_MANIFEST_BYTES + 1)
        .read_to_end(&mut observed)
        .map_err(|_| "LSP_BUNDLE_MANIFEST_UNAVAILABLE")?;
    check_operation(operation)?;
    if observed.as_slice() != expected_manifest {
        return Err("LSP_BUNDLE_MANIFEST_MISMATCH".into());
    }
    let mut analyzer = PinnedReadFile::open(&directory.join(&expected.artifact.file), operation)
        .map_err(|_| "LSP_BUNDLE_EXECUTABLE_UNAVAILABLE")?;
    if analyzer
        .file()
        .metadata()
        .map_err(|_| "LSP_BUNDLE_EXECUTABLE_UNAVAILABLE")?
        .len()
        != expected.artifact.size
    {
        return Err("LSP_BUNDLE_EXECUTABLE_MISMATCH".into());
    }
    let mut hash = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        check_operation(operation)?;
        let count = analyzer
            .read(&mut buffer)
            .map_err(|_| "LSP_BUNDLE_EXECUTABLE_UNAVAILABLE")?;
        if count == 0 {
            break;
        }
        total = total
            .checked_add(count as u64)
            .ok_or("LSP_BUNDLE_EXECUTABLE_MISMATCH")?;
        if total > expected.artifact.size {
            return Err("LSP_BUNDLE_EXECUTABLE_MISMATCH".into());
        }
        hash.update(&buffer[..count]);
    }
    check_operation(operation)?;
    if total != expected.artifact.size
        || format!("{:x}", hash.finalize()) != expected.artifact.sha256
    {
        return Err("LSP_BUNDLE_EXECUTABLE_MISMATCH".into());
    }
    analyzer
        .rewind()
        .map_err(|_| "LSP_BUNDLE_EXECUTABLE_UNAVAILABLE")?;
    Ok(VerifiedBundle {
        analyzer,
        _manifest: manifest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs::{self, OpenOptions};
    use std::time::Duration;

    struct Fixture {
        directory: PathBuf,
        manifest: Vec<u8>,
    }

    impl Fixture {
        fn new() -> Self {
            let directory =
                std::env::temp_dir().join(format!("browseros-lsp-bundle-{}", uuid::Uuid::new_v4()));
            fs::create_dir(&directory).unwrap();
            let bytes = b"non-executable bundled runtime fixture";
            let source: Value = serde_json::from_str(SOURCE_CONTRACT).unwrap();
            let manifest = serde_json::to_vec(&json!({
                "schema_version": 1,
                "source": source,
                "artifact": {
                    "file": "rust-analyzer.exe",
                    "sha256": format!("{:x}", Sha256::digest(bytes)),
                    "size": bytes.len()
                },
                "build": {
                    "rustc_release": "1.100.0",
                    "rustc_commit": "0ef1bc386059596dada8fb65b50425aab2ea637a",
                    "rustc_sha256": "a".repeat(64),
                    "target": "x86_64-pc-windows-msvc"
                }
            }))
            .unwrap();
            fs::write(directory.join("rust-analyzer.exe"), bytes).unwrap();
            fs::write(directory.join("runtime.json"), &manifest).unwrap();
            Self {
                directory,
                manifest,
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.directory).expect("remove only this synthetic bundle");
        }
    }

    fn operation() -> OperationBudget {
        OperationBudget::new(Duration::from_secs(30))
    }

    #[test]
    fn verified_bundle_pins_manifest_and_executable_until_staging() {
        let fixture = Fixture::new();
        let selected = open_directory(&fixture.directory, &fixture.manifest, &operation()).unwrap();
        let executable = fixture.directory.join("rust-analyzer.exe");
        let manifest = fixture.directory.join("runtime.json");
        assert_eq!(selected.path(), executable.canonicalize().unwrap());
        assert!(OpenOptions::new().write(true).open(&executable).is_err());
        assert!(fs::rename(&executable, fixture.directory.join("changed.exe")).is_err());
        assert!(fs::write(&manifest, b"altered").is_err());
        drop(selected);
        fs::rename(&executable, fixture.directory.join("changed.exe")).unwrap();
        fs::write(&manifest, b"released").unwrap();
    }

    #[test]
    fn altered_bundle_source_manifest_binary_and_size_are_rejected() {
        let fixture = Fixture::new();
        let manifest_path = fixture.directory.join("runtime.json");
        fs::write(&manifest_path, b"{}").unwrap();
        assert_eq!(
            open_directory(&fixture.directory, &fixture.manifest, &operation())
                .err()
                .unwrap(),
            "LSP_BUNDLE_MANIFEST_MISMATCH"
        );
        for field in ["source", "size", "file", "hash"] {
            let mut changed: Value = serde_json::from_slice(&fixture.manifest).unwrap();
            let expected_error = match field {
                "source" => {
                    changed["source"]["source_commit"] = json!("unreviewed");
                    "LSP_BUNDLE_MANIFEST_INVALID"
                }
                "size" => {
                    changed["artifact"]["size"] =
                        json!(changed["artifact"]["size"].as_u64().unwrap() + 1);
                    "LSP_BUNDLE_EXECUTABLE_MISMATCH"
                }
                "file" => {
                    changed["artifact"]["file"] = json!("../another.exe");
                    "LSP_BUNDLE_MANIFEST_INVALID"
                }
                "hash" => {
                    changed["artifact"]["sha256"] = json!("b".repeat(64));
                    "LSP_BUNDLE_EXECUTABLE_MISMATCH"
                }
                _ => unreachable!(),
            };
            let changed = serde_json::to_vec(&changed).unwrap();
            fs::write(&manifest_path, &changed).unwrap();
            assert_eq!(
                open_directory(&fixture.directory, &changed, &operation())
                    .err()
                    .unwrap(),
                expected_error,
                "{field}"
            );
        }
        fs::write(&manifest_path, &fixture.manifest).unwrap();
        let executable = fixture.directory.join("rust-analyzer.exe");
        let mut changed = fs::read(&executable).unwrap();
        changed[0] ^= 1;
        fs::write(&executable, changed).unwrap();
        assert_eq!(
            open_directory(&fixture.directory, &fixture.manifest, &operation())
                .err()
                .unwrap(),
            "LSP_BUNDLE_EXECUTABLE_MISMATCH"
        );
    }

    #[test]
    fn missing_bundle_and_cancelled_preparation_fail_closed() {
        let fixture = Fixture::new();
        fs::remove_file(fixture.directory.join("rust-analyzer.exe")).unwrap();
        assert_eq!(
            open_directory(&fixture.directory, &fixture.manifest, &operation())
                .err()
                .unwrap(),
            "LSP_BUNDLE_EXECUTABLE_UNAVAILABLE"
        );
        let cancelled = operation();
        cancelled.cancel();
        assert_eq!(
            open_directory(&fixture.directory, &fixture.manifest, &cancelled)
                .err()
                .unwrap(),
            "LSP_OPERATION_CANCELLED"
        );
    }
}
