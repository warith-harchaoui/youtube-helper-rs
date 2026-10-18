//! The agent-skill surface is checked, not merely written.
//!
//! The Python side of this suite enforces its skills with
//! `scripts/check_triggers.py` wired into pytest. Rust has no equivalent, so
//! this integration test is the gate: a skill that drifts out of spec — a
//! description over the published cap, a renamed folder, a reference that no
//! longer exists, a trigger catalogue that stopped matching its mirror — fails
//! `cargo test` and therefore fails the pre-push hook.
//!
//! Every rule below comes from the published skill standard or from this
//! suite's own convention, and each assertion says which.

use std::fs;
use std::path::{Path, PathBuf};

/// The skill's folder name, which must also be its `name:` field.
const SKILL: &str = "youtube-helper-rs";

/// The published cap on `description`, in characters.
const DESCRIPTION_CAP: usize = 1024;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn skill_dir() -> PathBuf {
    repo_root().join("skills").join(SKILL)
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// Split a Markdown file into its YAML frontmatter and its body.
fn frontmatter(text: &str) -> (String, String) {
    let rest = text
        .strip_prefix("---\n")
        .expect("SKILL.md must open with a `---` frontmatter fence");
    let end = rest
        .find("\n---\n")
        .expect("SKILL.md frontmatter must be closed by a `---` line");
    (rest[..end].to_string(), rest[end + 5..].to_string())
}

/// Pull one scalar field out of the frontmatter, folding YAML's `>-` blocks.
fn field(fm: &str, key: &str) -> String {
    let mut lines = fm.lines();
    let head = lines
        .find(|l| l.starts_with(&format!("{key}:")))
        .unwrap_or_else(|| panic!("frontmatter is missing `{key}:`"));
    let inline = head[key.len() + 1..].trim();
    if inline != ">-" && !inline.is_empty() {
        return inline.to_string();
    }
    // Folded block: take the indented continuation lines and join them.
    let mut out = String::new();
    for line in lines {
        if !line.starts_with(' ') {
            break;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(line.trim());
    }
    out
}

#[test]
fn the_skill_declares_the_name_of_its_own_folder() {
    // Standard: `name` must be kebab-case and match the directory, or the host
    // cannot resolve the skill it just loaded.
    let (fm, _) = frontmatter(&read(&skill_dir().join("SKILL.md")));
    let name = field(&fm, "name");
    assert_eq!(name, SKILL, "`name:` must equal the folder name");
    assert!(
        name.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
        "`name:` must be kebab-case, got {name:?}"
    );
}

#[test]
fn the_description_fits_the_published_cap_and_says_when_to_skip() {
    let (fm, _) = frontmatter(&read(&skill_dir().join("SKILL.md")));
    let description = field(&fm, "description");
    assert!(
        description.chars().count() <= DESCRIPTION_CAP,
        "description is {} characters, over the {DESCRIPTION_CAP} cap",
        description.chars().count()
    );
    // Convention: the description is all the host model sees before deciding to
    // load the skill, so it has to carry the negative cases too. Without them a
    // skill fires on work that belongs to a sibling.
    assert!(
        description.contains("Do NOT use"),
        "description must carry an explicit SKIP clause"
    );
    // Every operation the crate actually performs must be reachable from the
    // description, or it can never be the reason the skill loads.
    for bucket in ["metadata", "resolve", "download"] {
        assert!(
            description.to_lowercase().contains(bucket),
            "description never mentions the `{bucket}` operation"
        );
    }
}

#[test]
fn the_frontmatter_carries_no_angle_brackets() {
    // Standard: `<` and `>` inside frontmatter *values* break parsers in the
    // wild — a contact address written as `Name <a@b.c>` is the usual way they
    // sneak in. YAML's own block-scalar indicators (`>-`, `>`, `|`, `|-`) are
    // not that, so they are stripped before the check rather than banned: the
    // folded `description: >-` that every skill in this suite uses would
    // otherwise fail its own gate.
    let (fm, _) = frontmatter(&read(&skill_dir().join("SKILL.md")));
    let stripped: String = fm
        .lines()
        .map(|line| match line.split_once(':') {
            Some((key, value)) if matches!(value.trim(), ">-" | ">" | "|" | "|-") => {
                key.to_string()
            }
            _ => line.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !stripped.contains('<') && !stripped.contains('>'),
        "frontmatter values must not contain angle brackets"
    );
}

#[test]
fn the_skill_folder_holds_no_readme() {
    // Standard: a README.md inside a skill folder shadows SKILL.md for some
    // hosts. The installation notes live one level up, in skills/README.md.
    assert!(
        !skill_dir().join("README.md").exists(),
        "skills/{SKILL}/README.md must not exist; put it in skills/README.md"
    );
    assert!(
        repo_root().join("skills/README.md").exists(),
        "skills/README.md must exist and explain installation"
    );
}

#[test]
fn every_reference_the_skill_names_actually_exists() {
    let body = read(&skill_dir().join("SKILL.md"));
    let mut seen = 0;
    for (index, _) in body.match_indices("references/") {
        let tail = &body[index + "references/".len()..];
        let name: String = tail
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
            .collect();
        if !name.ends_with(".md") {
            continue;
        }
        let path = skill_dir().join("references").join(&name);
        assert!(
            path.exists(),
            "SKILL.md points at a missing {}",
            path.display()
        );
        seen += 1;
    }
    assert!(
        seen >= 3,
        "expected the three reference files to be cited, saw {seen}"
    );
}

#[test]
fn the_trigger_catalogue_is_mirrored_not_forked() {
    // Convention: the root TRIGGERS.md is the user-facing catalogue and
    // references/triggers.md is the agent-facing mirror. Only the heading
    // differs; everything below it must stay identical, or the two drift and
    // the catalogue stops being authoritative.
    let root = read(&repo_root().join("TRIGGERS.md"));
    let mirror = read(&skill_dir().join("references/triggers.md"));
    let body_of = |text: &str| -> String {
        text.split_once('\n')
            .map(|(_, rest)| rest.trim_start().to_string())
            .unwrap_or_default()
    };
    assert!(
        mirror.ends_with(&body_of(&root)),
        "references/triggers.md no longer mirrors the root TRIGGERS.md"
    );
    // The catalogue is useless without its negative half.
    assert!(
        root.contains("SKIP"),
        "TRIGGERS.md must carry a SKIP section"
    );
}
