# Markdown navigator

Open with **Ctrl+T**, or **t** in View; also available through Ctrl+K.
It preserves View/Edit and does not enter document Find mode.

- **Headings** starts at the current section, indented by level (compact H1–H6
  labels on narrow screens). Type a case-insensitive fuzzy filter. Parent-heading
  context participates in matching and appears beside results when space permits.
- **Tab / Shift+Tab** switches between Headings and Links.
- **Up/Down** cycles results; Page Up/Down moves by a page. **Enter** or clicking a
  result jumps to its source location and closes the navigator. Arrows do not
  move the underlying document. **Esc** dismisses without moving it.
- In Links, **Ctrl+Enter** follows a `#heading-anchor` within this document.
  **Ctrl+K** exposes switching lists, clearing the filter, following an internal
  link, copying a URL, and returning to the previous location. External URLs are
  copied explicitly; the navigator does not fetch URLs or launch a browser.
- **Ctrl+U** clears the query. Query/list state survives closing. Jump-back history
  holds at most 32 source locations and ignores locations from earlier edits.
- Jumps reveal enclosing collapsed details, including nested details. Other
  disclosures retain their state. Comments, frontmatter and fenced code are not
  interpreted as document headings/links. Duplicate heading anchors use the
  same generator as the renderer.

Try `editio -v examples/markdown/35-ai-human-resources.md`. Search `puddle`, jump
into the nested disclosure, follow `human-oversight-1`, switch to Links, then use
Back to previous location through the command palette.

## Costs and limits

There is no startup index and no background navigation work until first opened.
A single worker builds a source-offset index from a shared Rope snapshot. Results
are cached by revision; stale results are rejected. Reopening or resizing does
not reparse. It does not build a rendered preview just to populate the list.
Drawing formats visible results only; query changes scan bounded labels, with
parent-heading matching computed once rather than once per child link.

Initial limits: 2 MiB source, 10,000 headings/links, 256 KiB aggregate labels/URLs,
4 KiB per label, 256 bytes per query, and 32 disclosure nesting levels. A 150 ms
acceptance budget is checked between parser events and after disclosure indexing;
a single parser call is not preemptible. Limits fail explicitly with Find/Go to
line as alternatives, rather than presenting an incomplete outline as complete.
Ordinary file editing retains only an optional navigator-state pointer until use.
