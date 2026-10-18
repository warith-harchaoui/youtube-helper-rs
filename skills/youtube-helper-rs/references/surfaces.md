# Surfaces — youtube-helper-rs

The Python suite's rule is that a helper exposes five surfaces: a library, two
CLIs, a FastAPI HTTP server, an MCP tool set mounted on that same app, and an
installable agent skill. This crate deliberately carries three of them, and the
reasoning for the other two matters more than the gap.

## What exists

**Library** — `cargo add youtube-helper-rs`. The primary surface. Typed values
(`VideoMetadata`, `AudioFormat`, `DownloadOptions`) and a `thiserror` enum
(`YoutubeHelperError`); no stringly-typed errors reach a caller.

**CLI** — `cargo install youtube-helper-rs` gives the `youtube-helper-rs`
binary. Thin by design: parse arguments, call one library function, print,
exit. See `cli-reference.md`.

**Agent skill** — this directory.

## What does not exist, and why

**No HTTP API.** The Python twin already serves one, and it covers a strictly
larger feature set. A second HTTP surface with fewer routes would be a second
thing to keep in sync, for no gain.

**No MCP.** This is the important one. The suite's rule is that MCP is *never*
a second implementation: `fastapi-mcp` mounts `/mcp` onto the FastAPI app the
helper already serves, so every MCP tool **is** one of its routes and the two
cannot drift apart. Rust has no equivalent of that mounting trick, so an MCP
surface here would necessarily be a separate implementation of the same tools —
exactly the drift the rule exists to prevent. Agents reach this crate through
the skill above, and through the Python twin's `/mcp` when they need the full
feature set.

**No GUI.** Nothing here is visual.

## Choosing between the two implementations

| Need | Reach for |
|---|---|
| Rust calling code, one static binary, no Python runtime | `youtube-helper-rs` |
| Typed metadata and a clean error enum | `youtube-helper-rs` |
| Video track, thumbnails, subtitles | Python `youtube-helper` |
| Channel listings, engagement figures | Python `youtube-helper` |
| Stream catalogue and picker | Python `youtube-helper` |
| HTTP API, MCP tools, browser GUI | Python `youtube-helper` |

Both install side by side without colliding.
