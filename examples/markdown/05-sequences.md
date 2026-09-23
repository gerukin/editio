# Mermaid sequence diagrams

Participants and message labels should stay readable.

```mermaid
sequenceDiagram
    participant Reader
    participant Editor
    Reader->>Editor: Open notes.md
    Editor-->>Reader: Show highlighted text
    Reader->>Editor: Save changes
    Editor-->>Reader: Saved
```

This file focuses on one diagram family so it is easy to inspect its layout.
