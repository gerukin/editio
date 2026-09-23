# Sequences: messages, lifecycles, notes, and groups

## Request handling with aliases, activation, and numbering

```mermaid
sequenceDiagram
    title Saving a document
    autonumber 10 5
    participant UI as Editor
    participant API as File service
    participant Disk as Local disk
    UI->>+API: Save content
    Note left of UI: Unsaved changes
    API->>API: Validate UTF-8
    API->>Disk: Write temporary file
    Disk-->>API: Write complete
    Note over API,Disk: Atomic rename
    API-->>-UI: Saved
    Note right of UI: Ready
```

## Open and cross-ended messages

```mermaid
sequenceDiagram
    participant Client
    participant Server
    Client->Server: Open message
    Server-->Client: Dotted open reply
    Client-xServer: Cancel operation
    Server--xClient: Cancellation acknowledged
    Client->>Server: Filled arrow
    Server-->>Client: Dotted filled reply
```

## Participant groups and temporary workers

```mermaid
sequenceDiagram
    box Frontend
        participant UI
    end
    box Backend
        participant API
    end
    UI->>API: Start job
    create participant Worker
    API->>Worker: Create task
    Worker-->>API: Result
    destroy Worker
    API-xWorker: Dispose
    API-->>UI: Complete
```

## Wrapped message and note text

```mermaid
sequenceDiagram
    participant Client
    participant Service
    Client->>Service: wrap: A longer request description that demonstrates message wrapping in a terminal diagram
    Note over Client,Service: wrap: Both participants agree on a stable document format before saving
    Service-->>Client: Accepted
```
