# Fence language names and alternatives

Aliases are case insensitive. See `docs/language-support.tsv` for the bundled
grammar or plain-text fallback for every GitHub identifier.

## JavaScript aliases

```JS
const greet = (name) => `Hello ${name}`;
```

```node
console.log("Same JavaScript grammar");
```

## TypeScript and TSX

```ts
interface Note { title: string; done: boolean }
const note: Note = { title: "Read", done: false };
```

```tsx
export const Greeting = () => <h1>Hello!</h1>;
```

## Shell, YAML, C#, and PowerShell aliases

```shell
for name in Ada Grace; do echo "Hello $name"; done
```

```yml
editor:
  preview: true
  languages: [rust, markdown]
```

```csharp
record Note(string Title, bool Done);
```

```pwsh
$names = @('Ada', 'Grace')
$names | ForEach-Object { "Hello $_" }
```

## Configuration and modern languages

```toml
[editor]
preview = true
```

```kotlin
fun greet(name: String) = "Hello $name"
```

```swift
let greeting = "Hello, world!"
```

```dockerfile
FROM scratch
COPY app /app
ENTRYPOINT ["/app"]
```
