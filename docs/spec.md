# Text editor spec

A minimal, keyboard-first TUI for effortlessly viewing and editing text files. Built with Rust and Ratatui, usable as a standalone app or an embedded editor.

## Goals

- Rust only for the application, tests, benchmarks, and development tooling. No Python or shell scripts.

- Follow the TUI best practices in `~/dev/docs/tui-best-practices.md`: terminal-theme inheritance, visible focus/selection, Unicode display widths, responsive layouts and scrollable dialogs, modal mouse isolation, event-driven redraws, separated state/rendering/integration, and tested terminal cleanup.
- No filename header. Use a muted bottom bar for mode, top viewport line/total in view mode or line/total:column/max column in edit mode (regardless of line-number visibility), wrap state, optional ellipsized filename, and right-aligned Ctrl+? help. Full path/file details live in the command center and dedicated Info screen; F1 and mouse provide help fallbacks. Offer a line-number toggle in the command center and stop scrolling at the last full viewport.
- Remember line numbers, ordinary wrapping, and an optional 120-column cap per file type in user configuration. The cap defaults off, forces text wrapping at min(120, available width), and excludes visual renders; tables instead cap each cell at 120 while permitting whole-table overflow. Embedded hosts opt into persistence. Help emphasizes screen shortcuts and brief app information; Info contains file and current-view statistics rather than repeating its command list; every editor action has a palette entry.

- Fast startup, instant-feeling input, low CPU and memory use, and responsive resizing. Render only what is visible; avoid idle work and keep expensive highlighting, searching, and rendering off the input path.
- Support Linux (including Omarchy), macOS, and Windows terminals running PowerShell.
- Open files from the command line, optionally at a line/column; support `$EDITOR`/`$VISUAL`, Git editor use, and platform default-editor registration where available. Return meaningful exit codes and guard unsaved changes.
- Keep the interface small: text, a discreet status line, and contextual prompts. No IDE, plugin platform, project management, or built-in terminal.

## Modes

- Help and Info fit their wrapped content within the viewport using Ratatui measurement/layout; Info values use its blue accent. The main help label is just Ctrl+?. Narrow status bars prioritize the filename over the wrap indicator.
- Find defaults to case-insensitive matching. Its one-line footer drops detail at narrow widths while retaining the right-aligned command-palette shortcut. Ctrl+K in Find exposes only case, fuzzy, and wildcard toggles and returns to the preserved Find state after setting changes or dismissal.
- Use terminal ANSI colors and defaults: neutral Help, cyan Commands, yellow Find, blue Info. Dim inverse status highlights distinguish view (blue), edit (cyan), and search (yellow). View documents have no hardware cursor; editable modal fields retain their cursor. Keep Tab intact when a terminal cannot distinguish Ctrl+I; Info has view-mode `i` and palette fallbacks.
- **Ctrl+G** toggles view/edit when the document has focus; modal input and host handoff keys retain precedence. The command palette offers the same action.
- Preserve the visible document anchor and its screen row across view/edit changes. Remember exact paired viewports to prevent drift on unchanged round trips; use visible-text correspondence for rendered content and a proportional fallback when no counterpart exists.
- **View:** read-only navigation, selection, copying, search, and syntax highlighting. Markdown renders by default, with an easy source toggle.
- CSV/TSV view mode presents aligned, scrollable tables with wrapped cells, quoted-field parsing, and a pinned first record when space permits; source editing remains available.
- Share a word-wrap toggle between view/edit modes for Markdown and prose files (palette, `--no-wrap`). Wrap at words with bounded hanging indentation; oversized tokens fill the current row before continuing in both modes. Fenced code follows standalone language highlighting, wrapping, and section rules; show a language label, flush muted border, and viewport-aligned copy control, except for rendered graphics. Keep source tables and diagrams intact. Rendered tables wrap cells when needed, with readable column minima and horizontal scrolling when those cannot fit. Selection/copy excludes decoration, padding, and soft breaks while preserving logical code lines and table cells.
- Find has its own modal with live context previews, highlighted matches, a clear-input shortcut, and a footer for hit counts and case/strict-fuzzy/wildcard switches. Preserve query, options, and selected result across dismissal. Small files search immediately; large files debounce cancellable background work and load bounded result batches on demand, marking unknown totals. Enter opens a hit in document search mode: Tab/Shift+Tab and Up/Down navigate, Page Up/Down select first/last loaded hits, Esc exits without clearing state. Word-occurrence search uses the same flow. Search never changes the underlying mode or render/source setting; rendered views search and highlight visible text in place.
- Every dialog shows Esc separately at the top right. Info (Ctrl+I, or `i` in view mode) provides a clickable full path with muted separators and normal statistic labels with accented values. Use a muted highlighted bottom bar and a distinct search-mode accent.
- **Edit:** direct text editing, multiple cursors/selections, undo/redo, and saving. Preserve encoding and line endings where supported; clearly report unsupported files and save failures.
- **Command palette:** a filterable list of context-appropriate commands; no quick-key submode. Display current shortcuts and view-mode alternatives (`g` for mode switching, `f` for find).
- A searchable command palette exposes every command and its current binding. Mode switching and palette access must always be discoverable.

## Input and useful commands

- Use familiar VS Code bindings where the terminal can distinguish them safely: selection, copy/cut/paste, undo/redo, save, find/replace, word/line navigation, indent/outdent, toggle comment, duplicate line, and move line up/down.
- Support adding a cursor above/below, selecting the next matching occurrence, selecting all matches, and editing all selections together. Undo groups each multi-cursor edit as one action.
- Copy selected/current lines up or down and delete selected/current lines across all cursors, using VS Code platform shortcuts. Preserve cursor columns and selections on copied lines, exclude an endpoint at the next line's start, and group each action into one undo step. Linux defaults: Ctrl+Shift+Alt+↑/↓ to copy, Shift+Alt+↑/↓ to add cursors, and Ctrl+Shift+K to delete lines; retain palette fallbacks.
- Include literal find, optional regex and case matching, fuzzy find within the current file, next/previous occurrence of the current word, go to line, and bracket matching.
- Provide configurable bindings and conflict-aware presets for standalone use, terminals, tmux/herdr, and embedded hosts. Do not assume Ctrl/Alt/Shift combinations are delivered distinctly or override reserved OS/terminal/multiplexer shortcuts.
- Every command has a palette fallback. **Ctrl+K** opens the palette ready to filter immediately. **F2** and view-mode `:` are palette fallbacks. Use aligned command/shortcut columns, accented shortcuts, muted section labels, and symmetrically centered/padded modals. Allow remapping; avoid relying on Ctrl+P or Ctrl+Shift+P.
- Full mouse support: click to position the cursor, drag to select, double-click words, scrolling, and multi-cursor placement. Route coordinates and events correctly inside multiplexers and embedded panes; document terminal limitations and retain keyboard equivalents.

## Highlighting and Markdown

- Broad syntax highlighting for common programming, markup, configuration, and plain-text formats, detected by filename/content with a manual override.
- First-class GitHub-compatible Markdown viewing: headings, emphasis, lists, task lists, tables, links, images, blockquotes, alerts, footnotes, code fences with highlighting, math, and Mermaid diagrams. Track GitHub rendering coverage with representative fixtures, including supported HTML and extensions.
- Headings retain colored leading hashes, colored underlined text, plain separating spaces, and the muted underlined anchor. Below 60 viewport columns, hide the trailing anchor and its gap. Heading text and all heading markers share anchor-copy clicks and the heading context menu; double-click and drag selection remain available. Quotes have colored vertical borders and normal body color; dim all nested content while preserving its Markdown formatting.
- Render Mermaid diagrams in view mode, using terminal graphics where available and a readable text approximation otherwise; keep source accessible. Provide readable fallbacks for images, math, or layout a terminal cannot faithfully display.
- Keep navigation and text selection useful in rendered Markdown. Cache expensive rendering and handle unavailable renderers gracefully; never execute document code or scripts.
- Preview clicks never change mode. Select rendered characters within cell/box boundaries; Markdown links copy their hidden URL on click and offer text/URL copying on right-click. Distinguish heading levels visually; show Mermaid output without code fences.
- Double-click selects rendered words. Trailing heading anchors copy relative/absolute file paths or heading text. Hide comments, support collapsible details and default-open state, use colored thick callout borders, and exclude decorative borders/padding from selections. Rules span the viewport.
- Use one bottom status/help bar; action results appear as dismissible toasts and expire after three seconds, while confirmations remain until answered.
- Confirm new-file creation and unsaved exits in keyboard/mouse dialogs, with Esc at top right. Save As is a content-sized, wrapping path field with muted separators, native paths and home expansion. Create missing parent directories recursively on save. Permission failures preserve edits; privilege escalation is not implemented.
- Dialog defaults are initially focused and right-aligned; other actions stay left. Stack buttons with the default last on narrow screens. Safe actions use cyan, neutral actions the terminal default, reversible consequential actions yellow, and destructive actions red. Enter activates focus; Esc cancels. Create/Save are defaults, Cancel is neutral, and Discard is red.
- Only toast borders use Markdown callout colors: note blue, tip/success green, important magenta, warning yellow, and caution/error red. Bodies default to literal, normally colored text; callers can opt into lightweight Markdown. Respect monochrome mode.
- Recognize manual restoration of saved contents using debounced, cancellable background comparison of shared text snapshots, without scanning on opening or switching modes. Above 16 MiB retain revision-based dirty tracking. Pending verification remains conservatively dirty; saved encoding and line-ending changes also count.

## Ratatui embedding

- Ship a reusable editor library plus a thin standalone binary. Hosts can enable view and edit modes plus the command palette and supply buffers, configuration, themes, and commands.
- The host owns the terminal, event loop, layout, and focus. Expose rendering, event handling, buffer access, and outcomes such as handled, unhandled, changed, save requested, and focus released.
- Esc releases editor focus to the host. Hosts can configure additional handoff keys or predicates, such as Enter or Ctrl+Enter to submit a message, with explicit precedence over editing and menus.
- On handoff, return the trigger and current buffer state without inserting the triggering key. Allow hosts to resume editing with cursors, selections, and undo history intact.

## Acceptance

- Common viewing/editing tasks work entirely from the keyboard and with the mouse, standalone and embedded.
- Verify behavior on all target platforms, in tmux/herdr, and in a small Ratatui host demonstrating focus handoff and submission.
- Benchmark startup, input latency, idle CPU, and memory on representative small and large files; set measurable budgets before implementation. Large files and complex Markdown must not freeze interaction.

## First version

The runnable implementation and its verified coverage are described in `../README.md`. Full GitHub rendering parity, graphical diagrams/images, regex search, and exhaustive platform/multiplexer verification remain goals rather than implied guarantees.

Markdown view uses ○ bullets and single-cell ☐/☑ tasks in the shared list accent.
H1 titles center between muted horizontal rules without hashes; H2–H6 use one
leading rule character per level (at most one below 60 columns). H2 rules use
normal muted styling; H3–H6 dim them. H4–H6 omit trailing rules. Wrapped
continuations align with the first row's text and have no rules of their own.
Disclosure controls are muted neutral, with only their labels underlined.
Leading YAML frontmatter is a collapsed disclosure,
expanding into the normal YAML code renderer. Source editing preserves all markup.

JSON View pretty-prints by default with two-space indentation; the contextual
palette offers “Toggle JSON pretty printing” to show the original layout.
Formatting preserves source content, property order and literal values. Invalid
JSON falls back to its original syntax-highlighted layout.

Unsaved, unnamed buffers can preview recognizable JSON and Markdown using bounded,
revision-cached content detection. Plain text stays literal; named files retain
extension-based formatting. Previewing never assigns a filename or saves content.

Untitled detection also recognizes consistent CSV/TSV records using the shared
quote-aware table parser. Single records and inconsistent data remain literal.
- Rename/move is available in View and Edit through the palette, reusing the path input. Relative paths use cwd; create parents; never overwrite destinations. Preserve unsaved edits and undo history. Untitled buffers use Save As.
- View status shows `percent%/total`, rounded down through the last visible row, reaching 100% at the bottom. Wrapped-source progress uses visual rows; total retains its existing source/rendered meaning.
- Moving lines always moves all touched logical lines, even when only one character is selected.
- Delete file is a palette-only action for saved files in either mode. Confirm permanent deletion with Cancel selected by default. Preserve text and undo as dirty untitled content; reject external changes and symlink sources.

- Recovery drafts: 30-second edit debounce, bounded in-process worker, atomic checkpoints, final best-effort shutdown write. Recovery compares the draft against fresh disk contents; only successful explicit saving accepts it and ends comparison. Discarding or saving removes that session’s draft; inactive drafts expire after 30 days via throttled background cleanup only after startup/background work and two idle seconds. Active sessions are locked out of cleanup.
- Info paths: normal text for directory segments, muted separators, Info accent for the file name, including wrapped paths.

- Markdown navigator: Ctrl+T, or t in View; headings/links lists, fuzzy filter, cyclic arrows, Tab switches lists, Enter jumps, Ctrl+Enter follows internal heading links, Esc dismisses. Ctrl+K exposes contextual actions and jump-back history. Index only on demand, preserve View/Edit, reveal containing disclosures, and bound parser/output work.
