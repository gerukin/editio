# Sequences: control blocks

Each frame is separate: nested control blocks are currently unsupported.

## Alternative and optional paths

```mermaid
sequenceDiagram
    participant UI
    participant API
    alt Document changed
        UI->>API: Save
        API-->>UI: Stored
    else Unchanged
        API-->>UI: Nothing to save
    end
    opt Notifications enabled
        API->>UI: Show confirmation
    end
```

## Loops and early termination

```mermaid
sequenceDiagram
    participant Client
    participant Server
    loop Every 30 seconds
        Client->>Server: Check revision
        Server-->>Client: Revision number
    end
    break Connection closed
        Server-->>Client: Stop polling
    end
```

## Parallel work and critical sections

```mermaid
sequenceDiagram
    participant UI
    participant API
    participant Index
    par Save content
        UI->>API: Persist
    and Refresh search
        UI->>Index: Update
    end
    critical Acquire file lock
        API->>API: Atomic write
    option Lock unavailable
        API-->>UI: Retry later
    end
```

## Highlighted region and overlapping parallel work

```mermaid
sequenceDiagram
    participant Client
    participant Service
    rect rgb(230, 240, 255)
        Client->>Service: Important phase
        Service-->>Client: Completed
    end
    par_over Background work
        Client->>Service: Reindex
        Service-->>Client: Continue editing
    end
```
