# Code blocks

Keywords, strings, numbers, and comments use your terminal's ANSI palette.
Switch between edit and view mode to compare the same code.

## Rust

```rust
// A greeting with a numeric limit.
fn greet(name: &str) -> String {
    let limit = 3;
    format!("Hello, {name}! ({limit})")
}
```

## JavaScript

```js
const readers = ["Ada", "Grace"];
for (const name of readers) {
  console.log(`Hello, ${name}!`);
}
```

## JSON

```json
{"name": "Editio", "tabSize": 4, "enabled": true}
```

## SQL

```sql
-- A small filtered query.
SELECT name FROM readers WHERE active = TRUE LIMIT 3;
```

## Shell and language aliases

```bash
# This is display-only sample text, not an application script.
name="reader"
printf 'Hello, %s\n' "$name"
```

## Multiple-line comments

```c
/* The comment remains colored
   on the second line, too. */
int count = 42;
```
