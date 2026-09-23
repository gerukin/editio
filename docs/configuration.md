# Configuration

Editio configuration and preferences use one JSON
file; the theme palette is compiled into the app and only its choice is stored.

| Platform | Configuration file |
| --- | --- |
| Linux/Omarchy | `~/.config/editio/config.json` |
| macOS | `~/.config/editio/config.json` |
| Windows | `%USERPROFILE%\.config\editio\config.json` |

```json
{
  "version": 1,
  "theme": "terminal",
  "file_types": {
    "markdown": { "wrap": true, "line_numbers": false, "limit_120": false }
  }
}
```

Theme choices: `terminal` (default) and `tokyo-night-omarchy` (bundled palette).
JSON is parsed only when loading/settings change, not every frame. Writes merge
only the changed theme or file-type settings under a lock and atomic replacement.
Unknown keys, versions and invalid data are rejected without overwriting the file.

The application reads and writes only this file. No config file is created merely
by opening with defaults. No system configuration is changed during development
checks.

New writes always use the user home directory, ignoring APPDATA and XDG redirects.
Rust's standard home resolver uses HOME or the account database on Unix and
USERPROFILE or the Windows user-profile API on Windows. Missing/relative home
paths produce a visible error, never a write relative to the current directory.
An inaccessible config is reported without silently discarding it.

Table separators are global (not per file type):

```json
"tables": { "rows": "adaptive", "columns": "adaptive" }
```

Place that member alongside `theme` and `file_types`. Values are `adaptive`,
`always`, or `never`; both default to adaptive. Ctrl+K offers **Table row
separators** and **Table column separators** to cycle them. Info shows the choices.
Rows appear at 24 document rows; columns appear at 60 document columns. The header
underline is unchanged. All rendered Markdown, CSV and TSV tables share these rules.

Recovery snapshots use `~/.local/state/editio/recovery/` on all supported systems,
under the native user home. The debounce is currently fixed at 30 seconds.
See [recovery behavior](recovery.md); no original-file auto-save is performed.
