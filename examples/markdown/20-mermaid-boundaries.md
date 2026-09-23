# Mermaid boundary examples

These intentionally exercise unsupported features. Each should show an explicit
explanation and readable source instead of silently omitting meaning.

## Unsupported diagram families

```mermaid
pie title Files
    "Markdown" : 70
    "Other" : 30
```

```mermaid
stateDiagram-v2
    [*] --> Reading
    Reading --> Editing
    Editing --> [*]
```

```mermaid
gantt
    title Release plan
    dateFormat YYYY-MM-DD
    section Work
    Write :2026-09-01, 3d
```

## Unsupported flowchart shape

```mermaid
flowchart LR
    A{{Hexagon}} --> B[Rectangle]
```

## Nested sequence frames

```mermaid
sequenceDiagram
    participant A
    participant B
    loop Retry
        alt Success
            A->>B: Complete
        else Failure
            B-->>A: Retry
        end
    end
```

## Actor-shaped participants

```mermaid
sequenceDiagram
    actor Reader
    participant Editor
    Reader->>Editor: Open
```

## Class relationship endpoint cardinalities

```mermaid
classDiagram
    Workspace "1" *-- "many" Document : owns
```

## Unmarked class associations

```mermaid
classDiagram
    Document -- History : tracks
```
