# The Department of Finding Things

A small playground for links, file discovery and remembering where you put things.
Start here: `editio -v examples/navigation/README.md`.

## Links worth following

Right-click these links: inspect the complete URL, open with the default app, or
open local files in Editio. Ordinary click still copies the URL.

- [Today's plan](notes/plan.md)
- [A file with spaces](notes/coffee%20budget.md#emergency-reserve)
- [A hidden heading](notes/plan.md#classified)
- [An archived plan with the same filename](archive/plan.md)
- [Missing file: expect a harmless error](notes/not-created.md)
- [Web URL](https://example.com/department/filing?status=misfiled)
- [Mail](mailto:filing@example.com)

Path separators should be subdued while these keep their usual inline-code color:
`file:///tmp/editio/example.md`, `https://example.com/reports/2026`, `notes/plan.md`.

## File finder

Open **Find other files** in Ctrl+K (or Ctrl+Shift+F). Its Ctrl+K menu switches
between filenames/content, tags, recently viewed/modified, and glob patterns.
Tab cycles **current file → recently viewed → recently modified → tags →
registered globs → current file**; Shift+Tab reverses it. Tags and registered
globs show their full lists first: filter, then Enter to search that scope.
Use **Ctrl+L** (or its command) to switch filenames/content independently.
The glob list always includes `*.md` and `**/*.md`. Register both `notes/*.md` and `archive/*.md` using the temporary-glob and
remember-pattern commands. Cycle away and back: both must appear, with no scan
until you select one. Esc from results returns to the full picker.
The tags picker lists user tags only; tag the examples below to populate it.
Type `paperclip` and go around both ways: the query stays intact. Ctrl+K should
replace results inside the same frame; Esc returns without losing the query.
Choose “Use fuzzy matching”, reopen commands, and check that it now offers
“Use exact matching”. Repeat for case sensitivity and wildcards.

Try `*.md`, `**/*.md`, `notes/*.md`, and an absolute pattern under your own home.
Remember a custom glob, close/reopen the finder, and use it again.

Filename search uses the actual filename, not the headings in these documents.
Search `plan`, then switch to content and search `paperclip`, `PAPERCLIP`, or
`p*clip` with wildcards on. Try fuzzy matching `pprclp` too.

Tag some examples without opening them (these update your normal preferences):

```sh
editio --tag navigation-demo examples/navigation/notes/plan.md
editio --tag navigation-demo 'examples/navigation/notes/coffee budget.md'
editio --tag navigation-demo examples/navigation/archive/plan.md
editio --list-tags
```

Select the `navigation-demo` tag. Empty filename searches already have results.
Open and save a disposable copy to exercise the two system histories.

```sh
editio --untag navigation-demo examples/navigation/notes/plan.md
editio --untag navigation-demo 'examples/navigation/notes/coffee budget.md'
editio --untag navigation-demo examples/navigation/archive/plan.md
```

## Link completion and unsaved changes

Switch to Edit. On a fresh line, type `[plan](notes/` and pause. Use arrows and Tab
to complete a relative destination; Esc dismisses suggestions. Try a space in a
filename and parent paths such as `../` from a nested document.

Make an unsaved change, then open another local link/file. Check **Cancel** keeps
this document, **Save** saves before switching, and **Discard** switches without
writing your edits. Missing targets must never discard the current document.

## Git ignore rules

Try the [ignore-rule playground](ignore-demo/README.md). File searches respect
Git ignore rules by default, including parent rules when started below the repo
root. Ctrl+K offers a temporary **Include Git-ignored files** command. An ignored
directory that you are already inside remains searchable.
