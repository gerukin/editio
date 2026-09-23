# Editor integration

## Current behavior

Editio starts in Edit mode and waits until the session ends. No `--wait` is needed.
Normal quit, including discard-and-quit, returns 0. Command palette **Abort editing**
always asks for confirmation and returns 1, even for unchanged documents. Cancel
is selected by default; `a` confirms abort. Earlier saves remain on disk. The
caller decides whether failure cancels, pauses or rejects its operation.
Fatal application errors return 1; invalid CLI arguments return 2.
Terminal cleanup runs before returning the exit status.

A directory argument creates a clean, untitled explanatory document and changes
only Editio's process working directory. Save accepts relative names and creates
missing parent directories. The parent shell's directory does not change.

Stdout must be a terminal. Redirected output fails before terminal setup, without
escape output. Help/version remain usable with redirection. Redirected stdin is
imported into an untitled document when no file or `-` is supplied. Keyboard input
uses Crossterm's controlling terminal (`/dev/tty` or `CONIN$`). Pipe loading runs
off-thread, is cancellable, and accepts up to 64 MiB. It waits for EOF before
editing; no partial input is silently accepted. Format overrides share fence
aliases. See README for examples and validation limits. Output filtering remains
future work. This is not a claim of compatibility with every editor-launching tool.

## Manual synthetic caller

From the repository, run:

```sh
./target/release/examples/editor-client ./target/release/editio
```

Both executables are built locally. To rebuild the harness yourself, use
`cargo build --release --example editor-client` (Windows adds `.exe`).

Each run creates a disposable draft, waits for Editio, and prints whether the
synthetic client accepted or cancelled it. Nothing is submitted or configured.
Repeat with these scenarios:

1. Ctrl+Q without edits: accepted, status 0.
2. Edit, Ctrl+S, Ctrl+Q: accepted with saved text, status 0.
3. Edit, Ctrl+K, type `Abort editing`, Enter, `a`: cancelled, status 1;
   original draft remains on disk.
4. Save an edit, make another edit, then abort: cancelled; first save remains.
5. Open Abort editing, press Enter: Cancel is selected; editing continues.
6. Edit, Ctrl+Q, `d`: accepted with the last saved text, status 0.

For directory behavior, run `editio /path/to/a/test-directory`, replace the notice,
save as `nested/note.md`, and verify it appears under that directory.

The harness is Rust and accepts an executable path as a single argument. It uses
inherited terminal handles and portable process status APIs; its temporary draft
is removed when the harness exits. Automated Unix PTY tests cover statuses,
unsaved-data preservation, terminal restoration and relative directory saves.
The lifecycle/clipboard suites provide additional terminal coverage. macOS and
Windows execution is deferred; neither is claimed tested.

## Omarchy research — 2026-09-21

No editor defaults, environment variables, desktop associations or Omarchy files
were changed. The installed selection remains `zeditor`.

The installed launcher and upstream quattro use
`~/.local/state/omarchy/defaults/editor`. The default selector has a fixed editor
list; the launcher separately recognizes TUI executable names and otherwise uses
its GUI branch. Editio is absent from both. Writing just its name into state is
insufficient. Older master selector snapshots still modify `uwsm/default`; avoid
applying those historical instructions to this installation.

Evidence:

- [Current selector](https://github.com/omacom/omarchy/blob/quattro/bin/omarchy-default-editor)
  and [launcher](https://github.com/omacom/omarchy/blob/quattro/bin/omarchy-launch-editor).
- [Helix issue #1739](https://github.com/basecamp/omarchy/issues/1739) and
  [PR #1740](https://github.com/basecamp/omarchy/pull/1740) document the same missing
  TUI classification problem.
- [Fresh issue #4014](https://github.com/basecamp/omarchy/issues/4014) proposes a
  configurable list or other alternatives. These are suggestions, not an adopted
  roadmap; inspected launcher code still uses the explicit list.
- [Discussion #6023](https://github.com/basecamp/omarchy/discussions/6023) discusses
  access to environment configuration after the Defaults UI changes; it does not
  establish a custom-editor registration API.
- [Third-party setup.defaults](https://github.com/nightdevil00/setup.defaults)
  discovers additional applications and writes Omarchy's state. Its advertised
  registration alone does not resolve the installed launcher's TUI allowlist.
  It was neither installed nor treated as an official integration mechanism.

Preferred future integration: contribute support through Omarchy development,
covering selector, TUI classification and discoverability. A general registration
mechanism should record executable and terminal-vs-GUI intent explicitly, preserve
arguments and exit status for `--inline`, and use Omarchy's selected terminal for
desktop launches. This is our proposal, not a confirmed upstream plan.
Recheck upstream before implementation. Do not patch `/usr/share/omarchy`, shadow
its launcher or override EDITOR/VISUAL to bypass defaults. After upstream support,
select Editio through `omarchy default editor`; test inline waiting/cancellation,
desktop launches and `$EDITOR .`. File associations are a separate opt-in step.

## Platform plan and remaining validation

- Other Linux: ordinary EDITOR/VISUAL integration plus an optional terminal desktop
  entry and MIME registration. Preserve argument boundaries; use `--` before paths.
- macOS: same process/exit contract; test native terminals and signals later. Finder
  integration needs a small app launcher and user-selected Open With associations.
- Windows: same process/exit contract with inherited console handles; validate
  PowerShell, Windows Terminal, paths/Unicode, ConPTY and file locking later.
  Desktop registration needs a launcher and user-selected Default Apps associations.
- No terminal-spawning wrapper in synchronous CLI calls. Desktop launching is a
  separate path. No automatic rollback of files saved before abort.
- Still to test: real Git/gh workflows, broken terminal
  connections, signals/suspend, permission/ACL/link edge cases and physical terminal
  behavior. No cross-platform or comprehensive signal-cleanup claim is made.
