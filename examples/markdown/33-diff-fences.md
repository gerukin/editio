# Diff fences

Explicit diff fences display the supplied changes. They never compare files.
Both versions stay visible; copying preserves the original patch text.

## A plain patch

```diff
--- a/settings.txt
+++ b/settings.txt
@@ -1,2 +1,2 @@
-theme=terminal
+theme=tokyo-night
 wrap=true
```

## Diff with Rust syntax

```diff-rust
 fn greeting(name: &str) -> String {
-    format!("Hello {name}")
+    format!("Welcome, {name}!")
 }
```

## Wrapped changes inside a quote

> ```diff-text
> -This long previous description illustrates a removed line wrapping across the available viewport, with a red minus sign and no file-change border.
> +This long replacement description illustrates an added line wrapping across the available viewport, with a green plus sign and no file-change border.
> ```

## Ordinary code stays ordinary

```text
-negative_expression
+positive_expression
```

Only an explicit `diff`, `patch`, `udiff`, or `diff-<language>` fence enables
diff styling. No comparisons, files, commands, or background workers are created.
