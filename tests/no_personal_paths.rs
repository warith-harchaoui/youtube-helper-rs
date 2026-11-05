//! No machine-specific path in a tracked file.
//!
//! A path written in a tracked file travels with every clone, every crate and
//! every published page. Two families of path never belong there:
//!
//! 1. A path that names a machine or an account, in any tracked text file: a
//!    home under `/Users/`, `/home/` or `C:\Users\`, a mounted volume under
//!    `/Volumes/`, or a `~/<folder>` that only exists on one machine.
//!    Placeholders (`/home/user`, `/Users/you`) name nobody and pass.
//! 2. A hard-coded absolute path in Rust code: a string literal whose whole
//!    value is a filesystem path. A path comes from an environment variable,
//!    an argument or a config; a default is computed at run time. The only
//!    literals left are the ones an OS imposes (`SYSTEM_PATHS`) and the
//!    entries of `EXCEPTIONS`, each with its reason.
//!
//! The scan reads `git ls-files`, so it sees exactly what a clone sees. This
//! file is excluded from its own scan: it has to spell what it hunts.

use std::path::Path;
use std::process::Command;

const SELF: &str = "tests/no_personal_paths.rs";

/// Account names that name nobody: examples and CI accounts.
const PLACEHOLDER_USERS: &[&str] = &[
    "user",
    "username",
    "you",
    "vous",
    "me",
    "moi",
    "alice",
    "bob",
    "bobby",
    "runner",
    "runneradmin",
    "app",
    "dev",
];
/// `~/<x>` that exists on any machine, or is obviously an example.
const GENERIC_HOME_DIRS: &[&str] = &[
    "Downloads",
    "Documents",
    "Desktop",
    "Library",
    "AppData",
    "project",
    "projects",
    "...",
];
/// Filesystem roots: a literal starting with one of these is a path, not a URL route.
const ROOTS: &[&str] = &[
    "Users",
    "home",
    "Volumes",
    "tmp",
    "var",
    "opt",
    "usr",
    "etc",
    "private",
    "mnt",
    "srv",
    "data",
    "root",
    "app",
    "Library",
    "System",
    "Applications",
    "nix",
    "snap",
    "dev",
    "proc",
    "sys",
    "bin",
    "sbin",
    "lib",
    "run",
    "media",
];
/// Literals an OS or a protocol imposes: nobody can choose another value.
const SYSTEM_PATHS: &[&str] = &[
    "/dev/null",
    "/dev/stdin",
    "/dev/stdout",
    "/dev/stderr",
    "/usr/bin/env",
    "/bin/sh",
    "/bin/bash",
    "/bin/zsh",
    "/etc/passwd",
];
/// Prefixes of the same kind: kernel interfaces, devices, font folders.
const SYSTEM_PREFIXES: &[&str] = &[
    "/proc/",
    "/sys/",
    "/dev/video",
    "/Library/Fonts",
    "/System/Library/Fonts",
    "/usr/share/fonts",
    "/Applications/",
];
/// Obviously fictitious paths.
const FICTITIOUS_PREFIXES: &[&str] = &["/path/to/", "/home/user/", "/Users/you/"];

/// `(file, literal, reason)`: literals the scan flags that are right where they stand.
const EXCEPTIONS: &[(&str, &str, &str)] = &[
    // Candidate yt-dlp paths fed to the pure chooser under test, never opened.
    (
        "src/provision.rs",
        "/usr/bin/yt-dlp",
        "a candidate on PATH, never opened",
    ),
    (
        "src/provision.rs",
        "/opt/a-moi/yt-dlp",
        "a declared candidate, never opened",
    ),
    // System prefixes the cache directory must never fall under.
    ("src/provision.rs", "/usr", "a prefix the test refuses"),
    ("src/provision.rs", "/opt", "a prefix the test refuses"),
];

const TEXT_SUFFIXES: &[&str] = &[
    "rs", "md", "txt", "toml", "yml", "yaml", "json", "sh", "html", "py", "cfg", "ini", "lock",
];

fn tracked() -> Vec<String> {
    let root = env!("CARGO_MANIFEST_DIR");
    let out = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(root)
        .output()
        .expect("git ls-files");
    String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|f| !f.is_empty() && *f != SELF)
        .map(str::to_owned)
        .collect()
}

fn is_text(rel: &str) -> bool {
    let p = Path::new(rel);
    p.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| TEXT_SUFFIXES.contains(&e))
        || matches!(
            p.file_name().and_then(|n| n.to_str()),
            Some("Dockerfile" | "Makefile" | "pre-push" | ".gitignore")
        )
}

fn excepted(rel: &str, literal: &str) -> bool {
    EXCEPTIONS
        .iter()
        .any(|(file, lit, _why)| *file == rel && *lit == literal)
}

/// The run of name characters starting at `s`.
fn name_at(s: &str) -> &str {
    let end = s
        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.' || c == '-'))
        .unwrap_or(s.len());
    &s[..end]
}

/// True when the byte before `i` cannot precede a path (a word, `$`, `.`, `-`).
fn starts_token(line: &str, i: usize, also: &[char]) -> bool {
    line[..i].chars().next_back().is_none_or(|c| {
        !(c.is_alphanumeric() || c == '_' || c == '.' || c == '$' || c == '-' || also.contains(&c))
    })
}

fn personal_paths(rel: &str, text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let n = n + 1;
        for prefix in [
            "/Users/",
            "/home/",
            "/Volumes/",
            ":\\Users\\",
            ":\\\\Users\\\\",
        ] {
            for (i, _) in line.match_indices(prefix) {
                let start = if prefix.starts_with(':') {
                    i.saturating_sub(1)
                } else {
                    i
                };
                if !starts_token(line, start, &[]) {
                    continue;
                }
                let name = name_at(&line[i + prefix.len()..]);
                if name.is_empty() {
                    continue;
                }
                let placeholder = if prefix == "/Volumes/" {
                    name == "some-unmounted-disk"
                } else {
                    PLACEHOLDER_USERS.contains(&name)
                };
                let hit = format!("{prefix}{name}");
                if !placeholder && !excepted(rel, &hit) {
                    found.push(format!("{rel}:{n}: {hit}"));
                }
            }
        }
        if rel.starts_with(".github/workflows/") {
            continue; // the runner's home, rebuilt for every job
        }
        for (i, _) in line.match_indices("~/") {
            if !starts_token(line, i, &['/', '~']) {
                continue;
            }
            let rest = &line[i + 2..];
            let name = if rest.starts_with("...") {
                "..."
            } else {
                name_at(rest)
            };
            if name.is_empty() || name.starts_with('.') || GENERIC_HOME_DIRS.contains(&name) {
                continue;
            }
            let hit = format!("~/{name}");
            if !excepted(rel, &hit) {
                found.push(format!("{rel}:{n}: {hit}"));
            }
        }
    }
    found
}

/// The string literals of the code part of each line (comments skipped).
fn rust_literals(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("//") {
            continue;
        }
        let mut chars = line.char_indices().peekable();
        while let Some((_, c)) = chars.next() {
            if c != '"' {
                continue;
            }
            let mut lit = String::new();
            while let Some((_, d)) = chars.next() {
                match d {
                    '\\' => {
                        if let Some((_, e)) = chars.next() {
                            lit.push('\\');
                            lit.push(e);
                        }
                    }
                    '"' => break,
                    _ => lit.push(d),
                }
            }
            out.push((n + 1, lit));
        }
    }
    out
}

fn is_absolute_path(lit: &str) -> bool {
    if let Some(rest) = lit.strip_prefix('/') {
        let first = rest.split('/').next().unwrap_or("");
        return ROOTS.contains(&first);
    }
    let b = lit.as_bytes();
    b.len() > 2 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'\\'
}

fn hardcoded_paths(rel: &str, text: &str) -> Vec<String> {
    if !rel.ends_with(".rs") {
        return Vec::new();
    }
    rust_literals(text)
        .into_iter()
        .filter(|(_, lit)| {
            is_absolute_path(lit)
                && !SYSTEM_PATHS.contains(&lit.as_str())
                && !SYSTEM_PREFIXES.iter().any(|p| lit.starts_with(p))
                && !FICTITIOUS_PREFIXES.iter().any(|p| lit.starts_with(p))
                && !excepted(rel, lit)
        })
        .map(|(n, lit)| format!("{rel}:{n}: {lit:?}"))
        .collect()
}

fn scan() -> (Vec<String>, Vec<String>) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let (mut personal, mut hardcoded) = (Vec::new(), Vec::new());
    for rel in tracked() {
        if !is_text(&rel) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(root.join(&rel)) else {
            continue;
        };
        personal.extend(personal_paths(&rel, &text));
        hardcoded.extend(hardcoded_paths(&rel, &text));
    }
    (personal, hardcoded)
}

#[test]
fn no_personal_path_in_tracked_files() {
    let (personal, _) = scan();
    assert!(
        personal.is_empty(),
        "paths naming a machine or an account in tracked files \
         (use an environment variable or an argument):\n{}",
        personal.join("\n")
    );
}

#[test]
fn no_hardcoded_absolute_path_in_code() {
    let (_, hardcoded) = scan();
    assert!(
        hardcoded.is_empty(),
        "hard-coded absolute paths in code (read an environment variable or an \
         argument, or compute the default at run time):\n{}",
        hardcoded.join("\n")
    );
}

#[test]
fn the_scan_catches_what_it_hunts() {
    let who = ["war", "ith"].concat();
    assert!(!personal_paths("x.md", &format!("see /Users/{who}/notes")).is_empty());
    assert!(!personal_paths("x.md", &format!("C:\\Users\\{who}\\x")).is_empty());
    assert!(!personal_paths("x.md", &["on /Vol", "umes/scratch/x"].concat()).is_empty());
    assert!(!personal_paths("x.md", &["from ~", "/my-checkout"].concat()).is_empty());
    assert!(personal_paths("x.md", "/home/user/a ~/.cache/x ~/Downloads").is_empty());
    let tmp = ["let p = \"/t", "mp/run.log\";"].concat();
    assert!(!hardcoded_paths("x.rs", &tmp).is_empty());
    assert!(hardcoded_paths("x.rs", "let n = \"/dev/null\"; let r = \"/health\";").is_empty());
    assert!(hardcoded_paths("x.rs", "// see \"/var/lib/x\"").is_empty());
}
