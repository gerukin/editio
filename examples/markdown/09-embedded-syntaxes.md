# Embedded grammar samples

These bundled grammars have no filename extensions. They are demonstrated as
explicitly named code fences, rather than pretending they are standalone file types.

## Java documentation

```JavaDoc
/**
 * Greet a reader.
 * @param name the reader's name
 * @return a greeting
 */
```

## JavaScript and Python regular expressions

```Regular Expressions (Javascript)
^(hello|welcome),\s+(?<name>[A-Za-z]+)!$
```

```Regular Expressions (Python)
(?P<name>[A-Za-z]+)\s*=\s*(?P<value>\d+)
```

## Embedded PHP

```PHP Source
$name = "reader";
echo "Hello, {$name}!";
```

## MultiMarkdown

```MultiMarkdown
Title: A small note

## A heading

A **bold** thought with a footnote.[^one]

[^one]: A short explanation.
```

## camlp4

```camlp4
let parse = parser
  | [< 'Genlex.Int n >] -> n
```

## Shell grammar components

```Shell-Unix-Generic
name="reader"
if test -n "$name"; then
  echo "Hello, $name"
fi
```

```commands-builtin-shell-bash
printf 'Hello, %s\n' reader
export EDITOR=editio
```
