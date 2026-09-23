# A little space for tabs

This folder has an EditorConfig: Markdown uses two spaces, Rust uses four,
and other files use tabs displayed at width two. Open Info to see the source.

- Humanity delegates its chores.
  > The robots request a clearly indented contract.
  > This longer quote should wrap inside its list item, retaining both its border and alignment.
- The contract includes code:
  ```rust
  fn delegate() {
      println!("The human is supervising a nap.");
  }
  ```
  This paragraph still belongs to the same item.

  So does this second paragraph.

- Nested responsibilities:
   - Three source spaces: this marker has a color in Edit.
    - Four source spaces: another nested item, also colored.

## Standard Markdown still applies

This paragraph continues
   despite these leading spaces.

Four spaces after a blank line make code:

    This is an indented code block.

- A deliberate Setext heading inside a list
  ---

Use Ctrl+] / Ctrl+[ (Cmd on macOS) on current/selected lines. Use the
palette's indentation commands to change policy without rewriting text, then
try an explicit conversion and Ctrl+Z. Use automatic settings to restore this
folder's policy. Conversion preserves columns, not inferred nesting levels.
