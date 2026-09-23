# Flowcharts: shapes, branches, groups, and routes

Scroll horizontally with Left/Right when a layout exceeds the terminal width.

## A publishing pipeline

```mermaid
flowchart TD
    Start([Draft]) --> Review{Approved?}
    Review -->|yes| Publish[[Publish release]]
    Review -->|no| Revise[Revise text]
    Revise --> Review
    Publish --> Store[(Archive)]
    Store --> Done((Done))
```

## Nested groups and cross-boundary connections

```mermaid
flowchart LR
    User[Reader] --> Cache
    subgraph Delivery[Delivery system]
        Cache[Page cache] --> API[Content API]
        subgraph Storage[Persistent storage]
            Notes[(Notes)]
            Assets[(Assets)]
        end
        API --> Notes
        API --> Assets
    end
    API --> Audit[Audit log]
```

## Open, dotted, thick, labeled, and longer edges

```mermaid
flowchart LR
    Draft[Draft] --- Review[Review]
    Review -. optional .-> Check[Spell check]
    Review ==> Release[Release]
    Release ----> Archive[Archive]
```

## Fan-out, merge, feedback, and a self-loop

```mermaid
flowchart TD
    Queue[Queue] --> A[Worker A]
    Queue --> B[Worker B]
    A --> Merge[Results]
    B --> Merge
    Merge --> Queue
    A --> A
```

## Labels with explicit line breaks

```mermaid
flowchart LR
    A[Read<br/>document] --> B[Check<br/>formatting]
```
