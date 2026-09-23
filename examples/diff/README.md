# Diff examples

- `editio --diff examples/diff/before.rs examples/diff/after.rs`
- `editio -v examples/diff/changes.patch`
- `editio -v examples/diff/inline.rs`

Ctrl+P/N cycles change blocks in View or Edit. In a two-file comparison, `[` shows
the reference and `]` shows the current file. Ctrl+G always edits the second file;
Ctrl+S saves only it. Standalone patches have no reference view and are never applied.

Try word wrap in the palette and resize to 24 columns. The fixed right marker
covers continuation rows; it never becomes copied text. Red/green signs remain
literal in Edit. View omits removed lines and strips the diff signs. A patch's
hunk headers identify fragments with omitted intervening content.
