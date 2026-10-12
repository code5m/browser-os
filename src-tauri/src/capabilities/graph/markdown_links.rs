//! Pure bounded note-link extraction and resolution. No file I/O, Tauri, or second graph store.
//! Relative Markdown/Wiki targets are resolved only inside the currently indexed vault.

use std::collections::BTreeSet;

pub const MAX_LINKS_PER_NOTE: usize = 128;

/// Extract [[Wiki]], [[Wiki|alias]], [[Wiki#heading]], and [label](relative.md).
/// Ignore fenced / inline code, schemes, absolute paths and malformed targets.
pub fn note_links(body: &str) -> Vec<String> {
    let mut found = BTreeSet::new();
    let mut fence: Option<(char, usize)> = None;
    for line in body.lines() {
        let trimmed = line.trim_start();
        let marker = trimmed.chars().next().unwrap_or(' ');
        if marker == '`' || marker == '~' {
            let width = trimmed.chars().take_while(|c| *c == marker).count();
            if width >= 3 {
                match fence {
                    None => {
                        fence = Some((marker, width));
                        continue;
                    }
                    Some((c, w)) if c == marker && width >= w => {
                        fence = None;
                        continue;
                    }
                    _ => {}
                }
            }
        }
        if fence.is_some() {
            continue;
        }
        let mut visible = String::with_capacity(line.len());
        let mut in_inline = false;
        for c in line.chars() {
            if c == '`' {
                in_inline = !in_inline;
            } else if !in_inline {
                visible.push(c);
            }
        }
        let bytes = visible.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() && found.len() < MAX_LINKS_PER_NOTE {
            if bytes[i..].starts_with(b"[[") {
                if let Some(end) = visible[i + 2..].find("]]") {
                    let raw = &visible[i + 2..i + 2 + end];
                    if let Some(link) = clean_target(raw) {
                        found.insert(link);
                    }
                    i += 2 + end + 2;
                    continue;
                }
            }
            if bytes[i] == b']' && bytes.get(i + 1) == Some(&b'(') {
                if let Some(end) = visible[i + 2..].find(')') {
                    let raw = &visible[i + 2..i + 2 + end];
                    if let Some(link) = clean_target(raw) {
                        found.insert(link);
                    }
                    i += 2 + end + 1;
                    continue;
                }
            }
            i += 1;
        }
        if found.len() >= MAX_LINKS_PER_NOTE {
            break;
        }
    }
    found.into_iter().collect()
}

fn clean_target(raw: &str) -> Option<String> {
    let raw = raw.trim().trim_start_matches('<').trim_end_matches('>');
    let raw = raw.split('|').next()?.split('#').next()?.trim();
    let decoded = urlencoding::decode(raw).ok()?;
    let target = decoded.trim();
    if target.is_empty()
        || target.len() > 256
        || target.starts_with('/')
        || target.starts_with('\\')
        || target.contains('\\')
        || target.contains(':')
        || target.contains('\0')
        || target.starts_with("//")
    {
        return None;
    }
    Some(target.to_owned())
}

/// Resolve path only if unique. Same-folder wins; vault-root path next; bare title
/// is accepted only if it names exactly one note. Traversal above vault fails closed.
pub fn resolve_note(target: &str, source: &str, all_paths: &[String]) -> Option<String> {
    let target = clean_target(target)?;
    let filename = if target.to_ascii_lowercase().ends_with(".md") {
        target
    } else {
        format!("{target}.md")
    };
    let normalized = |path: &str| -> Option<String> {
        let mut parts: Vec<&str> = Vec::new();
        for part in path.split('/') {
            match part {
                "" | "." => {}
                ".." => {
                    parts.pop()?;
                }
                _ => parts.push(part),
            }
        }
        Some(parts.join("/"))
    };
    let dir = source.rsplit_once('/').map(|(dir, _)| dir).unwrap_or("");
    let relative = normalized(&format!("{dir}/{filename}"))?;
    if all_paths.contains(&relative) {
        return Some(relative);
    }
    let rooted = normalized(&filename)?;
    if all_paths.contains(&rooted) {
        return Some(rooted);
    }
    if filename.contains('/') {
        return None;
    }
    let mut matches = all_paths
        .iter()
        .filter(|path| path.rsplit('/').next() == Some(filename.as_str()));
    let found = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    Some(found.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_and_bounds_notes() {
        let s = "# Note\nSee [[A|label]], [[notes/B#heading]], [read](../C.md), [[Missing]].\n```\n[[Hidden]]\n```\n`[[Inline]]`";
        let links = note_links(s);
        assert!(links.contains(&"A".to_owned()));
        assert!(links.contains(&"notes/B".to_owned()));
        assert!(links.contains(&"../C.md".to_owned()));
        assert!(!links.contains(&"Hidden".to_owned()));
        assert!(!links.contains(&"Inline".to_owned()));
        assert!(!note_links("[x](https://evil.example) [[../../secret]]")
            .contains(&"https://evil.example".to_owned()));
        assert!(note_links(&"[[A]]".repeat(300)).len() <= MAX_LINKS_PER_NOTE);
    }
    #[test]
    fn disambiguates_and_prevents_traversal() {
        let paths = vec!["A.md", "folder/A.md", "x/Duplicate.md", "y/Duplicate.md"]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>();
        assert_eq!(
            resolve_note("A", "folder/source.md", &paths),
            Some("folder/A.md".into())
        );
        assert_eq!(resolve_note("Duplicate", "folder/source.md", &paths), None);
        assert_eq!(resolve_note("../../A", "folder/source.md", &paths), None);
        assert_eq!(
            resolve_note("/etc/shadow", "folder/source.md", &paths),
            None
        );
    }
}
