# Diff library comparison

Historical comparison before the switch to imara. `results.csv` retains those
measurements. The `custom` harness branch calls the live framework matcher, so
after that switch it measures the imara adapter; reproduce the original custom
engine only with framework revision 3665333. See Editio's docs/diffs.md for the
currently implemented policy.

Measured 2026-09-22 on Linux x86-64, Intel Core 5 320, Rust 1.98.1.
This is a standalone benchmark package: Editio/framework runtime dependencies
and behavior were not changed. Cargo.lock pins imara-diff 0.2.0 and similar 3.2.0.

## Recommendation

For Editio, prefer **similar 3.2, Myers, with an explicit deadline**, given our
responsiveness requirement. It is competitive on ordinary/sparse documents and
its deadline stopped the hardest tested input around 100 ms. Imara Histogram is
the stronger choice for moved/reversed blocks and lower memory use there, but one
repeated-pattern input consumed 314 ms and its public API has no cancellation or
deadline. Removing optional postprocessing did not improve that result.

Keep size limits, background scheduling, stale-result rejection and safe coarse
fallback regardless of library. Deadlines are cooperative: even similar's
Histogram overshot 100 ms to 175 ms here. These results do not establish a hard
wall-clock bound for Myers either. Avoid retaining a second custom matcher solely
as a fallback; a whole-region replacement is sufficient when giving up.

## Results

Median milliseconds, 31 measured runs after three warmups per engine/case:

| Case | Current custom | imara Histogram | similar Myers | similar Histogram |
|---|---:|---:|---:|---:|
| Sparse 74 KB / 2,000 lines | 0.67 | 0.24 | 0.42 | 0.62 |
| Sparse 1.38 MB / 35,000 lines | 5.68 | 3.16 | 1.72 | 1.80 |
| Identical 1.38 MB | 5.48 | 3.60 | 1.69 | 1.69 |
| Dense replacement, 150 KB | 1.10 | 0.63 | 1.01 | 0.99 |
| Fully different repeated lines, 665 KB | 2.44 | 1.31 | 2.33 | 2.28 |
| Ambiguous repeated pattern, 345 KB | 2.16* | 314.09 | 100.06† | 175.42† |
| Reversed 35,000 lines | 8.92 | 3.80 | 54.81 | 12.82 |
| Moved half of 35,000 lines | 10.34 | 3.76 | 34.49 | 11.49 |
| 1.60 MB / 100 long lines | 3.13 | 1.88 | 1.71 | 1.73 |
| Editor source, 98 KB, 28 changed blocks | 0.63 | 0.41 | 0.58 | 2.10 |
| Unicode and mixed line endings, 84 KB | 0.35 | 0.29 | 0.15 | 0.15 |
| Oversized 3.18 MB / 80,000 lines | disabled | 7.48 | 3.83 | 3.95 |

*Custom uses a whole-file coarse replacement on the ambiguous case: speed is not
equivalent output quality. Imara preserved substantially more equal lines there.
†Deadline 100 ms, approximate output allowed. Similar Histogram did more work past
the deadline. All returned scripts passed unchanged-range/reconstruction checks;
this proves validity on these inputs, not minimality or human readability.

Reversed-case first-diff process high-water RSS: custom 11.2 MiB, imara 6.7 MiB,
similar Myers 12.5 MiB, similar Histogram 13.9 MiB. Baseline was about 5.4 MiB.
Process RSS includes fixtures and runtime; it is not exact allocated bytes.
Sparse medium-file RSS instead favored similar (~6.5 MiB vs imara ~6.9 MiB).
See results.csv for p95, baseline/peak RSS, changed-line counts and raw-mode results.

## Method and reproduction

```sh
CARGO_TARGET_DIR=target cargo build --release --locked --manifest-path benchmarks/diff-libraries/Cargo.toml
target/release/editio-diff-benchmark imara medium
target/release/editio-diff-benchmark similar-myers ambiguous
```

Engines: `custom`, `imara`, `imara-raw`, `similar-myers`, `similar-histogram`.
Cases: `small`, `medium`, `identical`, `dense`, `repeated`, `ambiguous`, `reversed`,
`moved`, `long-lines`, `real-source`, `unicode`, `large`.

Release opt-level 3, thin LTO, sequential separate processes, no CPU affinity or
frequency control. A 30-second process watchdog enclosed each 34-run case; none
was killed. Timed region includes Unicode-aware line splitting/tokenization,
matching, output collection and temporary teardown. Imara includes its optional
line postprocessing; `imara-raw` omits it. Similar uses its normal captured-op
compaction. Both use identical terminator-preserving token boundaries. No file
I/O, verification, syntax highlighting, rendering or fixture generation is timed.

Memory sampled after the first call, before verification. Every result's ranges
are checked for bounds/order and matching unchanged spans, sufficient to
reconstruct the new input by substituting its changed ranges. RSS is Linux-only.
Full recomputation is measured, not the editor's cached incremental path or
imara's reusable token cache. Similar and custom receive 100 ms matching budgets;
imara cannot receive one. The oversized custom case is an intentional rejection,
not a speed result. Library limits were not applied for that exploratory case.

Mostly synthetic fixtures, including deliberate worst cases; `real-source` reads
the sibling framework editor source with mechanical edits. Measured framework
revision: 3665333. No Mac/Windows or slower-machine performance claim. Local CPU
variation and allocator reuse explain why these medians differ from earlier
single-shot measurements. Benchmarks are not a complete library correctness audit.

## Maintenance and adoption evidence

- **imara-diff:** used by [Helix](https://github.com/helix-editor/helix/blob/master/helix-vcs/Cargo.toml)
  and [gitoxide](https://github.com/GitoxideLabs/gitoxide/blob/main/gix-diff/Cargo.toml).
  [Release 0.2](https://github.com/pascalkuthe/imara-diff/releases/tag/v0.2.0)
  added Git-style hunk postprocessing. The repository has regression and fuzz
  testing; visible history includes late-2025 robustness/fuzzing work. A smaller,
  slower-moving project, but clearly deployed rather than an unproven experiment.
- **similar:** used by [Insta](https://github.com/mitsuhiko/insta/blob/master/insta/Cargo.toml)
  (currently a 2.x requirement: adoption of the library does not imply 3.x adoption).
  [Recent changelog](https://github.com/mitsuhiko/similar/blob/main/CHANGELOG.md)
  shows active 3.x algorithm/performance work and correctness fixes. Histogram was
  added in 3.0; 3.2 further improved its behavior. Earlier imara-vs-similar published
  benchmarks concern older similar implementations and do not settle this choice.

Both show meaningful maintenance and production use. Neither has a guarantee of
future maintenance or freedom from bugs. The fetched published versions, not
unreleased master changes, are the versions measured here.
