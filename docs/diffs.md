# Diffs

```sh
editio --diff original.rs modified.rs
editio -v changes.patch
git diff | editio -v --format diff
```

Comparisons start in View. The first file is a read-only snapshot; only the second
is edited or saved. Both arguments must be distinct existing regular files.
No patch application, file creation from patch paths, or external process occurs.
Ordinary save conflict and permission checks still apply.

- **Ctrl+P / Ctrl+N:** previous/next change, cyclically, in either document mode.
- **[ / ]:** reference/current side, in View for two-file comparisons only.
- **Ctrl+G:** switch View/Edit; Edit always addresses the second file.
- **Ctrl+K:** discover these commands and **Refresh diff**.

Ctrl+Up/Down retain their existing behavior. Modal inputs keep their own bindings.
Syntax colors remain normal; a thick right edge marks additions (green), removals
(red), and replacements (blue), consistently in Edit and View. Wrapped fragments retain that edge.
Edit preserves source, coloring actual patch signs without tinting entire rows.

Patch View shows current-side fragments with signs removed. File and `@@` hunk
headers remain visible: their ranges identify omitted context between fragments.
No complete previous file is available. Malformed/unsupported hunks remain literal.
Edit preserves the raw patch. Two-file View displays syntax-highlighted source,
including for Markdown. Try [the examples](../examples/diff/README.md).

Conservative detection recognizes adjacent removed/added runs in the first 4,096
characters of ordinary files, excluding list-style `- ` / `+ ` signs. These files
retain their grammar. Explicit `.diff`/`.patch` paths and `--format diff` identify
patch documents. Detection skips fenced code, so a Markdown diff example never
starts whole-document diff mode. Detection is a heuristic, not a language parser.

## Markdown fences

Use `diff`, `patch`, or `udiff` for a supplied patch, and `diff-rust`, `diff-js`,
etc. to retain syntax colors for an underlying language. Both removed and added
lines stay visible. Only their signs get diff colors; Edit and View share the same syntax highlighting;
wrapping, selection and the Copy button retain original text. Ordinary fences
receive no diff metadata or matching work. Shiki comment annotations are not
interpreted. See [the example](../examples/markdown/33-diff-fences.md).

Fences reuse the existing code renderer and cached layout. They create no
comparison snapshot, token interner, matcher or background job per block.

## Performance policy

Per file: **2 MiB and 50,000 lines** maximum. Larger comparisons open the target
normally with a warning; the CLI avoids loading an oversized reference. Small
files (**128 KiB and 4,000 lines** maximum) update after a 150 ms edit debounce.
Medium files update on entering View, navigating changes, or Refresh diff.

One matching worker per editor, no queued revisions. Stale results are rejected
and old annotations disappear on edits. Cached comparisons reuse unaffected
ranges and rematch an edit window; prefix/suffix discovery remains linear off the
event thread. **imara-diff 0.2 Histogram** handles line matching, followed by its
line postprocessing. There is no second in-house matching algorithm.

After removing equal prefix/suffix lines, a region containing more than 8,000
lines across both sides and fewer than one distinct line per 64 lines becomes a
coarse replacement with a toast. The cheap linear token check avoids the measured
pathological repeated-pattern case while preserving unchanged edges.

Matching has a **100 ms** acceptance budget, checked during tokenization and around
the library calls. **Imara itself cannot be interrupted**; an over-budget result
is discarded after it returns. Size/repetition caps are heuristics, not a strict
worst-case latency guarantee. Preview/syntax preparation has a separate
**200 ms** guard. Exceeding a guard disables annotations with a warning and retains
normal viewing/editing. Refresh diff retries. Deadlines are cooperative, not hard
preemption of individual syntax-library calls. Fallback never changes file content.

Ordinary files have no diff worker or periodic wakeup. Detection samples at most
4,096 characters once per revision, then caches the flag; oversized files skip
the sample. This bounded cost is included below rather than called literally zero.

## Current implementation measurements

Single-shot release measurements on the same machine: 74 KB sparse matching
0.28 ms, incremental update 0.27 ms; 1.38 MB sparse matching 6.07 ms, update 4.07 ms.
The 345 KB/35,000-line repeated-pattern case coarsens in 1.63 ms, avoiding the
314 ms unguarded imara result. Medium syntax previews still reach their separate
budget; changing the matcher does not remove that cost.

`cargo run --release --example measure_diff_fences -- diff-text` measures 1,000
explicit fences: preparation median 16.71 ms versus 14.86 ms for ordinary `text`
fences. Cached 1,000-frame drawing was comparable (28/33 ms). No matching work is
done for either. Total stripped executable growth for both features: 37.3 KiB.

## Original implementation measurements (before imara)

Linux release build, Intel Core 5 320, 2026-09-22. Reproduce with
`cargo run --release --example measure_diff -- small` (also `medium`, `dense`,
`repeated`, `large`, `plain`). Individual local measurements, not cross-machine
guarantees; guards leave slower systems a safe exit.

| Input | Initial match | Incremental update | Syntax/preview | Peak process RSS |
|---|---:|---:|---:|---:|
| 74 KB, 2,000 lines, sparse | 0.86 ms | 0.23 ms | 53 ms | 12.8 MiB |
| 1.38 MB, 35,000 lines, sparse | 13.1 ms | 4.15 ms | guarded at 201 ms | 21.9 MiB |
| 150 KB, dense replacement | 2.33 ms, coarse | 0.34 ms | 71 ms | 15.7 MiB |
| 665 KB, repeated replacement | 4.56 ms, coarse | 2.57 ms | guarded at 201 ms | 17.2 MiB |
| 3.18 MB, 80,000 lines | rejected in 6 µs | — | — | fixture included |

Ordinary document: 2,000 cached detection calls took 7.4 µs; 200 edit/detect/undo
cycles took 2.26 ms including buffer work. Two thousand ordinary 80×24 draws took
168 ms with no worker. Peak RSS includes fixtures/runtime/syntax assets; it is not
an allocation delta or a pre-feature comparison.

Tests cover preservation, target-only saving, Unicode lines, malformed hunks,
wrapping, styles, cyclic navigation, incremental reconstruction/undo, mode
transitions, limits, and Linux PTY callers. Physical macOS/Windows tests remain pending.
