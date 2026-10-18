# youtube-helper-rs as an agent skill

`skills/youtube-helper-rs/` packages this crate as a **Claude Skill** *and* an
**OpenCode skill** — both ecosystems read the same `SKILL.md` (YAML frontmatter
+ Markdown body + progressive-disclosure `references/`). Installing it lets an
agent discover the crate and read metadata, resolve media URLs and download
audio on the user's behalf, without the user opening a terminal.

## Layout

```
skills/youtube-helper-rs/
├── SKILL.md                  # name + trigger-rich description + instructions
└── references/
    ├── cli-reference.md      # every subcommand, flag and exit code
    ├── surfaces.md           # which surfaces exist here, and which do not
    └── triggers.md           # exhaustive, auditable trigger catalogue
```

Progressive disclosure, as the published skill standard prescribes: the YAML
frontmatter is always in the host model's context, `SKILL.md`'s body loads only
when the skill looks relevant, and `references/*.md` load only when a task
actually needs that depth. The `description` stays under the 1024-character cap;
the exhaustive trigger list lives in `references/triggers.md`.

## Install for Claude Code / Claude Desktop

Skills live under `~/.claude/skills/` (user) or `.claude/skills/` (project). To
track this repo's copy rather than duplicate it, symlink it:

```bash
ln -sfn "$PWD/skills/youtube-helper-rs" ~/.claude/skills/youtube-helper-rs
# per-project instead:
mkdir -p /path/to/project/.claude/skills
ln -sfn "$PWD/skills/youtube-helper-rs" /path/to/project/.claude/skills/youtube-helper-rs
```

## Install for OpenCode

OpenCode reads skills from `~/.opencode/skills/` (or `~/.config/opencode/skills/`):

```bash
mkdir -p ~/.opencode/skills
ln -sfn "$PWD/skills/youtube-helper-rs" ~/.opencode/skills/youtube-helper-rs
```

## Keeping triggers enforced

The host model only sees `SKILL.md`'s `description` before deciding to load the
skill, so every real trigger must be represented there, compressed into the
1024-character budget. `references/triggers.md` is the human-reviewable
superset and mirrors the repo-root `TRIGGERS.md`. When one changes, change the
other.

This crate is network-bound by definition — fetching a remote URL is its job.
It talks to the platform through `yt-dlp` and writes to local paths only: no
account, no telemetry, no third-party relay. Nothing of the user's is uploaded.
