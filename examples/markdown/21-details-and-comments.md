# Details and comments

<!-- This comment must never appear in view mode. -->

This paragraph has an inline <!-- hidden author note -->comment between words.

## Collapsed by default

<details>
<summary>Click to reveal formatting examples</summary>

This is **bold**, this is *italic*, and [this link](https://www.rust-lang.org/)
can be copied using the mouse.

> [!TIP]
> Selecting this text excludes the colored border and its padding.
>
> - Markdown lists work here.
> - So does `inline code`.
>
> > A nested quote remains a quote.
>
> ```rust
> let answer = 42;
> println!("{answer}");
> ```

| Name | Value |
| --- | --- |
| Alpha | One |
| Beta | Two |

<details open>
<summary>Nested section, open by default</summary>

The child remains expanded when the parent is closed and opened again.

</details>
</details>

## Expanded by default

<details open>
<summary>Visible immediately</summary>

This body is visible initially. Click the summary to collapse it.

---

The rule above should stretch across the available width.

</details>

## A default summary

<details>

No summary was supplied, so the control says **Details**.

</details>

## Literal HTML in code

Inline `<!-- keep this literal -->` is code, not a hidden comment.

```html
<!-- This code example remains visible. -->
<details open>
  <summary>This is literal code, not a control</summary>
  Body
</details>
```

<!-- A multiline comment.
Nothing in this block should appear,
including <details open>fake controls</details>.
-->

## Duplicate heading!

Click the muted trailing # to copy a working-directory-relative path and anchor.
Right-click it to copy the heading text, relative path, or absolute path.

## Duplicate heading!

This second heading receives a distinct anchor.

A setext heading
---------------

Setext headings have anchors too.
