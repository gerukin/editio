# Embedding Editio

Requires access to the private sibling `tapp-ui` checkout.

Use `Editor::new(Buffer::new(text))`, `Editor::draw(frame, rect)`, and `Editor::handle(event)`. The host owns the terminal, event loop, saving, clipboard, layout, and focus. `Outcome` distinguishes edits, save/quit requests, clipboard requests, unhandled input, and focus release. `Editor::paste` inserts host-provided clipboard text.

Save and quit confirmations are part of the editor component. On `SaveRequested`, perform the save and pass its result to `Editor::save_finished(result)`; exit if it returns `QuitRequested`. A direct `QuitRequested` has already passed any necessary confirmation (including explicit discard). `Buffer::save()` creates missing parents and protects newly created files from overwriting a file that appeared after confirmation.

Esc always returns `FocusReleased`. Add `KeyEvent`s to `handoff_keys` (e.g. Enter or Ctrl+Enter) to release focus before editor or modal handling; the triggering key is not inserted. Retain the `Editor` to preserve text, cursors, and undo history. Set `options_key` and `palette_key` for host-specific bindings. Use buffer methods for mutations so revision tracking stays valid.

Use `next_wakeup()` as the event-wait timeout; when it fires, call both `poll_background()` and `poll_timers()`, then redraw if either returns true. A `None` timeout means the host can block without idle polling. `notify(text)` shows a three-second informational toast; `notify_success`, `notify_warning`, and `notify_error` assign severity explicitly. `notify_with_kind(ToastKind, text)` colors only the border: blue Note, green Tip/success, magenta Important, yellow Warning, and red Caution/error, respecting monochrome mode. Bodies are literal text by default; `notify_markdown(ToastKind, text)` opts into lightweight emphasis, code, links, lists, and breaks. Toast rendering is bounded to 8,192 characters and the visible rows. `persistent_message(text)` remains available for host-provided persistent status messages. `has_background_work()` reports pending rendering, search, and saved-content verification. The host handles `CopyRequested` for links and heading paths just as for normal selections.

