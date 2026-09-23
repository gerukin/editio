# The Department of Selective Memory

This folder inherits Git rules from the repository, even when opened directly.
Its own `.gitignore` excludes `scratch/` and `*.private.md`, but re-includes
`public.private.md`. Create disposable test material locally:

```sh
mkdir -p examples/navigation/ignore-demo/scratch
cp examples/navigation/notes/plan.md examples/navigation/ignore-demo/scratch/draft.md
cp examples/navigation/notes/plan.md examples/navigation/ignore-demo/notes.private.md
editio -v examples/navigation/ignore-demo/README.md
```

1. Find → globs → `**/*.md`: neither disposable file should appear by default.
2. Ctrl+K → **Include Git-ignored files**: both become searchable, by name or content.
3. Ctrl+K → **Respect Git ignore rules**: they disappear again.
4. Quit and reopen: ignore filtering is enabled again; preferences did not change.
5. Open `scratch/draft.md` directly and search `**/*.md`: `draft.md` is searchable,
   because the starting directory is explicit, even though its parent ignored it.

`public.private.md` demonstrates a negated rule. Hidden files are not excluded
merely because their names start with a dot. Normal Find within the open document
always searches that document.
