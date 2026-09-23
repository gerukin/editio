# Responsive tables

Try widths around 40, 80, and 120 columns. Toggle the `word wrap` command in Ctrl+K to compare
wrapping with natural widths. Copying produces logical rows with tab-separated
cells, without borders, alignment padding, or display-only line breaks.

## Compact columns and long descriptions

| ID | Description | State |
| ---: | :--- | :---: |
| 7 | A long description receives the spare width while the identifier and state stay compact. | Ready |
| 104 | **Bold text**, *emphasis*, and a [link with a longer label](https://example.com/docs) retain their styling and link action across wrapped rows. | Review |
| 2 | Short description. | Done |

## Alignment, Unicode, and long tokens

| Left aligned | Centered | Right aligned |
| :--- | :---: | ---: |
| 日本語の文章 and café words should wrap at terminal display-cell boundaries. | Centered words spread over several visual lines when needed. | 1234.50 |
| `very_long_identifier_without_breaks_for_testing` | [Documentation](https://example.com/a/long/path) | 8.00 |
| https://example.com/a/very/long/unbroken/address | Short | 0.25 |

## Empty cells and unequal row heights

| Name | Notes | Result |
| --- | --- | --- |
| Ada | A much longer note wraps over several rows while adjacent cells remain empty below their content. | Pass |
| Bo | | Pending |
| | An intentionally missing name and result. | |
| Cy | Two  spaces inside a cell remain meaningful when copying wrapped text. | Pass |

## More columns than the terminal can fit

Compact columns retain their natural width. Longer columns keep at least 20 cells
when absorbing small overflow; larger overflow scrolls horizontally. Flexible columns
can grow to 80 cells normally or 120 with the width cap enabled.

| Component description | Owner information | Current investigation | Next planned action | Expected outcome |
| --- | --- | --- | --- | --- |
| Markdown rendering pipeline | Documentation maintainers | Checking nested structures and wrapped selection | Resize and copy every example | Readable content and faithful clipboard text |
| Source editing pipeline | Editor maintainers | Comparing standalone and fenced syntax colors | Check long prose and fixed code | Consistent behavior in both contexts |

## Nested tables

> [!TIP]
> Tables account for the surrounding border when calculating available width.
>
> | Kind | Explanation |
> | --- | --- |
> | Callout | This longer cell wraps inside its container without including either border in the selected text. |
> | Copy | Drag within a cell to select only that cell's content. |

> | Quoted item | Description |
> | --- | --- |
> | Muted table | All content inside this quote remains muted, including wrapped text and inline **formatting**. |
