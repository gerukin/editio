# Heading level one

## Heading level two

### Heading level three

#### Heading level four

##### Heading level five

###### Heading level six

Level one is centered between muted horizontal rules, with no hash markers.
H2 uses normal muted rules; H3 dims them; H4–H6 omit trailing rules.
Below 60 columns, H2–H6 use at most one leading rule character; H1 still fills both sides. Wrapped lines
have no rules and align with the first line's text. Rules and separating spaces
are not underlined or copied. See `30-heading-hierarchy-and-wrapping.md`.

> Normal quote text stays in the terminal's text color, with the whole block dimmed.
> **Bold**, *italic*, ~~deleted~~, `inline code`, and [links](https://example.com) retain their formatting.
>
> ### A heading inside a quote
>
> - A list item
> - [x] A completed task
>
> ```rust
> fn answer() -> usize {
>     42
> }
> ```
>
> | Key | Value |
> | --- | --- |
> | greeting | **Hello** |
> | number | `42` |
>
> > A nested quote, also dimmed. Selecting it excludes both borders.
>
> > [!TIP]
> > A nested callout retains its accent color and inherits the quote's dimming.
>
> <details open>
> <summary>Expandable quoted content</summary>
>
> **This content** starts expanded and stays inside the quote.
>
> </details>
>
> ```mermaid
> flowchart LR
>     Read --> Edit
>     Edit --> Preview
>     Preview --> Read
> ```
>
> This deliberately long paragraph demonstrates that narrow terminals wrap quoted prose while repeating the colored border and keeping every continuation muted. Resize the terminal to inspect the result.

Outside the quote, text returns to normal brightness.
