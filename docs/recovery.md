# Recovery drafts

Editio writes recovery snapshots, not automatic changes to the original file.
After 30 seconds without content, format or path changes, a dirty document is
checkpointed. Nonempty untitled/imported text is included. Unchanged named files
do not create drafts. Continued typing postpones the debounce; there is no
additional periodic checkpoint during continuous changes.

On SIGTERM/SIGHUP or a Windows console-close notification, the event loop requests
an immediate checkpoint of current text and waits up to three seconds for the
worker. Other terminal-I/O failures also request a checkpoint before returning.
No allocation or file I/O happens inside POSIX signal handlers. Windows' control
handler signals the loop and waits up to four seconds. Ctrl+C remains Copy.

## Recovery

Reopening the same path offers:

- **Recover draft** (default): compare against the file as it exists now. The draft
  is the editable/saveable side; the original remains unchanged until explicit
  Save. Save failure retains the draft and comparison. Successful saving exits
  recovery comparison and schedules draft cleanup.
- **Disk: discard draft**: open current disk contents and remove this draft.
- **Esc**: leave without changing either copy.

Plain `editio` offers untitled recovery, compared against an empty buffer. Piped
input and directory landing text do not get replaced with an unrelated draft.
Explicit format overrides, BOM and CRLF preferences survive recovery.
The normal 2 MiB/50,000-line diff limits still apply: larger drafts are recovered
with an explicit warning and remain editable without a computed comparison.

Save, explicit Discard and Abort remove the current session's recovery draft.
Other sessions' active drafts are neither overwritten nor removed. If more than one
inactive draft matches a path, the latest is offered first and older ones remain
available on subsequent reopening. Failed integrity checks are reported; damaged content is never silently opened.
The retention rule below still applies to recognizable old draft headers.

## Storage and resource use

Storage is always under the user's home:
`~/.local/state/editio/recovery/session-*/draft` (the Windows user home is used on
Windows as well). A draft contains a versioned JSON header followed by UTF-8 Rope
content. The header records native path bytes, explicit syntax override, original
encoding flags, byte count, timestamp and a checksum covering header and content.
Recovery does not save undo history. Unix session directories are private and
snapshot files are created with tempfile's restrictive permissions.

One lazy worker thread, a 256 KiB configured stack, and one replaceable pending
job are used; there is no extra process or unbounded queue. Rope snapshots share
text chunks. At most an active and pending checkpoint can retain older chunks.
Writing streams the Rope through a small buffer; no document-sized String or
byte vector is created for checkpointing. Checksumming is linear in document size
and happens on the worker. A checkpoint temporarily needs space for old and new
snapshot files. Ordinary editing never scans draft contents or writes to disk.

Writes flush and sync a temporary file before atomic replacement. Unix directory
entries are synced as well. Per-session OS locks prevent another instance from
claiming an active draft. Clear and write requests run in the same worker so an
older write cannot recreate a draft after an accepted Save/Discard. Write failures
leave the previous committed draft, show an error, and retry after the debounce.
Shutdown and recovery completion explicitly wake the event loop. Debounce uses
an actual deadline; an idle session needs no periodic checks or redraws.

Inactive drafts expire after **30 days since the last successful checkpoint**.
Cleanup adds no startup scan or worker: after the first UI draw and all pending
editor background work, it waits for two seconds of quiet. A lazy background
thread then makes one pass, reading bounded headers only, with a 10 ms pause
between entries. It waits when input or background work resumes and stops when
the app exits. No extra redraws or main-thread filesystem work are introduced.
An in-flight filesystem operation cannot be interrupted; cleanup is best effort.

Both the header timestamp and draft modification time must be older than 30 days.
Active/recovered sessions remain protected by exclusive locks. Unknown/corrupt
headers, future timestamps, symlinks and failed reads are skipped. Draft contents
are not loaded or checksummed for expiration. Empty session directories are
removed; uncommitted crash-left temporary files are retained rather than guessed
at. If the app stays busy or is closed quickly, cleanup waits for a later run.


## Limits and validation

No implementation can promise recovery from disk failure, full storage, power
loss, SIGKILL/TerminateProcess or every terminal shutdown path. Only completed
checkpoints are guaranteed by this protocol to replace their predecessors;
uncheckpointed changes and an interrupted pending write may be lost. Shutdown
has a deadline and does not guarantee a large/slow write will finish. Saving the
original retains the editor's external-change checks; unrelated programs are not
locked out of editing that file.

Linux tests exercise actual 30-second debounce, SIGTERM, SIGHUP, recovery against
new disk contents, save conflicts, explicit discard, cancelled recovery, atomic
write failure, checksum/length validation, session locks and pending-write cleanup.
macOS and Windows shutdown behavior has not been runtime-tested here.

## Manual check

Use a fresh test name: `editio recovery-check.md`. Enter some text, then stop
editing for 35 seconds. Do not save or choose Discard. Close the terminal window,
then reopen the same path from the same working directory. The recovery chooser
should appear; the original file is not written by recovery checkpoints.

For an untitled draft, launch `editio` without a path, enter text and repeat the
same steps. Reopen with plain `editio`. For a named but never-saved draft, reopen
its original path; the file need not exist yet. Draft files live below
`~/.local/state/editio/recovery/session-*/draft`. An instance still running owns
its draft exclusively, so another instance will not offer that active draft.

Terminal-loss regression coverage closes the actual PTY master (with no inherited
master descriptor in the child), checks process exit and draft recovery. Merely
sending SIGHUP is not sufficient coverage of terminal disappearance.

## Idle scheduling

There is no periodic recovery poll or redraw. The main loop blocks until input,
shutdown, worker completion, or the earliest actual deadline (30-second draft
debounce, two-second cleanup quiet period, or an editor timer/job). Clean files
without an existing recovery session do not schedule a draft timer. Completed
unchanged drafts are not rewritten.

A bounded 32-message queue connects a blocking terminal-input thread to the main
loop. On Unix a separate blocking signal listener wakes it for SIGTERM/SIGHUP;
Windows console close wakes the same queue. These are threads, not processes,
with 256 KiB and 128 KiB requested stacks respectively. They sleep without timers;
resident memory/allocator overhead is additional and platform-dependent. The
recovery writer remains lazy and sleeps on its condition variable between jobs.

Input failures, including terminal disappearance, end the input reader after one
error; the host then checkpoints and exits. An old running executable does not
acquire this fix when the executable on disk is rebuilt.

## Shutdown deadline

Terminal input failure, SIGHUP/SIGTERM, Windows console close, a fatal panic, or
an accepted quit/abort arms a five-second process-exit deadline. Early returns
also arm it before terminal-session restoration. It is not armed by a cancelled
quit dialog or by an ordinary save. The deadline is not reset by repeated signals.
It starts one small watchdog thread only when exiting; normal idle sessions gain
no timer, thread or polling from this backstop.

Graceful checkpointing retains its three-second finish budget. If the process
still has not exited after five seconds, the watchdog terminates it without
stdio flushing or destructors, which could themselves be stuck. Existing atomic
committed drafts survive; the last unfinished checkpoint is not guaranteed.
Normal quit retains its success status; forced shutdown exits unsuccessfully.

This is an application-level bound, not an absolute OS guarantee. A stopped or
unscheduled process cannot execute the watchdog, uninterruptible kernel I/O may
hold an exiting task, and a zombie waits for its parent to reap it. A multiplexer
that intentionally keeps a live PTY open is not terminal loss. Those conditions
cannot be repaired by a user-space timeout. Windows/macOS runtime validation is
still outstanding.
