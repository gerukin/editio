# Classes: members, relationships, and namespaces

## Interfaces, inheritance, and implementation

```mermaid
classDiagram
    class Document {
        +String title
        -String content
        +save() bool
        +load(path) Document$
    }
    class MarkdownDocument {
        +render() String
    }
    class Renderable {
        <<interface>>
        +render() String
    }
    Document <|-- MarkdownDocument : extends
    Renderable <|.. MarkdownDocument : implements
```

## Ownership, association, and dependency

```mermaid
classDiagram
    Workspace *-- Document : owns
    Workspace o-- Plugin : uses
```

## Association and dependency chain

```mermaid
classDiagram
    Editor --> Document : edits
    Document ..> History : tracks
    class Document {
        +String title
    }
    class History {
        +undo()
        +redo()
    }
```

## Namespaces, branching inheritance, and disconnected components

```mermaid
classDiagram
    namespace Content {
        class Note {
            +String title
        }
        class Article
        class Checklist
    }
    Note <|-- Article
    Note <|-- Checklist
    class Settings {
        +bool preview
    }
```
