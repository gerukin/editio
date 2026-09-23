# Greedy tables and quiet separators

Resize to 50, 80, 100 and 160 columns. In Ctrl+K, cycle **Table row separators**
and **Table column separators**. These preferences apply to every table format.
Adaptive rules disappear below 24 document rows or 60 document columns respectively.
The header underline always remains.

## Compact identifiers, flexible prose

The ID and state stay at their natural widths. Description uses its natural width,
up to the viewport width, or the smaller of that and 120 when the 120-column
wrapping option is enabled. A whole table may still overflow horizontally.

Columns never expand beyond their content. On a wide terminal, this short
four-column table should leave unused space on the right:

| Action | Linux | Windows | macOS |
| --- | --- | --- | --- |
| Copy lines up/down | Ctrl+Shift+Alt+↑/↓ | Shift+Alt+↑/↓ | Shift+Option+↑/↓ |
| Add cursor above/below | Shift+Alt+↑/↓ | Ctrl+Alt+↑/↓ | Cmd+Option+↑/↓ |
| Delete current/selected lines | Ctrl+Shift+K | Ctrl+Shift+K | Cmd+Shift+K |

| ID | Description | State |
| ---: | --- | :---: |
| 7 | A description with enough content to benefit from the wider viewport. Resize to see it receive the available space while the short columns remain compact. | Ready |
| 104 | **Styled text**, [a link](https://example.com), and 日本語 remain selectable without rules or display padding. | Done |
| 2 | Short text. | Ready |

## Slight overflow versus deliberate horizontal scrolling

A deficit of at most 30 percent of the viewport width is absorbed by flexible columns. With a much
narrower viewport this table overflows: scroll right rather than crushing every cell.

| Compact | First explanation | Second explanation |
| --- | --- | --- |
| ID-21 | This explanation occupies roughly fifty cells. | Another explanation uses approximately fifty cells. |
| ID-22 | Brief. | Brief too. |

## Individual cell caps, not a whole-table cap

With wrap:120, each cell is at most 120 display cells. The entire table can be
wider than 120 and may overflow a narrow viewport.

| First description | Second description | Third description |
| --- | --- | --- |
| This deliberately long cell contains enough ordinary words to demonstrate wrapping while preserving readable lines and leaving other columns their fair share of the width. | Another long cell explains that table layout remains independent of the 120-column cap applied to ordinary document text; the cap applies separately to each cell instead. | A final long cell ensures this table can exceed the viewport and still retain readable columns. Horizontal scrolling keeps the header and cell boundaries aligned. |

## Compact tables do not stretch meaningless space

All columns here are narrow, so this table stays compact even on a wide screen.

| ID | State | Count |
| ---: | --- | ---: |
| 1 | Done | 12 |
| 2 | Ready | 345 |
