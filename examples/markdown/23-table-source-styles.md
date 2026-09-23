# Tables in view and edit mode

Switch modes to compare colors. Edit mode keeps every pipe, backslash, alignment
marker, URL, and formatting delimiter visible.

| **Feature** | *Example* | `Status` |
| :--- | :---: | ---: |
| Bold | **Important text** | Ready |
| Emphasis | *Soft emphasis* | Ready |
| Deletion | ~~Old value~~ | Ready |
| Inline code | `value = 42` | Ready |
| Link | [Rust](https://www.rust-lang.org/) | Ready |
| Reference link | [Documentation][docs] | Ready |
| Escaped pipe | `left\|right` | Ready |
| Unicode | 日本語 • café | Ready |

## Optional outer pipes

Name | Description
:--- | ---:
Alice | **Maintainer**
Bob | [Documentation][docs]

## Tables inside other blocks

> | Quoted header | Value |
> | --- | --- |
> | Muted text | `quoted code` |
> | **Bold quote** | [Documentation][docs] |

> [!NOTE]
> | Callout header | Value |
> | --- | --- |
> | Normal body color | *Emphasis* |

## Literal table source

```markdown
| These characters | Are inside code |
| --- | --- |
| Preserve the language grammar | No table layout here |
```

[docs]: https://doc.rust-lang.org/book/
