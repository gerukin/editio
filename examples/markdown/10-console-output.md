# Console and build-output grammars

These syntax definitions are selected explicitly by their fence names.
All output below is illustrative fixture text, not a command transcript.

## Cargo build results

```Cargo Build Results
   Compiling greeting v0.1.0
warning: unused variable: `count`
 --> src/main.rs:2:9
  |
2 |     let count = 3;
  |         ^^^^^ help: prefix it with an underscore: `_count`
    Finished dev profile in 0.05s
```

## Make output

```Make Output
cc -Wall -o greeting greeting.c
greeting.c:4:9: warning: unused variable 'count'
make: Leaving directory '/example'
```

## LaTeX log

```LaTeX Log
This is pdfTeX, Version 3.141592653
(./document.tex
LaTeX Warning: Reference `intro' undefined on input line 12.
Output written on document.pdf (1 page).
)
```

## R console

```R Console
> values <- c(1, 2, 3)
> mean(values)
[1] 2
```
