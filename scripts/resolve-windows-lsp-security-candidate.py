#!/usr/bin/env python3
"""Temporary, review-only resolver for the pinned Windows rust-analyzer source.

Runs only in an explicitly dispatched GitHub Actions job. The committed lock,
normal audit jobs, and production checkout are never changed. Delete this script
with the temporary workflow job when the reviewed resolver output is imported.
"""
from __future__ import annotations

import base64
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
import tempfile
import time
import tomllib

REPOSITORY = "https://github.com/rust-lang/rust"
COMMIT = "0ef1bc386059596dada8fb65b50425aab2ea637a"
SUBDIRECTORY = "src/tools/rust-analyzer"
TOOLCHAIN = "1.100.0"
XTASK_MANIFEST = SUBDIRECTORY + "/xtask/Cargo.toml"
XTASK_DIST = SUBDIRECTORY + "/xtask/src/dist.rs"
XTASK_MANIFEST_HASH = "31f58662e9be197079eca81be8abc6dea3ddb58c37f9a525b255596dbf72b6db"
XTASK_DIST_HASH = "e2b16f0ac6f4b2a53f1ff8b678504f112245a53ae17f9e7c6c2b85c274254472"
ZIP_BEFORE = 'zip = { version = "4.0", default-features = false, features = ["deflate-flate2", "time"] }'
ZIP_AFTER = 'zip = { version = "=9.0.5", default-features = false, features = ["deflate-flate2", "time"] }'
TEST_NAME = "dist::browseros_dependency_security_candidate::zip_writer_retains_payload_and_symbols"
ROOT = Path(__file__).resolve().parents[1]
ARTIFACTS = ROOT / "artifacts" / "rust-analyzer-security-candidate"
ASSETS = ROOT / "vendor" / "windows-rust-analyzer"
DEADLINE = time.monotonic() + 26 * 60

ZIP_FUNCTION = """fn zip(src_path: &Path, symbols_path: Option<&PathBuf>, dest_path: &Path) -> anyhow::Result<()> {
    let modified = OffsetDateTime::from(std::fs::metadata(src_path)?.modified()?);
    let modified = DateTime::try_from(time::PrimitiveDateTime::new(
        modified.date(),
        modified.time(),
    ))?;
    let file = File::create(dest_path)?;
    let mut writer = ZipWriter::new(BufWriter::new(file));
    writer.start_file(
        src_path.file_name().unwrap().to_str().unwrap(),
        SimpleFileOptions::default()
            .last_modified_time(modified)
            .unix_permissions(0o755)
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(9)),
    )?;
    let mut input = io::BufReader::new(File::open(src_path)?);
    io::copy(&mut input, &mut writer)?;
    if let Some(symbols_path) = symbols_path {
        writer.start_file(
            symbols_path.file_name().unwrap().to_str().unwrap(),
            SimpleFileOptions::default()
                .last_modified_time(modified)
                .compression_method(zip::CompressionMethod::Deflated)
                .compression_level(Some(9)),
        )?;
        let mut input = io::BufReader::new(File::open(symbols_path)?);
        io::copy(&mut input, &mut writer)?;
    }
    writer.finish()?;
    Ok(())
}
"""

ZIP_TEST = r"""
#[cfg(test)]
mod browseros_dependency_security_candidate {
    use std::io::Read;

    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn zip_writer_retains_payload_and_symbols() -> anyhow::Result<()> {
        let directory = std::path::PathBuf::from(
            std::env::var_os("BROWSEROS_RA_ZIP_CANDIDATE_DIRECTORY")
                .expect("isolated candidate fixture directory"),
        );
        std::fs::create_dir(&directory)?;
        let _cleanup = Cleanup(directory.clone());
        let source = directory.join("rust-analyzer.exe");
        let symbols = directory.join("rust_analyzer.pdb");
        let source_bytes = b"BrowserOS pinned runtime payload\x00\xff";
        let symbols_bytes = b"BrowserOS symbol payload\x00\xfe";
        std::fs::write(&source, source_bytes)?;
        std::fs::write(&symbols, symbols_bytes)?;
        let modified = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
        std::fs::OpenOptions::new().write(true).open(&source)?.set_times(
            std::fs::FileTimes::new().set_modified(modified),
        )?;
        let expected_time = zip::DateTime::from_date_and_time(2023, 11, 14, 22, 13, 20)?;
        for include_symbols in [false, true] {
            let output = directory.join(if include_symbols { "with-symbols.zip" } else { "runtime.zip" });
            super::zip(&source, include_symbols.then_some(&symbols), &output)?;
            let mut archive = zip::ZipArchive::new(std::fs::File::open(&output)?)?;
            archive.validate_header_consistency()?;
            assert_eq!(archive.len(), if include_symbols { 2 } else { 1 });
            let expected = if include_symbols {
                vec![("rust-analyzer.exe", source_bytes.as_slice()), ("rust_analyzer.pdb", symbols_bytes.as_slice())]
            } else {
                vec![("rust-analyzer.exe", source_bytes.as_slice())]
            };
            for (name, bytes) in expected {
                let mut entry = archive.by_name(name)?;
                assert_eq!(entry.compression(), zip::CompressionMethod::Deflated);
                assert_eq!(entry.last_modified(), Some(expected_time));
                if name == "rust-analyzer.exe" {
                    assert_eq!(entry.unix_mode().map(|mode| mode & 0o777), Some(0o755));
                }
                let mut actual = Vec::new();
                entry.read_to_end(&mut actual)?;
                assert_eq!(actual, bytes);
            }
        }
        let invalid_time_source = directory.join("invalid-time.exe");
        std::fs::write(&invalid_time_source, source_bytes)?;
        std::fs::OpenOptions::new().write(true).open(&invalid_time_source)?.set_times(
            std::fs::FileTimes::new().set_modified(std::time::UNIX_EPOCH),
        )?;
        let invalid_time_output = directory.join("invalid-time.zip");
        assert!(super::zip(&invalid_time_source, None, &invalid_time_output).is_err());
        assert!(!invalid_time_output.exists());
        let missing_output = directory.join("missing-source.zip");
        assert!(super::zip(&directory.join("missing.exe"), None, &missing_output).is_err());
        assert!(!missing_output.exists());
        Ok(())
    }
}
"""


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def require_hash(path: Path, expected: str, label: str) -> bytes:
    data = path.read_bytes()
    if sha256(data) != expected:
        raise RuntimeError("RA_SECURITY_HASH_MISMATCH:" + label)
    return data


def run(arguments: list[str], *, cwd: Path = ROOT, env=None, quiet=False) -> str:
    remaining = DEADLINE - time.monotonic()
    if remaining <= 0:
        raise RuntimeError("RA_SECURITY_CANDIDATE_DEADLINE")
    print("RA_SECURITY_COMMAND " + json.dumps(arguments), flush=True)
    with subprocess.Popen(
        arguments, cwd=cwd, env=env, stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT, text=True, encoding="utf-8", errors="replace",
        start_new_session=True,
    ) as process:
        try:
            output, _ = process.communicate(timeout=remaining)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            output, _ = process.communicate()
            with (ARTIFACTS / "commands.log").open("a", encoding="utf-8") as log:
                log.write(json.dumps(arguments) + "\n" + output)
            raise RuntimeError("RA_SECURITY_CANDIDATE_DEADLINE")
        with (ARTIFACTS / "commands.log").open("a", encoding="utf-8") as log:
            log.write(json.dumps(arguments) + "\n" + output)
        if not quiet or process.returncode:
            print(output, end="" if output.endswith("\n") else "\n", flush=True)
        if process.returncode:
            raise subprocess.CalledProcessError(process.returncode, arguments, output=output)
        return output


def transport(path: Path, source_sha: str) -> None:
    data = path.read_bytes()
    git_blob = hashlib.sha1(b"blob " + str(len(data)).encode("ascii") + b"\0" + data).hexdigest()
    actual_git_blob = run(["git", "hash-object", str(path)], quiet=True).strip()
    if actual_git_blob != git_blob:
        raise RuntimeError("RA_SECURITY_GIT_BLOB_HASH_MISMATCH")
    encoded = base64.b64encode(data).decode("ascii")
    chunks = [encoded[index:index + 3000] for index in range(0, len(encoded), 3000)]
    metadata = {
        "source_sha": source_sha, "file": path.name, "bytes": len(data),
        "sha256": sha256(data), "git_blob": git_blob, "chunks": len(chunks),
    }
    print("RA_SECURITY_FILE_META " + json.dumps(metadata, separators=(",", ":")), flush=True)
    for index, chunk in enumerate(chunks):
        print("RA_SECURITY_FILE_CHUNK " + json.dumps({
            "file": path.name, "index": index, "total": len(chunks), "base64": chunk,
        }, separators=(",", ":")), flush=True)


def main() -> None:
    if sys.platform != "linux" or os.environ.get("GITHUB_ACTIONS") != "true":
        raise RuntimeError("RA_SECURITY_CANDIDATE_REQUIRES_EXPLICIT_LINUX_ACTIONS_JOB")
    if os.environ.get("GITHUB_EVENT_NAME") != "workflow_dispatch":
        raise RuntimeError("RA_SECURITY_CANDIDATE_REQUIRES_MANUAL_DISPATCH")
    if os.environ.get("GITHUB_REF") != "refs/heads/fix/windows-owner-integrity-complete-20261122":
        raise RuntimeError("RA_SECURITY_CANDIDATE_BRANCH_MISMATCH")
    ARTIFACTS.mkdir(parents=True, exist_ok=False)
    source_sha = run(["git", "rev-parse", "HEAD"], quiet=True).strip()
    if source_sha != os.environ.get("GITHUB_SHA"):
        raise RuntimeError("RA_SECURITY_CANDIDATE_SOURCE_SHA_MISMATCH")
    (ARTIFACTS / "source-sha.txt").write_text(source_sha + "\n", encoding="ascii")
    status = {"source_sha": source_sha, "result": "FAILED", "phase": "prepare",
              "production_windows_acceptance": "NOT_RUN"}
    candidate_files: list[Path] = []
    try:
        source = json.loads((ASSETS / "source.json").read_bytes())
        if (
            source["source_repository"] != REPOSITORY
            or source["source_commit"] != COMMIT
            or source["source_subdirectory"] != SUBDIRECTORY
            or source["rust_release"] != TOOLCHAIN
            or source["target"] != "x86_64-pc-windows-msvc"
            or source["patch_file"] != "metadata-stdin.patch"
            or source["lock_file"] != "Cargo.lock"
        ):
            raise RuntimeError("RA_SECURITY_SOURCE_CONTRACT_MISMATCH")
        patch = ASSETS / source["patch_file"]
        lock = ASSETS / source["lock_file"]
        require_hash(patch, source["patch_sha256"], "reviewed patch")
        reviewed_lock = require_hash(lock, source["lock_sha256"], "reviewed lock")
        env = os.environ.copy()
        for key in list(env):
            if (
                key in {"RUSTC", "RUSTDOC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER",
                        "RUSTFLAGS", "RUSTDOCFLAGS", "CARGO_ENCODED_RUSTFLAGS",
                        "CARGO_ENCODED_RUSTDOCFLAGS", "CARGO_MAKEFLAGS", "MAKEFLAGS",
                        "CARGO_BUILD_TARGET", "CARGO_BUILD_RUSTC", "CARGO_BUILD_RUSTDOC",
                        "CARGO_BUILD_RUSTC_WRAPPER", "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
                        "CARGO_BUILD_RUSTFLAGS", "CARGO_BUILD_RUSTDOCFLAGS"}
                or key.startswith(("CARGO_TARGET_", "CARGO_PROFILE_"))
            ):
                env.pop(key)
        env["RUSTUP_TOOLCHAIN"] = TOOLCHAIN
        run(["rustup", "toolchain", "install", TOOLCHAIN, "--profile", "minimal"], env=env)
        compiler = run(["rustup", "which", "--toolchain", TOOLCHAIN, "rustc"], env=env, quiet=True).strip()
        cargo = run(["rustup", "which", "--toolchain", TOOLCHAIN, "cargo"], env=env, quiet=True).strip()
        rustdoc = run(["rustup", "which", "--toolchain", TOOLCHAIN, "rustdoc"], env=env, quiet=True).strip()
        env.update({"RUSTC": compiler, "RUSTDOC": rustdoc})
        compiler_version = run([compiler, "-vV"], env=env)
        if f"release: {TOOLCHAIN}\n" not in compiler_version or f"commit-hash: {COMMIT}\n" not in compiler_version:
            raise RuntimeError("RA_SECURITY_COMPILER_IDENTITY_MISMATCH")
        (ARTIFACTS / "rustc-version.txt").write_text(compiler_version, encoding="utf-8")
        with tempfile.TemporaryDirectory(prefix="browseros-ra-security-", dir=os.environ["RUNNER_TEMP"]) as temporary:
            temporary_path = Path(temporary)
            checkout = temporary_path / "source"
            env["CARGO_TARGET_DIR"] = str(temporary_path / "target")
            run(["git", "-c", "core.autocrlf=false", "clone", "--filter=blob:none",
                 "--no-checkout", "--depth", "1", "--branch", TOOLCHAIN, REPOSITORY, str(checkout)])
            run(["git", "-C", str(checkout), "config", "core.autocrlf", "false"])
            run(["git", "-C", str(checkout), "sparse-checkout", "init", "--cone"])
            run(["git", "-C", str(checkout), "sparse-checkout", "set", SUBDIRECTORY])
            run(["git", "-C", str(checkout), "checkout", "--detach", COMMIT])
            if run(["git", "-C", str(checkout), "rev-parse", "HEAD"], quiet=True).strip() != COMMIT:
                raise RuntimeError("RA_SECURITY_UPSTREAM_COMMIT_MISMATCH")
            project = checkout / SUBDIRECTORY
            metadata_path = checkout / source["metadata_source_file"]
            require_hash(metadata_path, source["upstream_metadata_sha256"], "upstream metadata")
            upstream_lock = require_hash(project / "Cargo.lock", source["upstream_lock_sha256"], "upstream lock")
            xtask_path = checkout / XTASK_MANIFEST
            xtask_bytes = require_hash(xtask_path, XTASK_MANIFEST_HASH, "upstream xtask manifest")
            dist_path = checkout / XTASK_DIST
            original_dist = require_hash(dist_path, XTASK_DIST_HASH, "upstream xtask distribution source")
            for license_info in source["licenses"]:
                require_hash(checkout / license_info["source"], license_info["sha256"], license_info["file"])
            run(["git", "-C", str(checkout), "apply", "--check", str(patch)])
            run(["git", "-C", str(checkout), "apply", str(patch)])
            require_hash(metadata_path, source["patched_metadata_sha256"], "patched metadata")
            (project / "Cargo.lock").write_bytes(reviewed_lock)
            xtask_text = xtask_bytes.decode("utf-8")
            if xtask_text.count(ZIP_BEFORE) != 1:
                raise RuntimeError("RA_SECURITY_ZIP_MANIFEST_CONTEXT_MISMATCH")
            xtask_path.write_bytes(xtask_text.replace(ZIP_BEFORE, ZIP_AFTER).encode("utf-8"))
            dist_text = original_dist.decode("utf-8")
            zip_start = dist_text.index("fn zip(src_path: &Path, symbols_path: Option<&PathBuf>, dest_path: &Path) -> anyhow::Result<()> {")
            zip_end = dist_text.index("\nstruct Target {", zip_start)
            dist_text = dist_text[:zip_start] + ZIP_FUNCTION + dist_text[zip_end:]
            dist_path.write_bytes(dist_text.encode("utf-8"))
            status["phase"] = "resolve"
            for name, version in [("zip", "9.0.5"), ("crossbeam-epoch", "0.9.20"),
                                  ("anyhow", "1.0.103"), ("salsa", "0.28.5"),
                                  ("salsa-macro-rules", "0.28.5")]:
                run([cargo, "update", "-p", name, "--precise", version], cwd=project, env=env)
            resolved = (project / "Cargo.lock").read_bytes()
            packages = tomllib.loads(resolved.decode("utf-8"))["package"]
            expected = {"zip": "9.0.5", "crossbeam-epoch": "0.9.20", "anyhow": "1.0.103",
                        "salsa": "0.28.5", "salsa-macros": "0.28.5",
                        "url": "2.5.9", "form_urlencoded": "1.2.3", "rustix": "1.1.6"}
            for name, version in expected.items():
                selected = [package["version"] for package in packages if package["name"] == name]
                if selected != [version]:
                    raise RuntimeError("RA_SECURITY_RESOLVED_VERSION_MISMATCH:" + name + ":" + repr(selected))
            macro_rules = [p["version"] for p in packages if p["name"] == "salsa-macro-rules"]
            if not macro_rules or any(tuple(map(int, version.split("."))) < (0, 28, 5) for version in macro_rules):
                raise RuntimeError("RA_SECURITY_SALSA_MACRO_RULES_TOO_OLD")
            metadata = run([cargo, "metadata", "--locked", "--format-version=1"], cwd=project, env=env, quiet=True)
            (ARTIFACTS / "metadata.json").write_text(metadata, encoding="utf-8")
            changed = set(run(["git", "-C", str(checkout), "diff", "--name-only"], quiet=True).splitlines())
            allowed = {source["metadata_source_file"], SUBDIRECTORY + "/Cargo.lock", XTASK_MANIFEST, XTASK_DIST}
            if changed != allowed:
                raise RuntimeError("RA_SECURITY_UNEXPECTED_SOURCE_CHANGES:" + repr(sorted(changed)))
            patch_text = run(
                ["git", "-C", str(checkout), "diff", "--no-ext-diff", "--no-textconv",
                 "--full-index", "--", source["metadata_source_file"], XTASK_MANIFEST, XTASK_DIST],
                quiet=True,
            )
            # Git accepts GNU diff's bare empty context lines. Normalize only a
            # context line containing one space; never alter added/deleted text.
            # This keeps patch assets valid under the repository whitespace gate.
            patch_bytes = "".join(
                "\n" if line == " \n" else line
                for line in patch_text.splitlines(keepends=True)
            ).encode("utf-8")
            (ARTIFACTS / "Cargo.lock").write_bytes(resolved)
            candidate_patch = ARTIFACTS / "metadata-stdin.patch"
            candidate_patch.write_bytes(patch_bytes)
            original_patched = {
                path: path.read_bytes() for path in (metadata_path, xtask_path, dist_path)
            }
            run(["git", "-C", str(checkout), "apply", "--reverse", "--check",
                 "--whitespace=error-all", str(candidate_patch)])
            run(["git", "-C", str(checkout), "apply", "--reverse",
                 "--whitespace=error-all", str(candidate_patch)])
            run(["git", "-C", str(checkout), "apply", "--check",
                 "--whitespace=error-all", str(candidate_patch)])
            run(["git", "-C", str(checkout), "apply",
                 "--whitespace=error-all", str(candidate_patch)])
            for path, expected_bytes in original_patched.items():
                if path.read_bytes() != expected_bytes:
                    raise RuntimeError("RA_SECURITY_PATCH_ROUNDTRIP_MISMATCH:" + path.name)
            (ARTIFACTS / "xtask.Cargo.toml").write_bytes(xtask_path.read_bytes())
            (ARTIFACTS / "xtask.dist.rs").write_bytes(dist_path.read_bytes())
            source["patch_sha256"] = sha256(patch_bytes)
            source["lock_sha256"] = sha256(resolved)
            source.update({
                "xtask_manifest_file": XTASK_MANIFEST,
                "upstream_xtask_manifest_sha256": XTASK_MANIFEST_HASH,
                "patched_xtask_manifest_sha256": sha256(xtask_path.read_bytes()),
                "xtask_dist_source_file": XTASK_DIST,
                "upstream_xtask_dist_sha256": XTASK_DIST_HASH,
                "patched_xtask_dist_sha256": sha256(dist_path.read_bytes()),
            })
            upstream_versions: dict[str, list[str]] = {}
            resolved_versions: dict[str, list[str]] = {}
            for package in tomllib.loads(upstream_lock.decode("utf-8"))["package"]:
                upstream_versions.setdefault(package["name"], []).append(package["version"])
            for package in packages:
                resolved_versions.setdefault(package["name"], []).append(package["version"])
            source["lock_updates"] = [
                {"name": name, "from": ",".join(sorted(upstream_versions.get(name, []))),
                 "to": ",".join(sorted(resolved_versions.get(name, [])))}
                for name in sorted(upstream_versions.keys() | resolved_versions.keys())
                if sorted(upstream_versions.get(name, [])) != sorted(resolved_versions.get(name, []))
            ]
            (ARTIFACTS / "source.candidate.json").write_text(
                json.dumps(source, indent=2, ensure_ascii=False) + "\n", encoding="utf-8",
            )
            candidate_files = [ARTIFACTS / name for name in (
                "Cargo.lock", "metadata-stdin.patch", "xtask.Cargo.toml", "xtask.dist.rs", "source.candidate.json",
            )]
            # Make valid resolver output reviewable even if subsequent compilation fails.
            for candidate_file in candidate_files:
                transport(candidate_file, source_sha)
            status["phase"] = "audit"
            audit = run(["cargo", "audit", "--file", str(ARTIFACTS / "Cargo.lock")])
            (ARTIFACTS / "candidate.audit.log").write_text(audit, encoding="utf-8")
            status["phase"] = "xtask-check"
            run([cargo, "check", "--locked", "-p", "xtask", "--all-targets"], cwd=project, env=env)
            status["phase"] = "xtask-zip-roundtrip"
            patched_dist = dist_path.read_bytes()
            env["BROWSEROS_RA_ZIP_CANDIDATE_DIRECTORY"] = str(temporary_path / "zip-fixture")
            try:
                dist_path.write_bytes(patched_dist + ZIP_TEST.encode("utf-8"))
                result = run(
                    [cargo, "test", "--locked", "-p", "xtask", TEST_NAME, "--",
                     "--exact", "--show-output", "--color", "never"],
                    cwd=project, env=env,
                )
                if (
                    re.search(r"^test " + re.escape(TEST_NAME) + r" \.\.\. ok$", result, re.MULTILINE) is None
                    or "test result: ok. 1 passed; 0 failed; 0 ignored;" not in result
                ):
                    raise RuntimeError("RA_SECURITY_ZIP_ROUNDTRIP_NOT_EXECUTED")
                (ARTIFACTS / "zip-roundtrip.log").write_text(result, encoding="utf-8")
            finally:
                dist_path.write_bytes(patched_dist)
            require_hash(project / "Cargo.lock", source["lock_sha256"], "lock after checks")
            require_hash(metadata_path, source["patched_metadata_sha256"], "metadata after checks")
            require_hash(xtask_path, source["patched_xtask_manifest_sha256"], "xtask manifest after checks")
            require_hash(dist_path, source["patched_xtask_dist_sha256"], "distribution source after checks")
            status.update({"result": "CANDIDATE_AUDIT_AND_XTASK_PASS", "phase": "complete"})
            print("RA_SECURITY_CANDIDATE_RESULT=CANDIDATE_ONLY; WINDOWS_RUNTIME_ACCEPTANCE=NOT_RUN", flush=True)
    except BaseException as error:
        status["error"] = type(error).__name__ + ":" + str(error)
        raise
    finally:
        (ARTIFACTS / "status.json").write_text(json.dumps(status, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
