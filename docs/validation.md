# Validation

For 0.1.1, the user accepted testing of the file monitoring and duplication update
on the personal ARM64 Mac and x86-64 Surface after local Linux validation.
The work Mac was unavailable for this update. Cross-compilation alone does not
validate the other architectures at runtime.

Editio is validated from this checkout with its private sibling `../tapp-ui`.

## Shift mouse selection and Mac cursor shortcuts (2026-09-30)

Local Linux x86_64: all 290 app tests passed (seven intentional ignores), along
with formatting, strict app/framework Clippy, all-feature framework tests and
the framework no-default-features check. The native optimized executable was
rebuilt and passed all three terminal lifecycle/selection PTY tests; the existing
local executable symlink points to that build.

The new PTY case extends then shrinks a selection using Shift mouse reports,
replaces and saves the selected text, and checks XTSHIFTESCAPE startup/cleanup
ordering. Offset TestBackend tests cover source View/Edit, wrapping enabled and
disabled, wide Unicode, tabs, reversed selections, continued dragging and copy
on release. Preview mouse release does not activate a Shift-selected link.
Ctrl+Option cursor addition retains text and edits every cursor together.
Framework commit: `dd8b0d8`.

Ghostty's Mac defaults reserve Cmd+Option+Up/Down for terminal split navigation.
The existing Ctrl+Option fallback is now visible in the Mac command labels.
The framework's Mac key-normalization unit test runs on Linux; the added
native-Mac-only editor dispatch test was not run. Actual macOS/Windows terminal
gestures and multiplexer forwarding remain unverified.

The managed environment injects `/tmp/.git`, which breaks the existing test for
search outside a repository even in escalated execution. The successful full
suite used `TMPDIR=/var/tmp` outside the sandbox, without changing that test.
Shared implementation and evidence:
[`2026-09-30-shift-mouse-and-mac-cursor-shortcuts.md`](../../tapp-ui/changes/2026-09-30-shift-mouse-and-mac-cursor-shortcuts.md).

## Duplication and external changes (2026-09-27)

Local Linux x86_64: 287 app tests and 204 framework tests passed, plus formatting,
strict Clippy, the framework minimal-feature check and native optimized build.
Framework main: `76c1d74` (transactional save/reload support starts at `df3d98d`).
Tests cover failure/cancellation without changing the original association,
external writes and atomic replacement, symlink retargeting, watcher teardown,
conflict dialogs, bounded reads, retained selections/search and recovery/exit.

The optimized executable's real-PTY idle test sampled small Markdown and a
3,750,000-byte source file in clean Edit, dirty Edit awaiting debounce, and View.
All six two-second windows measured **0 CPU ticks, 0 voluntary thread context
switches, and 0 terminal output bytes**, including native monitoring. This is a
local measurement, not a guarantee for every filesystem or machine.

```sh
EDITIO_TEST_BINARY="$PWD/target/release/editio" cargo test --locked --test recovery_pty idle_clean_dirty_and_large_documents_sleep_without_polling -- --nocapture
```

Native macOS/Windows watcher delivery has not been tested in this change.
Manual walkthrough: [file changes example](../examples/file-changes/README.md).

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
cargo test --test pty
cargo test --test clipboard_pty
```

The PTY tests exercise terminal setup/restore and clipboard dispatch on Unix.
They do not establish macOS, Windows, terminal-multiplexer, or light-theme
runtime coverage.

## Content-sized document tables — 2026-09-21

Shared framework `bea4e37` keeps Markdown/CSV/TSV columns at natural content
widths, with viewport/120 caps and the existing 30% squeeze allowance. Framework
and Editio regression suites passed, including two- and four-column cases across
wrapping modes. Shared strict Clippy, format and minimal-feature checks passed.

## Recovery checkpoints (2026-09-23)

Linux release measurement on this machine, temporary directory on the workspace's
Btrfs filesystem, including checksum, buffered write, file sync, atomic rename
and directory sync (one warm run per size):

| Draft | 10,000 Rope snapshot clones | Checkpoint | Process RSS before → after write |
| --- | --- | --- | --- |
| 2 MiB | 80.7 µs | 5.73 ms | 6,928 → 6,936 KiB |
| 16 MiB | 80.7 µs | 22.55 ms | 23,324 → 23,332 KiB |

These are local samples, not guarantees for other storage or machines. RSS is
whole-process resident memory sampled around writing, not peak memory, worker
stack commitment, or the cost of retained chunks during concurrent editing.
The manual measurement is `recovery::measurements::checkpoint_cost` in the release
binary test harness. The snapshot writer does not flatten the Rope.

## Markdown navigation and idle retention (2026-09-23)

Linux local release `cargo run --release --example measure_navigation`:

| Input | Entries | Index time |
| --- | ---: | ---: |
| 8,654-byte example essay | 41 | 1.49 ms |
| 87,780 bytes, headings and links | 4,000 | 2.35 ms |
| 80,000 bytes, repeated heading text | 10,000 | 5.43 ms |
| 2,000,009 bytes, mostly prose | 1 | 26.81 ms |
| 2 MiB + 1 byte | refused | 0.45 µs |

The sequential measurement process reached about 22,264 KiB reported peak RSS,
including all input fixtures, allocator and parser allocations; this is not
isolated retained index memory. The mostly-prose case moved the reported process
peak from about 12,444 to 22,264 KiB. Parser preparation is off the UI thread;
these are local CPU measurements, not guaranteed latency on other machines.

The framework's ignored release test `repeated_parent_filter_cost` measured
100 filters over 10,000 entries in 11.92 ms (about 0.12 ms/query), with 80,000 bytes
of reusable parent-match scratch. It demonstrates bounded repeated-parent work,
not terminal drawing speed. No startup index or per-frame source parsing exists.

Retention tests cover old/recent timestamps, recent file modification times,
active locks, and no cleanup worker during busy startup. Existing recovery PTY
checks continue to cover real terminal closure and the actual 30-second debounce.
A real-terminal navigator check jumps to a hidden heading and verifies that the
file remains unchanged. Native Windows/macOS runtime validation remains pending.

## Idle CPU and terminal loss — 2026-09-23

Two live orphaned instances used the 07:51 executable (unlinked after rebuild),
with deleted PTY descriptors and roughly 96% CPU each. Its copied executable
failed the real terminal-close/exit test. The newer mio reader already returned
terminal hangup errors; the same protection was added to the alternative
`use-dev-tty` reader. The two old instances were suspended, not killed, to stop CPU
consumption while retaining possible unsaved text in RAM.

App scheduling now uses explicit wake delivery and actual deadlines, removing the
100 ms recovery poll. Linux PTY samples in the debug build measured **0 CPU ticks,
0 voluntary thread context switches, 0 terminal-output bytes over two seconds**
for clean and dirty Edit, View, a 3,750,000-byte/250,000-line Rust file, and a draft
already checkpointed after the real 30-second debounce. Draft modification time
remained unchanged. These bounded observations are not an absolute zero-cost
claim: startup, rendering, real input and checkpoint I/O still require work.

Run `cargo test --test recovery_pty -- --nocapture` for assertions and counters.
These OS-signal tests require an environment that permits signal-hook's wake
socket operations; this session's restricted sandbox suppressed delivery through
that path, so lifecycle validation ran outside it using isolated temporary homes.
Windows/macOS runtime behavior remains untested.

The optimized release repeats the same zero-tick/zero-wakeup/zero-output samples.
Full app and framework suites, Clippy and the minimal framework build pass.
The PTY harness serializes openpty/CLOEXEC/spawn to prevent another concurrently
starting test from inheriting its master descriptor and masking terminal loss.
Both default and `crossterm/use-dev-tty` builds pass real master-closure recovery.

## Bounded process shutdown — 2026-09-23

After the user closed their sessions, four old processes remained. All four were
explicitly terminated at the user's request, and the process inventory was empty.
The app now arms a five-second hard-exit watchdog only during termination, ahead
of checkpoint/terminal cleanup; it adds no idle timer. The normal three-second
recovery finish budget is retained. The watchdog bypasses destructors/stdio if
shutdown stalls; it cannot override kernel scheduling or uninterruptible I/O.

A subprocess regression parks its main thread forever and verifies termination
by the actual five-second production watchdog. Real PTY scenarios cover closure
in Edit/View, command/find/info/help dialogs, pending dirty Markdown rendering and
the recovery chooser, plus eight consecutive clean quits. Cancelled quit remains
alive beyond the watchdog interval. Existing save/discard/abort and recovery tests
continue to exercise file preservation and exit statuses. Linux runtime coverage;
Windows/macOS still require platform validation.

## Optional framework capabilities — 2026-09-23

Editio explicitly enables the rich editor, Mermaid, diff engine, terminal session
and palette query. The framework's memory buffer, storage, dirty tracking,
text-search UI and individual renderer dependencies are now separable. Recovery
remains app-owned; configuration formats/paths and terminal theme defaults did
not change. See the sibling framework's `docs/features.md` and
`changes/2026-09-23-capability-isolation.md` for migration and measurements.

Validation: strict Clippy and format checks; 219 application tests passed (three
intentional measurement/helper/doc ignores), 190 framework tests passed, minimal
feature tests and 15 isolated consumer builds passed. The rebuilt release binary
also passed all 13 recovery PTY tests, including actual 30-second debounce,
terminal loss, cancelled quit, and sleeping idle clean/dirty/large documents.
Linux only; no macOS/Windows runtime claim. Release size: 13,883,280 to 13,588,816
bytes; local `~/.local/bin/editio` still points to the rebuilt release executable.

## File navigation (local Linux x86-64)

The file finder reuses shared search/dialog primitives and a lazy latest-request
worker. Tests cover bounded searches, cancellation, global preference
preservation, tag/history handling, relative completion, pasted query isolation,
missing targets, and narrow/offset modal rendering. The Rust PTY workflow tags a
fixture without launching the UI, opens empty-query tag results, cancels an
unsaved switch, then saves and switches successfully, verifying disk content and
history. Existing recovery/save/rendering tests remain enabled.

An optimized isolated test with 10,000 small Markdown files measured 15.1 ms for
filename matching and 33.7 ms for content matching, keeping 256 hits. Process
VmHWM was 6,440 KiB (whole test process, not incremental allocation). This is a
local warm-filesystem measurement, not a latency guarantee for remote mounts or
large files. Reproduce with `cargo test --release --bin editio
files::scale_tests::measure_file_search -- --ignored --nocapture`.

No macOS/Windows deployment or execution was performed for these features.

### Recursive filename cutoff regression

The reported `.local/app-name-research.md` case was reproduced read-only:
`**/*.md` stopped at seven matches because its tooling subtree exhausted the
old 50,000-entry budget in 120 ms. An independent enumeration found nine Markdown
files among roughly 90,000 files. The revised filename budget and streaming
root-first traversal returned all nine without truncation in 310 ms (debug build,
local warm filesystem). No directory sorting or whole-tree index is needed.

A synthetic regression with 50,100 nonmatching files verifies root and nested
Markdown matches without truncation. Run explicitly with `cargo test --bin
editio filename_search_survives_a_large_nonmatching_tool_tree -- --ignored`.

### Git-aware search

Tests cover ignore rules inherited while starting below the repository root,
nested overrides/negation, `.git/info/exclude`, filename and content modes,
tagged paths, the session-only toggle, and explicit searches starting inside an
ignored directory. Git filtering is disabled outside a repository. The latter
test runs outside the sandbox, whose synthetic `/tmp/.git` marker otherwise
makes temporary directories appear to belong to a repository.

The optimized `.local/**/` reproduction with Git filtering enabled returns all
nine Markdown files without truncation in 235 ms on this machine. Ignored
subdirectories are pruned by the traversal engine; individual tag/history paths
use per-request cached matchers. No index, watcher or extra worker is introduced.
