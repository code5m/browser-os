//! Recognize the two supported Cargo package-output suffixes without walking
//! arbitrary ancestors. The returned prefix preserves the selected target and
//! profile; PROFILE cannot reconstruct custom profile directory names.

use std::ffi::OsStr;
use std::path::{Component, Path};

fn unit_hash(value: &str) -> bool {
    value.len() == 16
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

pub fn executable_directory<'a>(output: &'a Path, package: &str) -> Option<&'a Path> {
    if !output.is_absolute()
        || output
            .components()
            .any(|component| matches!(component, Component::ParentDir))
        || package.is_empty()
        || !package
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        || output.file_name() != Some(OsStr::new("out"))
    {
        return None;
    }

    let unit = output.parent()?;
    let unit_name = unit.file_name()?.to_str()?;
    let build = if unit_hash(unit_name) {
        // Cargo 1.100: <profile>/build/<package>/<unit-hash>/out.
        let package_directory = unit.parent()?;
        if package_directory.file_name() != Some(OsStr::new(package)) {
            return None;
        }
        package_directory.parent()?
    } else {
        // Older Cargo: <profile>/build/<package>-<unit-hash>/out.
        let hash = unit_name.strip_prefix(package)?.strip_prefix('-')?;
        if !unit_hash(hash) {
            return None;
        }
        unit.parent()?
    };
    if build.file_name() != Some(OsStr::new("build")) {
        return None;
    }
    let profile = build.parent()?;
    profile.file_name()?;
    profile.parent()?;
    Some(profile)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const PACKAGE: &str = "mvp-browser-os";
    const HASH: &str = "ebc9cf92ff347765";

    fn fixture_root() -> PathBuf {
        if cfg!(windows) {
            PathBuf::from(r"C:\cargo-layout-fixture")
        } else {
            PathBuf::from("/cargo-layout-fixture")
        }
    }

    fn legacy(profile: &Path, package: &str, hash: &str) -> PathBuf {
        profile
            .join("build")
            .join(format!("{package}-{hash}"))
            .join("out")
    }

    fn unit(profile: &Path, package: &str, hash: &str) -> PathBuf {
        profile.join("build").join(package).join(hash).join("out")
    }

    #[test]
    fn legacy_layout_preserves_profile_directory() {
        for profile_name in ["debug", "release"] {
            let profile = fixture_root().join("target").join(profile_name);
            let output = legacy(&profile, PACKAGE, HASH);
            assert_eq!(
                executable_directory(&output, PACKAGE),
                Some(profile.as_path())
            );
        }
    }

    #[test]
    fn unit_layout_preserves_profile_directory() {
        for profile_name in ["debug", "release"] {
            let profile = fixture_root().join("target").join(profile_name);
            let output = unit(&profile, PACKAGE, HASH);
            assert_eq!(
                executable_directory(&output, PACKAGE),
                Some(profile.as_path())
            );
        }
    }

    #[test]
    fn target_prefix_and_custom_profile_are_preserved() {
        for target in ["x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc"] {
            let profile = fixture_root()
                .join("custom-artifacts")
                .join(target)
                .join("analysis");
            for output in [
                legacy(&profile, PACKAGE, HASH),
                unit(&profile, PACKAGE, HASH),
            ] {
                assert_eq!(
                    executable_directory(&output, PACKAGE),
                    Some(profile.as_path())
                );
                let actual = executable_directory(&output, PACKAGE).unwrap();
                assert_eq!(actual.file_name(), Some(OsStr::new("analysis")));
                assert_eq!(
                    actual.parent().unwrap().file_name(),
                    Some(OsStr::new(target))
                );
            }
        }
    }

    #[test]
    fn unrelated_packages_and_invalid_hashes_are_rejected() {
        let profile = fixture_root().join("target").join("debug");
        for other in [
            "different-package",
            "mvp-browser-os2",
            "mvp-browser-os-extra",
        ] {
            for output in [legacy(&profile, other, HASH), unit(&profile, other, HASH)] {
                assert_eq!(executable_directory(&output, PACKAGE), None);
            }
        }
        for hash in [
            "",
            "123",
            "ebc9cf92ff34776",
            "ebc9cf92ff3477650",
            "EBC9CF92FF347765",
            "gbc9cf92ff347765",
        ] {
            for output in [
                legacy(&profile, PACKAGE, hash),
                unit(&profile, PACKAGE, hash),
            ] {
                assert_eq!(executable_directory(&output, PACKAGE), None);
            }
        }
        let output = unit(&profile, PACKAGE, HASH);
        for package in ["", "mvp-browser", "../mvp-browser-os", "mvp/browser-os"] {
            assert_eq!(executable_directory(&output, package), None);
        }
    }

    #[test]
    fn non_absolute_traversal_and_unexpected_suffixes_are_rejected() {
        let profile = fixture_root().join("target").join("debug");
        let relative = PathBuf::from("target").join("debug");
        let traversal = fixture_root().join("target").join("..").join("debug");
        let invalid = [
            legacy(&relative, PACKAGE, HASH),
            unit(&relative, PACKAGE, HASH),
            legacy(&traversal, PACKAGE, HASH),
            unit(&traversal, PACKAGE, HASH),
            profile.join("other").join(PACKAGE).join(HASH).join("out"),
            profile
                .join("build")
                .join("extra")
                .join(PACKAGE)
                .join(HASH)
                .join("out"),
            profile
                .join("build")
                .join(format!("{PACKAGE}-{HASH}"))
                .join("output"),
            unit(&profile, PACKAGE, HASH).join("nested"),
            fixture_root().join("build").join(PACKAGE).join(HASH),
        ];
        for output in invalid {
            assert_eq!(executable_directory(&output, PACKAGE), None, "{output:?}");
        }
        #[cfg(windows)]
        for output in [
            r"C:target\debug\build\mvp-browser-os\ebc9cf92ff347765\out",
            r"\target\debug\build\mvp-browser-os\ebc9cf92ff347765\out",
        ] {
            assert_eq!(executable_directory(Path::new(output), PACKAGE), None);
        }
    }
}
