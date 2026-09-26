# User guide

See the [quick start](../README.md) for installation and basic usage.

## Controls

**Duplicate / save as new file** in Ctrl+K asks for a new path and switches to the
copy after a successful save. With unsaved edits, choose whether to save the
original first or take the edits only to the copy. Existing destinations are
protected; missing directories are created. Ctrl+U clears the suggested path.

External changes to the current file are detected automatically. Clean documents
reload with a toast, retaining nearby position/selection and refreshing Find.
With unsaved edits, a modal offers an independent unsaved copy, a copy compared
with the updated disk version, or discarding local edits and loading disk content.
Enter/Esc safely keeps a copy; saving it requires a new path. Comparison ends on
successful save. Deleted/unreadable/invalid files leave current text intact.

Use `--no-watch` or **Disable file change monitoring** in Ctrl+K to disable this
for the current session. Monitoring uses native notifications, not periodic
scanning. Untitled copies are not watched. Comparison references are snapshots;
only the active file is monitored. Filesystems that do not deliver native events
still have the existing save-time conflict protection.
For a never-saved path whose parent folders do not exist, monitoring starts after
the first successful save creates them.
Automatic reloads are capped at 64 MiB; larger external versions leave the current
buffer intact with a warning to reopen manually. This also bounds reads if a file
grows while being read. Cursor/selection relocation searches nearby content rather
than diffing whole documents; very large or no-longer-matching selections clear.

Opening a directory starts an untitled explanatory document with that directory
as the working directory. Replace the text and save with a relative filename.
Workspace browsing is not implemented yet.

Ctrl+K → **Abort editing** asks for confirmation, then exits with status `1`.
Cancel is the default. Abort discards unsaved changes but never reverses earlier
saves. Normal quit (including discarding unsaved changes) returns `0`; invalid
arguments return `2`. Interactive use requires terminal output and a keyboard terminal.
See [integration testing and registration plans](editor-integration.md).

Pipe text directly into an untitled document; the trailing `-` is optional:

```sh
printf '# Hello\n' | editio -v
printf '{"hello":42}\n' | editio -
some-command | editio --format markdown
```

Without piped input, `editio` still opens an empty untitled document. `--format`
accepts the same language names/aliases as Markdown fences (`md`, `rs`, `js`,
`c++`, `yml`, `csv`, `tsv`, `plaintext`, etc.) and overrides content/filename
detection for this session, including after saving. Recognized languages without
a bundled grammar use the same plain-text fallback as fenced code.

Save asks for a destination. Unchanged imported text is clean; edits trigger the
normal unsaved-changes dialog. Loading is cancellable with Esc and limited to
64 MiB; larger streams should be redirected to a file and opened normally.
Input must be valid UTF-8 text and follows existing file newline validation.
Piped input plus a separate file argument is rejected. For a file literally named
`-`, use `editio ./-`. This is input support, not an stdout filter: terminal output
must remain attached to a terminal. On Unix keyboard input uses `/dev/tty`; Windows
uses the attached console input. Windows/macOS execution remains untested.

| Action | Keys |
| --- | --- |
| Help | Ctrl+?; `?` in View; F1 fallback; click the bottom-right help label |
| View / file information | Ctrl+I; `i` in view mode; command palette fallback |
| Toggle line numbers | Search `line numbers` in the palette |
| Toggle word wrap (prose files) | Search `word wrap` in the palette |
| Toggle 120-column limit | Search `120-column` in the palette |
| Search command palette | Ctrl+K or Ctrl+Shift+P, then type immediately; F2 fallback; `:` in view mode |
| Switch view/edit | Ctrl+G; `g` in view mode |
| Save / quit | Ctrl+S / Ctrl+Q |
| Find / replace all | Ctrl+F (`f` or `/` in view mode) / Ctrl+H |
| Search options (inside Find) | Ctrl+K: unified search commands; Tab / Shift+Tab: cycle search modes |
| Next / previous match | Tab / Shift+Tab (document search) |
| Search results | Up/Down; Page Up/Down: first/last loaded hit; Enter or click: open result |
| Clear / leave search | Ctrl+U clears the Find field; Esc exits while retaining its state |
| Delete line prefix | Ctrl+U in Edit deletes from the logical line start to the cursor, preserving the suffix and newline |
| Select next occurrence | Ctrl+D, repeated to add selections |
| Undo / redo | Ctrl+Z / Ctrl+Y |
| Select / navigate | Shift+arrows, Ctrl+arrows, Home/End, Ctrl+Home/End, Page Up/Down |
| Copy / cut / paste | Ctrl+C / Ctrl+X / Ctrl+V; bracketed terminal paste |
| Close menu / release focus | Esc |

On macOS, forwarded **Cmd+C/X/V**, **Cmd+A**, **Cmd+S**, **Cmd+Z** and
**Cmd+Shift+Z** also work. **Cmd+Left/Right** go to the actual line's start/end,
including wrapped lines; **Cmd+Up/Down** go to the document boundaries.
**Option+Left/Right** navigate words; add Shift to extend a selection.
Home/End also use actual line boundaries for every cursor. Ctrl shortcuts remain
available when a terminal reserves Command keys; terminal paste remains supported.

**Ctrl+E** moves to the actual line end on every OS. On macOS, **Ctrl+A** moves
to the actual line start (on Linux/Windows it still selects all). This also handles
Ghostty's default Cmd+Left/Right translations. On Mac, use forwarded Cmd+A or
the palette to select all. Ghostty consumes Cmd+Up/Down for terminal prompt
navigation; use Ctrl+Home/End for document boundaries when forwarded.
Mode switching is **Ctrl+G**, or **g** in View; e, m and Enter are not mode aliases.

Wheel/trackpad scrolling locks to the gesture's first axis until a brief pause
(180 ms without wheel events), preventing diagonal drift. Keyboard navigation
remains independent.

Line shortcuts follow the [VS Code platform defaults](https://code.visualstudio.com/docs/reference/default-keybindings):

| Action | Linux | Windows | macOS |
| --- | --- | --- | --- |
| Copy lines up/down | Ctrl+Shift+Alt+↑/↓ | Shift+Alt+↑/↓ | Shift+Option+↑/↓ |
| Move current/selected lines up/down | Alt+↑/↓ | Alt+↑/↓ | Option+↑/↓ |
| Add cursor above/below | Shift+Alt+↑/↓ | Ctrl+Alt+↑/↓ | Cmd+Option+↑/↓ |
| Delete current/selected lines | Ctrl+Shift+K | Ctrl+Shift+K | Cmd+Shift+K |

Ctrl+Alt+↑/↓ also adds cursors on Linux/macOS, and Ctrl+Shift+K works on macOS when delivered by the terminal. Copy/delete acts on every cursor's touched lines, merges overlapping targets, and undoes as one action. Selections ending at the next line's beginning exclude that line. Copies preserve selections and place cursors on the copied lines. Repeated cursor addition retains the visual column across short lines, tabs, and Unicode text.

If a shortcut is intercepted or your terminal cannot distinguish Ctrl+Shift+K, search for the action in the command palette. The standalone app requests enhanced keyboard reporting on Unix terminals that support it; embedded hosts own that terminal configuration.

The palette exposes commands applicable to the current mode and file type, including cursor actions, fuzzy search, Go to line, Markdown/source toggle, and Save As. There is no quick-key submode. Scroll with arrows, Page Up/Down, or the wheel; Ratatui keeps the selection near the viewport center without adding blank rows. Commands can be clicked.

Switching view/edit keeps the visible text near its current screen row. Unchanged round trips restore exact scroll positions and cursors, so repeatedly pressing Ctrl+G does not accumulate drift. Markdown and tables use matching visible text to locate the source; diagrams and other content without a direct textual counterpart use an approximate position. Scrolling, editing, or changing layout establishes a new pair of positions.

Manually restoring the saved contents clears the modified indicator after background verification. For buffers up to 16 MiB, a 150 ms debounce precedes an exact, cancellable comparison against a shared saved snapshot. Different byte lengths need no scan; opening files and switching modes do not trigger scans. Larger buffers retain revision-based tracking (undo to the saved revision still works). Verification never changes undo history, and pending checks remain conservatively dirty. Line-ending/BOM changes also count. Hosts using `Buffer` without `Editor` can drive `dirty_check_pending()` and `poll_dirty()` themselves.

Saving a path that did not exist when opened asks for confirmation before creating it; Create is the cyan, initially focused action on the right. Narrow dialogs stack buttons with the default last. Unsaved changes on exit use a Save/Discard/Cancel dialog, with Save selected initially on the right, Discard in red and Cancel in the neutral color on the left. Untitled files use a compact Save As field that grows as the path wraps: enter relative or absolute paths, optionally quoted, or `~/…`; Ctrl+Left/Right moves between components, Up/Down moves between wrapped rows, and clicking positions the cursor. Path separators are muted. Enter saves; Esc cancels. Missing parent directories are created recursively on save. Permission failures keep the buffer open and show an error; the app never invokes sudo or requests administrator privileges.

Find defaults to case-insensitive matching. Its single-line footer adapts to available width and keeps a right-aligned Ctrl+K reminder for the palette of case, fuzzy, and wildcard toggles. Choosing a setting or dismissing that palette returns to Find with the query preserved. Find retains its query, options, input cursor, and result selection for the current editor session. Files up to 64 KiB search immediately; larger files use a 150 ms debounce and cancellable background scanning. A bounded list shows highlighted matches in context. Enter or click opens a result in the document, where Tab/Shift+Tab and Up/Down navigate hits and Page Up/Down select the first/last loaded hit. Continue past the last hit to load another batch (up to 1,000 hits); `+` means the total is not yet known. Previous batches can be revisited. Esc exits search mode; Ctrl+F reopens its saved state. The palette’s Next/Previous occurrence commands start the same flow for a word or selection. Search preserves the underlying view or edit mode: rendered previews search visible content and highlight it in place, excluding hidden Markdown comments and URLs. Use source view to search raw markup.

Click source text to place a cursor, drag to select, double-click a word, or Alt+click to add a cursor. In rendered previews, drag selects the visible characters and Ctrl+C copies them without changing mode or switching to source. Selections starting inside a table cell or diagram box stay within its interior. Markdown link labels hide their URLs: click to copy the URL, or right-click for a menu to copy text or URL (also operable with arrows/Enter/Esc). Terminal-native selection/copy commonly uses Shift+drag; exact behavior belongs to the terminal. Ctrl+K may be reserved by a terminal, input method, or custom multiplexer configuration; use Ctrl+Shift+P or F2. Additional modifier combinations depend on the terminal keyboard protocol.

## Included

Markdown previews support double-click word selection, including link text. H1 has centered thick rules and a magenta tint. H2 has a single leading rule and a cyan tint; H3–H6 use dim numbers. Titles are colored and underlined. Click heading text to copy its path and anchor; right-click for heading text, relative path, or absolute path. Duplicate headings receive distinct anchors. Tab/Shift+Tab focus preview controls, Enter activates them, and Shift+F10 opens a focused link's copy menu.

Quotes use a colored `│` border and normal text color, with dim styling throughout nested Markdown content. Formatting, syntax colors, tables, callouts, and expandable details remain available inside quotes. Wrapped prose repeats its border; copying omits borders and padding. Try [the headings and quotes example](../examples/markdown/22-headings-and-quotes.md).

Markdown edit mode shares the preview palette: magenta H1 and cyan lower headings, blue links, yellow inline code, colored callout markers, and dimmed quotes. Emphasis and fenced-code highlighting remain active; all Markdown punctuation, URLs, and fences stay visible and editable.

Source tables use cyan bold headers, muted pipes/alignment rows, and inline formatting inside cells, including reference links and escaped pipes. Document context also covers setext headings, multiline emphasis/comments, lists, footnotes, definitions, math, and nested code blocks. Try [the table styling example](../examples/markdown/23-table-source-styles.md). Parsed source styles are cached per revision for files up to 2 MiB; larger files retain lightweight line-based highlighting, and individual lines over 16 KiB skip syntax styling.

Word wrap is on by default for Markdown, `.txt`, `.text`, `.rst`, `.adoc`, `.asciidoc`, `.log`, and untitled/extensionless files. **Search `word wrap` in Ctrl+K** toggles it for both view and edit modes; the bottom bar shows `wrap:on` or `wrap:off`. Use `--no-wrap` to start with it off, or set `Editor.wrap` when embedding. Other source-code and CSV/TSV file types keep their existing layout.

Wrapping prefers word boundaries, follows indentation and list/task/quote prefixes, and limits continuation indentation to a third of the available width. Source tables and diagrams retain their geometry and horizontal scrolling. In source/edit mode, Up/Down and Page Up/Down follow visual rows; Home reaches the current visual row's beginning, while End reaches the actual source line's end, across any wrapped rows. Ctrl+Home/End retain document navigation. Shift extends the selection, including Shift+End for each cursor's source line. Mouse selection, copying, and multiple cursors use original source positions. Display-only indentation and wrap breaks are never copied; genuine source whitespace and line breaks remain intact. Try [the wrapping example](../examples/markdown/24-smart-wrapping.md) and resize the terminal.

Markdown code previews have a detected-language label and a muted neutral border flush with their container. The right-aligned `⧉ Copy` control copies the entire code block, preserving original whitespace, and stays visible when scrolling horizontally. Tab/Enter can activate it too. Fences share standalone syntax grammars, colors, and wrapping policy: prose wraps; programming languages retain their layout; Markdown samples preserve their internal section rules. Selection excludes the frame. Rendered diagrams and charts stay unframed. Try [the code layout examples](../examples/markdown/25-code-block-layout.md).

Ordered and unordered list markers share an accent color; task checkbox brackets are muted in preview and source. Long unbroken tokens fill the remaining row space before continuing, using the same prose width in view and edit modes.

Rendered Markdown tables use natural widths when they fit. With wrapping enabled, wider tables distribute space across columns and wrap cells independently. Short columns stay compact; long-content columns retain a 12–20 cell minimum based on their longest word. If the minima cannot fit, scroll horizontally. Copying reconstructs logical rows with tab-separated cells, excluding borders and alignment padding. Try [the responsive table examples](../examples/markdown/26-responsive-tables.md).

HTML comments are hidden except inside code. `<details>` sections are collapsible, respect the boolean `open` attribute, and retain their state through resizing. Click a summary or activate it with the keyboard. Callouts use colored thick borders and normal body text, retaining nested Markdown styling. Borders and table padding are excluded from selection/copying. Horizontal rules are muted and resize to the viewport. Try [the details/comments example](../examples/markdown/21-details-and-comments.md).

The document starts at the top of the viewport. A muted bottom bar shows mode, `top viewport line/total` in view mode or `line/total:column/max column` in edit mode, wrap state, and an ellipsized filename when space permits; only `Ctrl+?` sits at the right. Below 60 columns, the filename takes priority over the wrap indicator. Temporary messages appear as dismissible toasts for three seconds; save and exit confirmations use dialogs. The command center includes the filename and filters immediately as you type; only commands relevant to the current context are listed. Help lists main screen shortcuts first and brief app information last. Info provides the clickable full file path, statistics, view position, selection, search, and layout details. All dialogs have a separate Esc hint at the top right. Help and Info fit their wrapped content using Ratatui text measurement and centered layout, capped by the viewport; longer content scrolls. Info values use the same blue accent as its border. Ctrl+? opens Help (`?` also works in View). Help preserves an underlying input or confirmation and restores it on Esc; terminals that encode it as Backspace can use F1 or click the label.

Preview horizontal scrolling moves overflowing content while headings, prose, short code rows, and controls that fit remain stationary. Wide diagrams scroll as a unit to preserve their geometry. Selection and link actions follow each row's displayed position.

Line numbers are off by default. Toggle them with the command palette, `--line-numbers`, or `Editor.line_numbers` when embedding. Wrapped source continuations omit repeated numbers; rendered previews count rendered rows. The position indicator includes the total with numbers both on and off. Scrolling stops at the last full viewport, including after resizing.

The optional **120-column limit** is off by default. It forces text wrapping at 120 display cells or the available width, whichever is smaller, even when ordinary wrapping is off. It also applies to programming-language text and Markdown code blocks. Tables and rendered graphics retain their normal layout. Toggle it in the palette or start with `--limit-width`; embedded editors expose `Editor.limit_width`. [Example](../examples/markdown/27-column-limit.md).

The standalone app remembers line numbers, ordinary wrapping, and the column limit **per detected file type**. The theme choice and these preferences use one JSON file: `~/.config/editio/config.json` on Linux and macOS, and `%USERPROFILE%\.config\editio\config.json` on Windows. The earlier platform/XDG JSON file, or old editio TSV preferences and the shared theme choice, are imported once if the JSON file is absent; old files are left intact. Invalid or unsupported files are reported without overwriting them. Writes are bounded, atomic, and locked across instances. CLI layout flags override the session until a UI toggle persists that type. `Editor::new` performs no configuration I/O.

Ctrl+I opens Info (`i` in view mode); legacy terminals may encode Ctrl+I as Tab, so the command palette is also available. Help has neutral borders, Commands cyan, Find yellow, and Info blue. Status highlights use terminal ANSI blue for view, cyan for edit, and yellow for search, with inverse and dim attributes instead of fixed RGB backgrounds. View documents never show a hardware cursor; editable modal fields, including Find, show one; edit columns include the final insertion position. Narrow layouts omit secondary status details that remain available in Info.

Find settings persist for the editor session and apply to Tab/Shift+Tab navigation during document search. Exact matching finds literal substrings; fuzzy matching finds characters in order within a line. With wildcards enabled, `*` matches any number of characters and `?` matches one character, within a line; use `\*` or `\?` for literal symbols. Wildcards and fuzzy matching can be combined. Replace All and multi-cursor occurrence selection remain literal and case-sensitive.

CSV and TSV open as aligned tables in view mode. The first record is emphasized and pinned when it fits; cells wrap without losing their contents. Use arrows/Page Up/Page Down or the mouse wheel to scroll, and Left/Right or horizontal wheel scrolling to reach wide columns. The palette’s `Render / source` command toggles source; `g` enters source editing. Try `editio -v examples/tables/people.csv` or `examples/tables/inventory.tsv`. Quoted delimiters, doubled quotes, multiline fields, empty fields, ragged rows, CRLF, and Unicode are supported. Preview limits are 2 MiB input, 10,000 records, 64 columns, and 8 MiB formatted output; malformed or oversized tables show an explanation and keep the source available.

- Rope-backed Unicode text, multiple cursors/selections, undo/redo with shared snapshots and a 200-transaction history limit.
- UTF-8 files, optional BOM, LF or CRLF preservation, atomic saves, external-change checks, and unsaved-change protection. Save As protects existing paths. Mixed LF/CRLF and binary/non-UTF-8 files are rejected with an explanation.
- Syntax coloring for Syntect's bundled language set, using terminal ANSI colors and default backgrounds. No fixed application background or special icon font.
- Language-aware fenced-code highlighting in Markdown source and preview, including tilde fences, language aliases, and multiline code. Browse [the example files](../examples/README.md) for focused Markdown demonstrations and every bundled syntax type.
- Fence names use a pinned GitHub Linguist catalog and expanded two-face grammars. All 1,464 catalog names/aliases are recognized case-insensitively; 580 map to bundled grammars and 884 currently fall back to plain text. This is not complete GitHub highlighting parity. See the exact [language coverage report](language-support.tsv).
- Headings retain bold/color/underline styling. Code and Mermaid previews omit fence delimiters, and wide code/tables/diagrams scroll horizontally. The [Mermaid gallery](mermaid-coverage.md) has 28 supported diagram snapshots and eight explicit fallback cases.
- Markdown headings, lists/tasks, links, tables, quotes/alerts, footnotes, and code fences. Native Unicode Mermaid previews for flowcharts, sequence, class, ER, and XY diagrams through Merman; unsupported diagrams show source and an explanation.
- Background Markdown rendering, cached previews, viewport-only source drawing, and blocking idle input. Preview is limited to 2 MiB and individual diagrams to 200 lines/16 KB; source remains accessible. Long source lines skip syntax parsing above 16 KB.
- Native text clipboard access on supported desktop sessions, with an internal fallback when unavailable. No clipboard content is sent to a service.

## First-version limits

Full GitHub visual parity remains unfinished: HTML beyond comments/details and math remain readable source, images use text fallbacks, and diagrams use Unicode rather than terminal graphics. Outside Markdown fences, source syntax highlighting begins at the viewport and can miss multiline context. Markdown source scans preceding fences and parses the active code block to retain its multiline context. Mermaid support is limited to the families listed above.

Replace All is case-sensitive and literal. User-entered regex, full user keymaps, platform association installers, and incremental background search are future work. Moving a line requires one cursor. Very large searches and very long individual lines can still incur synchronous work. System clipboard availability and modifier keys depend on the host environment.

The design targets Linux/Omarchy, macOS, and Windows/PowerShell; only platforms actually exercised are listed in validation notes. The concise product goal remains in [docs/spec.md](spec.md), with TUI conventions from `~/dev/docs/tui-best-practices.md`.

Copy on selection is enabled by default: release a mouse selection to copy it,
including rendered Markdown/table text without decorative padding. Keyboard
selection requires explicit Ctrl+C and does not copy automatically. Toggle it in Ctrl+K (**Toggle copy on
selection**) or start with `--no-copy-on-selection`; Info shows the session setting.
Automatic copies are bounded to 64 KiB (rendered inspection work is also bounded);
oversized selections require explicit Copy and are never silently truncated.
No Foot/herdr configuration or special launcher is needed. This copies on selection,
not on the terminal's native Copy shortcut. Clipboard contents are not continually
synchronized, and successful automatic copies show the usual “Copied” toast.

Rendered Markdown, CSV and TSV tables share cyan headers and muted internal
separators, without outer borders. Headers stay visible while their table scrolls,
then leave with its last row. Wrapped cells cap at the viewport width, or 120 with
wrap:120; wide tables can still scroll horizontally. Adaptive row/column separators
can be cycled in Ctrl+K and are saved globally. See `examples/markdown/28-sticky-tables.md`
and `examples/tables/sticky.csv` / `sticky.tsv`.

`editio -v FILE` starts in View mode; `editio FILE` starts in Edit mode, and
`-V`/`--version` prints the version.
See [configuration and distribution notes](distribution.md).

Try [greedy table sizing](../examples/markdown/32-greedy-tables.md) and the matching
[CSV](../examples/tables/greedy.csv) / [TSV](../examples/tables/greedy.tsv) examples.
Columns stop at their natural content width; small overflow (up to
30% of the viewport width) is squeezed to fit. Larger overflow remains horizontally scrollable.
Wrapped cells never exceed the viewport width; the optional 120-column limit
further caps each cell. With both wrapping options off, natural widths remain.
View-mode Left/Right arrows scroll one terminal column per press.

Rename or move the current file with **Ctrl+K → Rename / move file** in View or
Edit. Relative destinations resolve from the working directory; missing parent
directories are created. Existing destinations are never replaced. The move keeps
unsaved edits and undo history in memory; Ctrl+S then saves to the new path.
Untitled files use Save As. Renaming a symbolic link is intentionally rejected.
The View position shows `percent%/total`. Progress is rounded down through the last visible row and reaches 100% at the bottom.

Delete a saved file with **Ctrl+K → Delete file permanently**. The confirmation
shows its path, defaults to Cancel and uses a red deletion action. This bypasses
trash. The text and undo history remain as an unsaved untitled buffer; closing it
still requires a save/discard decision. Changed-on-disk files and symlinks are
protected from deletion. There is no direct delete-file shortcut.

Recovery drafts checkpoint changed text after **30 seconds without changes**,
without overwriting the original. SIGTERM/SIGHUP and Windows console-close
notifications request a final checkpoint. Reopening a file offers its current
disk version (discard draft) or the recovered draft in comparison mode; saving
accepts the draft and ends that comparison. Plain `editio` also offers untitled
recovery. See [recovery behavior and limits](recovery.md).

Markdown navigation: **Ctrl+T** (or **t** in View) opens a filterable heading/link
navigator. **Tab** switches lists, **Enter** jumps, **Ctrl+Enter** follows an
internal heading link, and **Ctrl+K** exposes contextual actions including jump
back. Hidden sections open when needed. See [navigation](navigation.md) and
[the human-resources essay](../examples/markdown/35-ai-human-resources.md).

Inactive recovery drafts expire after 30 days. Cleanup runs only after startup
and background work settle, followed by two idle seconds; it pauses on activity.
See [recovery](recovery.md).

## Indentation and EditorConfig

New defaults use **tab characters**, displayed at **two-column tab stops**.
In Edit, Ctrl+] / Ctrl+[ indent/outdent whole current or selected lines (Cmd+] /
Cmd+[ on macOS). Tab advances to the next indentation stop, or indents selected
lines; Shift+Tab outdents. Legacy terminals may deliver Ctrl+[ as Esc; use
Shift+Tab or the command palette in that case.

Search **indentation** in Ctrl+K to choose spaces/tabs, set indentation size and
tab display width (1–32), or return to automatic settings. These changes affect
future edits and display, not existing text. **Remember as file type default**
saves the settings in the usual JSON preferences. Info shows the effective policy
and its source.

Precedence: explicit document override → matching EditorConfig properties →
file-type preference → tabs with size/width 2. Untitled and piped buffers use
preferences until named. Opening a named file, Save As/rename, and **Reload
EditorConfig indentation** resolve its configuration; there is no file watcher.

Only `indent_style`, `indent_size` (including `tab`), and `tab_width` are honored.
Parent inheritance, `root`, patterns, and `unset` use ec4rs. Other properties do
not transform files on save. Config reads are limited to 256 KiB per file / 1 MiB
total; invalid/unavailable configuration produces a warning and uses preferences.
Markdown parsing still uses standard Markdown indentation rules independently of
tab display width.

Conversion commands are available in **Edit mode** (Ctrl+G).
**Convert indentation to spaces/tabs/automatic settings** explicitly rewrites
leading whitespace throughout the document, preserving its current visual width.
Interior whitespace is untouched. Automatic conversion resolves without the
current document override. Conversion is one undoable operation; selecting a new
policy alone never converts text. Changing indentation size does not automatically
rescale existing nesting levels.

See [the indentation examples](../examples/indentation/README.md).

## Files, tags, and links

**Ctrl+Shift+F**, or **Find other files** in Ctrl+K, opens file search. Use its
Ctrl+K command view to choose a user tag, **Recently viewed**, **Recently modified**, or a
glob. Ctrl+F remains the current-document search; its menu can open file search.
Tab / Shift+Tab cycle **current file → recently viewed → recently modified →
tags → registered globs**, preserving the search query. Tags and registered globs
first show a filterable picker: Enter chooses a user tag or saved pattern and
starts searching there. Esc from those results returns to the picker. System
histories are separate scopes, not entries in the user-tag picker.
**Ctrl+L** switches filename/content search in any file scope; the command list
also offers **Search filenames** or **Search file contents**. This choice is
independent of scope cycling. Current-file Find always searches content.
Ctrl+K replaces the results with commands inside the same Find frame; Esc returns
to results. Commands name the state they will select, with relevant actions first.
Outside the dialog, Tab / Shift+Tab still navigate document matches.
Filename search matches filenames only.
 Content search uses the same case,
exact/fuzzy and wildcard options, with contextual snippets. Enter opens a result;
unsaved edits use the usual Save / Discard / Cancel dialog, including Save As for
untitled documents. Existing recovery drafts are offered normally.

Empty filename queries immediately populate tagged/history files. Results rank
by match quality, then newest activity: last viewed in Editio for Recently viewed,
last created/saved in Editio for Recently modified, and filesystem modification
time for tags/globs. Empty queries rank by recency; ties use full path, then line
number. Sorting happens before retaining the best 50 matches. Tag/glob pickers
rank filtered matches by quality; otherwise they use alphabetical order with
system globs first. The two system tags retain up to
256 files each; they record activity in Editio and cannot be manually removed.
User tags and remembered globs share the existing JSON preferences file.

```sh
editio --tag work notes.md
editio --tag work --tag ideas another.md
editio --untag ideas another.md
editio --list-tags
```

Relative globs (`*.md`, `**/*.md`, `notes/*.md`) resolve against the current file's
directory, or the working directory for an untitled document. Remembered globs
keep that meaning when changing documents. Absolute and home-relative patterns
such as `~/**/*.md` retain their own root. The glob picker always includes `*.md` and `**/*.md`, followed by saved user patterns (deduplicated). No filesystem index or idle scan runs.

Find can display twenty three-line results when the terminal is tall enough;
smaller terminals scroll. Search retains at most 50 results (shown as `50+` when capped). Filename searches
visit at most 200,000 entries; content searches visit at most 50,000. Each request
uses a cooperative 750 ms scan budget, and reads at most roughly 32 MiB of content.
Content search skips files above 2 MiB, binary/non-UTF-8 data and lines above
64 KiB; it searches raw text and returns the first match on each matching line.
Recursive globs do not follow directory symlinks and stop at 64 levels. A limited
result set asks you to refine the scope/filter and identifies an entry, time, or
content cutoff. Files in the scope root are checked before descending. Filesystem
calls can exceed the
cooperative budget on slow or unavailable mounts; search runs away from the UI.

Markdown link right-click menus offer full-URL inspection and opening in the
system's default application; local links can also open in Editio, including
heading anchors inside collapsed details. Ordinary click still copies the URL.
While authoring a relative Markdown destination, pause for suggestions, use
arrows to choose and Tab to complete. Completion reads only that directory;
absolute paths, URLs and anchor-only destinations are not suggested.

Try the [navigation examples](../examples/navigation/README.md) for a walkthrough.

Recent-file views reload the shared history whenever opened and do not apply Git-ignore rules.

Other file searches in Git repositories respect `.gitignore` (including parent and
nested rules), `.git/info/exclude`, and global Git excludes by default. Hidden
files remain eligible unless a rule excludes them. The open file's directory
is explicit: when it is already ignored, it remains searchable, but rules for
its descendants still apply. Normal Find within the open document is unaffected.
In Find's Ctrl+K commands, **Include Git-ignored files** disables filtering;
**Respect Git ignore rules** restores it. This setting is session-only and is
not saved in preferences. Tag/history searches use the same policy.

Matching uses the Rust [ignore library](https://docs.rs/ignore/latest/ignore/),
with lazy per-search work and no idle filesystem watcher or index.
