# Sticky table headers

Scroll down and back up. Each header stays with its own table, then leaves with
that table's last row. Try Ctrl+G repeatedly halfway through a table: the viewport
should return to the same place. Outer borders are absent; internal separators
and the muted header rule remain.

| Item | Description |
| --- | --- |
| 01 | First record |
| 02 | Second record |
| 03 | Third record |
| 04 | Fourth record |
| 05 | Fifth record |
| 06 | Sixth record |
| 07 | Seventh record |
| 08 | Eighth record |
| 09 | Ninth record |
| 10 | Tenth record |
| 11 | Eleventh record |
| 12 | Twelfth record |
| 13 | Thirteenth record |
| 14 | Fourteenth record |
| 15 | Fifteenth record |
| 16 | Sixteenth record |
| 17 | Seventeenth record |
| 18 | Eighteenth record |
| 19 | Nineteenth record |
| 20 | Last record of the first table |

This paragraph belongs to neither table. The first header should have left the
viewport before the next table begins to stick.

| A single column with a deliberately long header that wraps onto multiple lines on narrow screens |
| --- |
| This long cell stays at most forty display cells wide when wrapping is enabled, even in a very wide terminal. The 120-column option also enforces this per-column cap without imposing a total table-width limit. |
| A short second cell. |
| Another long cell demonstrates word wrapping, 日本語, and an_unbroken_identifier_that_exceeds_the_column_cap_and_must_wrap. |
| Last row of the second table. |

## After the tables

Only this content should remain once both tables have scrolled away.

First paragraph after the tables.

Second paragraph after the tables.

Third paragraph after the tables.

Fourth paragraph after the tables.

Fifth paragraph after the tables.

Sixth paragraph after the tables.

Seventh paragraph after the tables.

Eighth paragraph after the tables.

Ninth paragraph after the tables.

Tenth paragraph after the tables.
