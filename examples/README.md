# Example files

Open a small example directly:

```text
./target/release/editio --view examples/markdown/03-code.md
./target/release/editio examples/syntax/greeting.rs
```

These are display/editing fixtures, never executed by the application or its tests.
Samples written in other languages (including Python and shell) are document data;
all implementation, tests, and development tooling remain Rust-only.

## Markdown, one topic at a time

- [Heading hierarchy, narrow layouts, and wrapping](markdown/30-heading-hierarchy-and-wrapping.md)

- [Collapsed YAML frontmatter and open/closed details](markdown/29-frontmatter-and-details.md)

- [Prose and inline formatting](markdown/01-prose.md)
- [Lists, tasks, tables, alerts, and footnotes](markdown/02-lists-and-tables.md)
- [Syntax-highlighted code blocks](markdown/03-code.md)
- [Mermaid flowcharts](markdown/04-flowcharts.md)
- [Mermaid sequence diagrams](markdown/05-sequences.md)
- [Class, ER, and XY diagrams](markdown/06-data-diagrams.md)
- [Tilde fences, longer fences, aliases, and quoted code](markdown/07-fences.md)
- [Math, image, HTML, and unsupported-diagram fallbacks](markdown/08-rendering-fallbacks.md)
- [Embedded syntax definitions](markdown/09-embedded-syntaxes.md)
- [Console/build-output syntax definitions](markdown/10-console-output.md)
- [All five callouts, six heading levels, and clickable links](markdown/11-callouts.md)
- [GitHub language aliases and expanded fence grammars](markdown/12-language-aliases.md)
- [Complex flowchart layouts](markdown/13-flowchart-layouts.md)
- [Flowchart directions and local groups](markdown/14-flowchart-directions.md)
- [Sequence messages, lifecycle, notes, and groups](markdown/15-sequence-messages.md)
- [Sequence control blocks](markdown/16-sequence-control.md)
- [Class relationships and namespaces](markdown/17-class-diagrams.md)
- [Entity relationships and cardinalities](markdown/18-entity-relationships.md)
- [Charts, mixed series, negative ranges, and horizontal axes](markdown/19-charts.md)
- [Expected Mermaid limitations](markdown/20-mermaid-boundaries.md)
- [Details, comments, heading anchors, rules, and rich callouts](markdown/21-details-and-comments.md)
- [Heading markers and muted rich quotes](markdown/22-headings-and-quotes.md)
- [Table styling in view and edit modes](markdown/23-table-source-styles.md)
- [Smart wrapping, hanging indentation, and selection](markdown/24-smart-wrapping.md)
- [Code labels, borders, language wrapping, and nested sections](markdown/25-code-block-layout.md)
- [Responsive tables, column minima, alignment, and selection](markdown/26-responsive-tables.md)
- [Optional 120-column limit and unaffected tables/diagrams](markdown/27-column-limit.md)

See the [Mermaid coverage matrix](../docs/mermaid-coverage.md) for tested features
and expected fallbacks. Links copy their URLs on click; right-click offers link
text or URL. Dragging selects rendered text while remaining in view mode.

Toggle view/edit with Ctrl+G (or `g` from view mode). Toggle rendered Markdown/source
with the `Render / source` command in Ctrl+K. `--monochrome` removes syntax colors.

## Other supported file types

Table previews: [CSV](tables/people.csv) and [TSV](tables/inventory.tsv), including quoted separators, multiline fields, Unicode, and empty values. Open with `--view`; arrow keys scroll the table and the `Render / source` palette command shows source.

The [catalog](catalog.tsv) maps every one of the 63 filename-based syntax types
bundled with this version to a sample and lists all recognized filename aliases.
There is one representative file per syntax, not a duplicate for each extension.
The 12 extra grammars without filename extensions are shown in Markdown fences.
A Rust test compares the examples with the actual bundled catalog.

Syntax detection recognizes compound suffixes (`.js.erb`, `.sql.erb`), special
filenames (`Makefile`), and known first-line markers. Unrecognized files remain
editable as plain text; this catalog does not claim dedicated highlighting for
languages absent from the bundle. Standalone files and Markdown fences share
two-face's expanded grammars, including TypeScript and TOML; the catalog above
covers the original baseline syntax set. The complete GitHub
identifier [coverage report](../docs/language-support.tsv) explicitly lists
grammar choices and plain-text fallbacks.

`markdown/28-sticky-tables.md` demonstrates multiple sticky headers, trailing
paragraphs, a wrapped header, and the 40-cell column cap. `tables/sticky.csv` and
`tables/sticky.tsv` use the same table layout and include a multiline field.

### JSON pretty-printing

Open `examples/syntax/pretty-view.json` in View to see formatted JSON. Use Ctrl+K,
then “Toggle JSON pretty printing” to compare its original compact layout.
Edit mode and saving preserve the original source until you edit it.
