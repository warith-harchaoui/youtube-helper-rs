# CLI reference — `youtube-helper-rs`

Installed with `cargo install youtube-helper-rs`. The binary is named after the
crate, not after the Python twin's `youtube-helper`, so both can sit on `PATH`
without either shadowing the other.

## Shape

```
youtube-helper-rs <COMMAND>
```

Every command takes the URL as its first positional argument. Any URL `yt-dlp`
can extract is accepted; it must start with `http://` or `https://`, which is
checked before `yt-dlp` is invoked at all.

## `metadata`

```
youtube-helper-rs metadata <URL> [-p|--pretty]
```

Prints the video's metadata as JSON on stdout. One dense line by default;
`--pretty` indents it.

Fields: `id`, `title`, `duration`, `uploader`, `channel`, `webpage_url`,
`description`, `upload_date`, `view_count`, `like_count`, `thumbnail`. Every
field but `id` and `title` is nullable — the set `yt-dlp` returns varies by
site, and absent is reported as `null` rather than invented.

## `resolve`

```
youtube-helper-rs resolve <URL>
```

Prints one direct, player-readable media URL on stdout. Nothing is downloaded.
These URLs are short-lived and host-bound: resolve, use, discard. Do not cache
one and expect it to work an hour later.

## `download`

```
youtube-helper-rs download <URL> [-o|--out-dir DIR] [-f|--format FORMAT]
```

Downloads the audio track and prints the path of the produced file on stdout.

- `--out-dir` defaults to `.` and is created if missing.
- `--format` is one of `wav` (default), `mp3`, `m4a`, `opus`, `flac`, `best`.
  `best` keeps whatever the source already serves and is the only value that
  does no re-encoding — the fastest, and the only one that cannot degrade the
  audio, at the cost of not knowing the extension in advance.

The path comes from `yt-dlp --print after_move:filepath`, so it is the file as
it exists after `yt-dlp`'s own post-processing, not a guess from a template.

## Output contract

- Success: exactly one line on **stdout**, exit code `0`.
- Failure: one line on **stderr** prefixed `youtube-helper-rs: `, exit code `1`.

That split is what makes the binary safe in a pipeline: `url=$(youtube-helper-rs
resolve "$page")` captures the URL and nothing else.

## Failures worth recognising

| Message contains | Meaning |
|---|---|
| `yt-dlp` not found / binary | The binary could not be resolved. Fix the machine, not the code. |
| `invalid or unsupported URL` | Rejected before invocation, or `yt-dlp` has no extractor for it. |
| Non-zero exit from the tool | Network failure, removed video, age or region restriction. |
| output file not found | `yt-dlp` reported success but the file is not there. |
