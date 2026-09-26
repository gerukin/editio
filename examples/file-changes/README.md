# Two editors walk into a file

Copy `document.md` to a temporary working directory before testing.

1. Open that copy in two Editio terminals.
2. Edit and save in the first. The clean second instance should reload and toast.
3. In the second, find `checkpoint` and select a result. Change an unrelated line
   in the first and save: search refreshes while keeping the surviving result.
4. Leave an unsaved edit in both. Save the first: the second must ask what to do.
5. Try **Keep unsaved copy**, **Copy + compare**, and **Discard edits / load disk**
   in separate runs. Keeping a copy must never overwrite the first instance's file.
6. Save a kept copy under `nested/new-copy.md`; directories are created. Comparison
   ends after saving. Cancel/retry with an existing destination to check protection.
7. Ctrl+K → `duplicate`: try both choices with unsaved edits, then cancel the path
   dialog or use a different name. The default path is deliberately not acceptable.
8. Disable monitoring in Ctrl+K, save from the other terminal, then enable it again.
   Restarting always enables monitoring unless you pass `--no-watch`.

For replacement-style writes, edit with another editor that saves atomically.
Deleting or moving the watched file must leave its in-memory text intact. Recreate
it under the same name to resume updates. Unrelated files should do nothing.
