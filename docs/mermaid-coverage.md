# Mermaid terminal coverage

The gallery targets Merman ASCII/Unicode 0.7.0, not Mermaid's browser renderer.
Supported examples are rendered without visible fences. Wide layouts scroll
horizontally instead of wrapping and breaking borders.

| Gallery | Features exercised |
| --- | --- |
| 13-flowchart-layouts | Decisions, rounded/circle/stadium/subroutine/database shapes, branching/merging, feedback/self-loops, nested groups, external connections, labeled/open/dotted/thick/long edges, multiline labels |
| 14-flowchart-directions | LR/TD/TB/BT/RL, `graph` alias, local LR groups within TD, disconnected components |
| 15-sequence-messages | Aliases, title, autonumber start/step, solid/dotted/open/filled/cross messages, self calls, three note placements, activations, groups, create/destroy, wrapping |
| 16-sequence-control | alt/else, opt, loop, break, par/and, critical/option, rect, par_over |
| 17-class-diagrams | Visibility, attributes, methods, static members, interface annotation, inheritance, realization, composition, aggregation, association, dependency, namespaces, branching and disconnected classes |
| 18-entity-relationships | PK/FK/UK, identifying/non-identifying relations, mandatory/optional and one/many cardinalities, chains, parallel routes, independent entities |
| 19-charts | Titles, categories/numeric/inferred axes, ranges, negative values, bars, lines, overlays, multiple series, horizontal orientation |
| 20-mermaid-boundaries | Explicit failures: pie/state/gantt families, hexagons, nested sequence frames, actor shapes, class endpoint cardinalities and unmarked associations |

The integration test checks 28 supported diagrams against committed Unicode
snapshots and eight expected failures. Update snapshots deliberately with
`UPDATE_MERMAID_SNAPSHOTS=1 cargo test --test mermaid_examples`, then inspect the
changed files in `tests/snapshots/`. Tests also exercise the Markdown rendering
path at narrow/wide widths and check that Mermaid syntax/fences are absent.

Limitations: terminal geometry approximates supported shapes. Mermaid fills,
backgrounds, links/callbacks, icons/images, and browser typography are not rendered.
Some crossing class layouts and mixed subgraph directions remain unsupported.
This is broad feature coverage, not a claim of every Mermaid expression.

Primary references: [Merman flowchart support](https://docs.rs/crate/merman-ascii/0.7.0/source/FLOWCHART_SUPPORT.md),
[Merman sequence support](https://docs.rs/crate/merman-ascii/0.7.0/source/SEQUENCE_SUPPORT.md),
and the parser/model tests shipped with that crate for classes, ER, and XY charts.
