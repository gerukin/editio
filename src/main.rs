use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use editio::{Editor, Mode, Outcome, buffer::Buffer, terminal::Session};
use std::{io, io::IsTerminal, path::PathBuf, process::ExitCode};
mod editorconfig;
mod events;
mod file_ui;
mod files;
mod links;
mod piped;
mod recovery;
mod recovery_prompt;
mod shutdown;
#[derive(Parser)]
#[command(
    version,
    about = "Small terminal text editor. Ctrl+K: filter commands; Ctrl+?: help; F2: palette fallback."
)]
struct Args {
    /// File or directory to open; '-' reads stdin (also automatic when piped)
    file: Option<PathBuf>,
    /// Add a global tag to FILE without opening the editor (repeatable)
    #[arg(long)]
    tag: Vec<String>,
    /// Remove a global tag from FILE without opening the editor (repeatable)
    #[arg(long)]
    untag: Vec<String>,
    /// List global user tags without opening the editor
    #[arg(long)]
    list_tags: bool,
    /// Compare two files in unified View; Edit and Save affect only MODIFIED
    #[arg(long, num_args = 2, value_names = ["ORIGINAL", "MODIFIED"], conflicts_with_all = ["file", "format"])]
    diff: Vec<PathBuf>,
    /// Override detection using a Markdown code-fence language name or alias
    #[arg(long, value_parser = parse_format)]
    format: Option<String>,
    /// Start in view mode (switch to editing with Ctrl+G) (default is edit for $EDITOR integration)
    #[arg(short = 'v', long)]
    view: bool,
    #[arg(long, default_value_t = 1)]
    line: usize,
    #[arg(long, default_value_t = 1)]
    column: usize,
    /// Use monochrome source styling
    #[arg(long)]
    monochrome: bool,
    /// Disable soft wrapping for Markdown and prose files
    #[arg(long)]
    no_wrap: bool,
    /// Disable automatic copying when a text selection is completed
    #[arg(long)]
    no_copy_on_selection: bool,
    /// Show line numbers (also available in the command center)
    #[arg(long)]
    line_numbers: bool,
    /// Force text wrapping at 120 columns or the available width, whichever is smaller
    #[arg(long)]
    limit_width: bool,
    /// Command palette key: ctrl-k (default), ctrl-space, or f2
    #[arg(long,default_value="ctrl-k",value_parser=["ctrl-k","ctrl-space","f2"])]
    options_key: String,
}
fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            let _ = std::io::Write::write_fmt(&mut io::stderr(), format_args!("editio: {error}\n"));
            ExitCode::FAILURE
        }
    }
}

const DIRECTORY_NOTICE: &str = "Opening directories will be supported in a future version of Editio.\n\nFor now, replace this text with your content and press Ctrl+S to save a file here.\nEnter a relative file name, such as notes.md, in the save dialog.\n";

fn run() -> io::Result<ExitCode> {
    let args = Args::parse();
    if args.list_tags || !args.tag.is_empty() || !args.untag.is_empty() {
        let path = files::prefs_path()?;
        if !args.tag.is_empty() || !args.untag.is_empty() {
            let file = args
                .file
                .as_deref()
                .ok_or_else(|| io::Error::other("Tagging requires a file path"))?;
            files::Catalog::update(&path, |c| {
                for tag in &args.tag {
                    c.tag(file, tag, false)?;
                }
                for tag in &args.untag {
                    c.tag(file, tag, true)?;
                }
                Ok(())
            })?;
        }
        if args.list_tags {
            for (tag, paths) in files::Catalog::load(&path)?.tags {
                println!("{tag}\t{}", paths.len());
            }
        }
        return Ok(ExitCode::SUCCESS);
    }
    if !io::stdout().is_terminal() {
        return Err(io::Error::other(
            "interactive editing requires terminal output",
        ));
    }
    let explicit_stdin = args.file.as_deref() == Some(std::path::Path::new("-"));
    let pipe = !io::stdin().is_terminal();
    if pipe && !args.diff.is_empty() {
        return Err(io::Error::other(
            "--diff requires two files and terminal input",
        ));
    }
    if explicit_stdin && !pipe {
        return Err(io::Error::other("'-' requires piped or redirected input"));
    }
    if pipe && args.file.is_some() && !explicit_stdin {
        return Err(io::Error::other(
            "Choose either stdin ('-') or a file, not both",
        ));
    }
    let mut pipe_session = if pipe { Some(Session::start()?) } else { None };
    let _pipe_exit = pipe_session.as_ref().map(|_| {
        shutdown::guard_panics();
        shutdown::OnExit
    });
    let reference = if let Some(path) = args.diff.first() {
        if !path.is_file() || !args.diff[1].is_file() {
            return Err(io::Error::other(
                "--diff requires two existing regular files",
            ));
        }
        if std::fs::canonicalize(path)? == std::fs::canonicalize(&args.diff[1])? {
            return Err(io::Error::other(
                "Reference and modified file must be different files",
            ));
        }
        if std::fs::metadata(path)?.len() > tapp_ui::renderers::diff::MAX_BYTES as u64
            || std::fs::metadata(&args.diff[1])?.len() > tapp_ui::renderers::diff::MAX_BYTES as u64
        {
            None
        } else {
            Some(Buffer::open(path)?)
        }
    } else {
        None
    };
    let mut recovery_format = args.format.clone();
    let buffer = match args.file {
        _ if !args.diff.is_empty() => Buffer::open(&args.diff[1])?,
        _ if pipe => piped::read(pipe_session.as_mut().unwrap())?,
        Some(path) if path.is_dir() => {
            std::env::set_current_dir(path)?;
            Buffer::new(DIRECTORY_NOTICE)
        }
        Some(path) => Buffer::open(&path)?,
        None => Buffer::new(""),
    };
    let mut editor = if let Some(format) = args.format {
        Editor::with_format(buffer, &format).map_err(io::Error::other)?
    } else {
        Editor::new(buffer)
    };
    if let Some(reference) = &reference {
        if let Err(reason) = editor.compare_with(reference) {
            editor.notify_warning(reason);
        }
    } else if !args.diff.is_empty() {
        editor.notify_warning("Diff disabled: file exceeds 2 MiB; showing the modified file");
    }
    editor.file_navigation = true;
    editor.set_copy_on_selection(!args.no_copy_on_selection);
    let preferences = std::env::home_dir()
        .filter(|home| home.is_absolute())
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Cannot locate user home directory"))
        .and_then(|home| load_preferences(&mut editor, &home));
    if let Err(error) = preferences {
        editor.notify_error(format!("Preferences: {error}"));
    }
    editor.mode = if args.view || !args.diff.is_empty() {
        Mode::View
    } else {
        Mode::Edit
    };
    editor.monochrome = args.monochrome;
    if args.no_wrap {
        editor.wrap = false;
    }
    if args.line_numbers {
        editor.line_numbers = true;
    }
    if args.limit_width {
        editor.limit_width = true;
    }
    editor.options_key = match args.options_key.as_str() {
        "ctrl-space" => event::KeyEvent::new(KeyCode::Char(' '), event::KeyModifiers::CONTROL),
        "f2" => event::KeyEvent::new(KeyCode::F(2), event::KeyModifiers::NONE),
        _ => editor.options_key,
    };
    let line = args
        .line
        .saturating_sub(1)
        .min(editor.buffer.text.len_lines() - 1);
    let source_line = editor.buffer.text.line(line);
    let content_chars = tapp_ui::renderers::wrap::line_content_chars(source_line);
    let col = args.column.saturating_sub(1).min(content_chars);
    editor.buffer.cursors[0] =
        editio::buffer::Cursor::at(editor.buffer.text.line_to_char(line) + col);
    let mut indentation_config = editorconfig::State::default();
    indentation_config.sync(&mut editor);
    let mut clipboard = arboard::Clipboard::new().ok();
    let mut session = match pipe_session {
        Some(session) => session,
        None => Session::start()?,
    };
    let _exit = shutdown::OnExit;
    shutdown::guard_panics();
    let (events, wake) = events::Events::start()?;
    let shutdown = shutdown::Shutdown::install(wake.clone())?;
    let candidate =
        if !pipe && (editor.buffer.path.is_some() || editor.buffer.text.len_chars() == 0) {
            recovery::discover(editor.buffer.path.as_deref())?
        } else {
            None
        };
    let Some((candidate, recovered)) = recover_document(
        &mut editor,
        &mut session,
        &events,
        &shutdown,
        candidate,
        &mut recovery_format,
    )?
    else {
        return Ok(ExitCode::FAILURE);
    };
    let mut recovery = recovery::Recovery::new(candidate)?;
    recovery.wake = Some(wake.clone());
    let mut cleanup = recovery::Cleanup::default();
    cleanup.wake = Some(wake.clone());
    let mut navigation = file_ui::Navigation::new(wake.clone())?;
    if let Err(error) = navigation.record(editor.buffer.path.as_deref(), false) {
        editor.notify_error(format!("History: {error}"));
    }
    let mut pending_document: Option<(Editor, file_ui::Open)> = None;
    recovery.comparing = recovered;
    let mut palette_requested =
        !editor.monochrome && tapp_ui::terminal::request_selection_background().unwrap_or(false);
    let result = (|| -> io::Result<ExitCode> {
        loop {
            indentation_config.sync(&mut editor);
            session.terminal.draw(|f| {
                editor.draw(f, f.area());
                navigation.draw(f, f.area(), &editor);
            })?;
            let event = loop {
                if shutdown.requested() {
                    shutdown::arm_exit();
                    recovery.checkpoint(&editor.buffer, recovery_format.as_deref())?;
                    return Ok(ExitCode::FAILURE);
                }
                if let Err(error) = recovery.observe(&editor.buffer, recovery_format.as_deref()) {
                    editor.notify_error(format!("Recovery: {error}"));
                    session.terminal.draw(|f| {
                        editor.draw(f, f.area());
                        navigation.draw(f, f.area(), &editor);
                    })?;
                }
                let editor_wait = editor.next_wakeup();
                cleanup.idle(editor_wait.is_none() && !recovery.busy());
                let wait = editor_wait
                    .into_iter()
                    .chain(recovery.next_wakeup())
                    .chain(cleanup.next_wakeup())
                    .chain(navigation.next_wakeup())
                    .min();
                let Some(e) = events.wait(wait)? else {
                    let background = editor.poll_background() | navigation.poll(&mut editor);
                    let clipboard_feedback = copy_selection(&mut editor, &mut clipboard);
                    if editor.poll_timers() || background || clipboard_feedback {
                        session.terminal.draw(|f| {
                            editor.draw(f, f.area());
                            navigation.draw(f, f.area(), &editor);
                        })?;
                    }
                    continue;
                };
                cleanup.idle(false);
                if matches!(e,Event::Mouse(m) if matches!(m.kind,event::MouseEventKind::Moved))
                    || matches!(e,Event::Key(k) if k.kind==KeyEventKind::Release)
                {
                    continue;
                }
                break e;
            };
            if let Event::BackgroundColor(r, g, b) = event {
                if palette_requested {
                    tapp_ui::theme::set_terminal_background(r, g, b);
                    palette_requested = false;
                }
                continue;
            }
            let event = if navigation.accepts_text()
                && matches!(&event, Event::Key(k) if tapp_ui::focus::is_paste(*k))
            {
                match clipboard.as_mut().map(|c| c.get_text()) {
                    Some(Ok(text)) => Event::Paste(text),
                    _ => {
                        editor.notify_error("System clipboard unavailable");
                        continue;
                    }
                }
            } else {
                event
            };
            let mut outcome = if let Some(text) = navigation.copy_url(&event) {
                Outcome::CopyRequested(text)
            } else if let Some(intent) = navigation.handle(&event, &mut editor) {
                navigation_intent(intent, &mut editor, &mut pending_document)
            } else {
                editor.handle(event)
            };
            match outcome {
                Outcome::FindFilesRequested => {
                    navigation.open(&mut editor);
                    outcome = Outcome::Handled;
                }
                Outcome::TagFileRequested { remove } => {
                    navigation.tag_prompt(remove);
                    outcome = Outcome::Handled;
                }
                Outcome::LinkRequested { ref url, action } => {
                    let intent = navigation.link(url.clone(), action, &mut editor);
                    outcome = navigation_intent(intent, &mut editor, &mut pending_document);
                }
                _ => {}
            }
            let mut switch = false;
            match outcome {
                Outcome::CopyRequested(text) => match write_clipboard(&mut clipboard, text) {
                    Ok(()) => editor.notify_success("Copied"),
                    Err(error) => editor.notify_error(format!("{error}; kept in editor clipboard")),
                },
                Outcome::PasteRequested => {
                    paste_clipboard(&mut editor, clipboard.as_mut().map(|c| c.get_text()));
                }
                Outcome::SaveRequested => {
                    let result = editor.buffer.save();
                    if result.is_ok() {
                        if recovery.comparing {
                            editor.end_comparison();
                            recovery.comparing = false;
                        }
                        if let Err(error) = recovery.clear() {
                            editor.notify_error(format!("Recovery cleanup: {error}"));
                        }
                    }
                    if result.is_ok()
                        && let Err(error) = navigation.record(editor.buffer.path.as_deref(), true)
                    {
                        editor.notify_error(format!("History: {error}"));
                    }
                    let saved = editor.save_finished(result);
                    switch = saved == Outcome::DocumentSwitchReady;
                    if saved == Outcome::QuitRequested {
                        shutdown::arm_exit();
                        return Ok(ExitCode::SUCCESS);
                    }
                }
                Outcome::DeleteFileRequested => match editor.buffer.delete_file() {
                    Ok(()) => {
                        editor.notify_success("File deleted; text kept as untitled");
                        if let Err(error) =
                            recovery.checkpoint(&editor.buffer, recovery_format.as_deref())
                        {
                            editor.notify_error(format!("Recovery: {error}"));
                        }
                    }
                    Err(error) => editor.notify_error(error.to_string()),
                },
                Outcome::RenameRequested(path) => {
                    let old = editor
                        .buffer
                        .path
                        .as_ref()
                        .and_then(|p| std::fs::canonicalize(p).ok());
                    match editor.buffer.rename_to(&path) {
                        Ok(()) => {
                            if let (Some(old), Some(new)) = (
                                old,
                                editor
                                    .buffer
                                    .path
                                    .as_ref()
                                    .and_then(|p| std::fs::canonicalize(p).ok()),
                            ) && let Err(error) = navigation.rename(&old, &new)
                            {
                                editor.notify_error(format!("Tags/history: {error}"));
                            }
                            editor.prompt = None;
                            editor.notify_success("Renamed");
                            if let Err(error) =
                                recovery.checkpoint(&editor.buffer, recovery_format.as_deref())
                            {
                                editor.notify_error(format!("Recovery: {error}"));
                            }
                        }
                        Err(error) => editor.notify_error(error.to_string()),
                    }
                }
                Outcome::DocumentSwitchReady => {
                    switch = true;
                }
                Outcome::QuitRequested => {
                    shutdown::arm_exit();
                    recovery.clear()?;
                    return Ok(ExitCode::SUCCESS);
                }
                Outcome::AbortRequested => {
                    shutdown::arm_exit();
                    recovery.clear()?;
                    return Ok(ExitCode::FAILURE);
                }
                Outcome::FocusReleased(_) => {
                    editor.message.clear();
                }
                _ => {}
            }
            if switch {
                if let Some((mut next, target)) = pending_document.take() {
                    let prepared = (|| -> io::Result<_> {
                        let candidate = recovery::discover(next.buffer.path.as_deref())?;
                        let mut format = None;
                        let Some((candidate, comparing)) = recover_document(
                            &mut next,
                            &mut session,
                            &events,
                            &shutdown,
                            candidate,
                            &mut format,
                        )?
                        else {
                            return Ok(None);
                        };
                        let mut next_recovery = recovery::Recovery::new(candidate)?;
                        next_recovery.wake = Some(wake.clone());
                        next_recovery.comparing = comparing;
                        Ok(Some((next_recovery, format)))
                    })();
                    if let Err(error) = &prepared {
                        editor.notify_error(format!("Cannot open document: {error}"));
                    }
                    if let Ok(Some((next_recovery, format))) = prepared {
                        recovery.clear()?;
                        recovery.finish()?;
                        editor = next;
                        recovery = next_recovery;
                        recovery_format = format;
                        indentation_config = editorconfig::State::default();
                        jump_to(&mut editor, &target);
                        if let Err(error) = navigation.record(editor.buffer.path.as_deref(), false)
                        {
                            editor.notify_error(format!("History: {error}"));
                        }
                    }
                }
            } else if !editor.document_switch_pending() {
                pending_document = None;
            }
            navigation.after_event(&editor);
            copy_selection(&mut editor, &mut clipboard);
        }
    })();
    shutdown::arm_exit();
    if result.is_err()
        && let Err(error) = recovery.checkpoint(&editor.buffer, recovery_format.as_deref())
    {
        let _ = std::io::Write::write_fmt(
            &mut io::stderr(),
            format_args!("editio: recovery: {error}\n"),
        );
    }
    let checkpoint = recovery.finish();
    shutdown.complete();
    checkpoint?;
    result
}

fn parse_format(value: &str) -> Result<String, String> {
    let value = value.trim().to_lowercase();
    if tapp_ui::renderers::syntax::language(&tapp_ui::renderers::syntax::FENCE_SYNTAXES, &value)
        .is_some()
    {
        Ok(value)
    } else {
        Err(format!(
            "Unknown format '{value}'; use a Markdown code-fence language name or alias"
        ))
    }
}

fn paste_clipboard<E: std::fmt::Display>(editor: &mut Editor, result: Option<Result<String, E>>) {
    match result {
        Some(Ok(text)) => {
            editor.paste(&text);
        }
        Some(Err(error)) => editor.notify_error(format!("Cannot read clipboard: {error}")),
        None if !editor.internal_clipboard().is_empty() => {
            let text = editor.internal_clipboard().to_owned();
            editor.paste(&text);
        }
        None => editor.notify_error("System clipboard unavailable"),
    }
}

fn copy_selection(editor: &mut Editor, clipboard: &mut Option<arboard::Clipboard>) -> bool {
    if let Some(text) = editor.poll_selection_copy() {
        let result = write_clipboard(clipboard, text);
        if let Err(error) = result {
            editor.notify_error(format!("{error}; kept in editor clipboard"));
            return true;
        }
        editor.notify_success("Copied");
        return true;
    }
    false
}

fn write_clipboard(clipboard: &mut Option<arboard::Clipboard>, text: String) -> Result<(), String> {
    clipboard
        .as_mut()
        .ok_or_else(|| "System clipboard unavailable".to_string())
        .and_then(|c| c.set_text(text).map_err(|e| e.to_string()))
}

fn load_preferences(editor: &mut Editor, home: &std::path::Path) -> io::Result<()> {
    editor.enable_preferences(home.join(".config/editio/config.json"))
}

fn recover_document(
    editor: &mut Editor,
    session: &mut Session,
    events: &events::Events,
    shutdown: &shutdown::Shutdown,
    mut candidate: Option<recovery::Candidate>,
    recovery_format: &mut Option<String>,
) -> io::Result<Option<(Option<recovery::Candidate>, bool)>> {
    let mut recovered = false;
    if let Some(draft) = &candidate {
        let mut prompt = recovery_prompt::Prompt::new(draft.label());
        loop {
            session
                .terminal
                .draw(|f| prompt.draw(f, editor.monochrome, editor.theme))?;
            if shutdown.requested() {
                return Ok(None);
            }
            let Some(event) = events.wait(None)? else {
                continue;
            };
            match prompt.handle(&event) {
                Some(recovery_prompt::Choice::Quit) => return Ok(None),
                Some(recovery_prompt::Choice::Disk) => {
                    if let Some(path) = &editor.buffer.path {
                        editor.buffer = Buffer::open(path)?;
                    }
                    draft.discard()?;
                    candidate = None;
                    break;
                }
                Some(recovery_prompt::Choice::Recover) => {
                    let text = draft.load()?;
                    if let Some(path) = &editor.buffer.path {
                        editor.buffer = Buffer::open(path)?;
                    }
                    let mut current = Buffer::new("");
                    current.text = editor.buffer.text.clone();
                    current.path = editor.buffer.path.clone();
                    draft.restore(&mut editor.buffer, text);
                    if recovery_format.is_none() {
                        *recovery_format = draft.meta.format.clone();
                    }
                    if let Some(format) = recovery_format.as_ref() {
                        editor.set_format(format).map_err(io::Error::other)?;
                    }
                    if let Err(reason) = editor.compare_with(&current) {
                        editor.notify_warning(format!("Draft recovered. {reason}"));
                    }
                    editor.mode = Mode::View;
                    recovered = true;
                    break;
                }
                None => {}
            }
        }
    }
    Ok(Some((candidate, recovered)))
}

fn navigation_intent(
    intent: file_ui::Intent,
    editor: &mut Editor,
    pending: &mut Option<(Editor, file_ui::Open)>,
) -> Outcome {
    match intent {
        file_ui::Intent::None => Outcome::Handled,
        file_ui::Intent::Copy(text) => Outcome::CopyRequested(text),
        file_ui::Intent::FindCurrent(query) => {
            editor.open_find_query(query);
            Outcome::Handled
        }
        file_ui::Intent::Open(target) => {
            let result = (|| -> io::Result<Option<Editor>> {
                let path = std::fs::canonicalize(&target.path)?;
                if !path.is_file() {
                    return Err(io::Error::other("Choose a regular file"));
                }
                if editor
                    .buffer
                    .path
                    .as_ref()
                    .and_then(|p| std::fs::canonicalize(p).ok())
                    .as_ref()
                    == Some(&path)
                {
                    jump_to(editor, &target);
                    return Ok(None);
                }
                let mut next = Editor::new(Buffer::open(&path)?);
                next.file_navigation = true;
                next.mode = Mode::View;
                next.set_copy_on_selection(editor.copy_on_selection());
                next.monochrome = editor.monochrome;
                next.options_key = editor.options_key;
                next.enable_preferences(files::prefs_path()?)?;
                Ok(Some(next))
            })();
            match result {
                Ok(Some(next)) => {
                    *pending = Some((next, target));
                    editor.request_document_switch()
                }
                Ok(None) => Outcome::Handled,
                Err(error) => {
                    editor.notify_error(error.to_string());
                    Outcome::Handled
                }
            }
        }
    }
}
fn jump_to(editor: &mut Editor, target: &file_ui::Open) {
    if let Some(anchor) = &target.anchor {
        if let Err(error) = editor.navigate_anchor(anchor) {
            editor.notify_warning(error);
        }
    } else {
        let line = target.line.min(editor.buffer.text.len_lines() - 1);
        let pos = editor.buffer.text.line_to_char(line)
            + target
                .column
                .min(tapp_ui::renderers::wrap::line_content_chars(
                    editor.buffer.text.line(line),
                ));
        editor.navigation_jump(pos);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_navigation_target_preserves_unsaved_document() {
        let dir = tempfile::tempdir().unwrap();
        let mut editor = Editor::new(Buffer::new("original"));
        editor.buffer.insert("unsaved");
        let before = editor.buffer.text.to_string();
        let mut pending = None;
        let outcome = navigation_intent(
            file_ui::Intent::Open(file_ui::Open {
                path: dir.path().join("missing.md"),
                line: 0,
                column: 0,
                anchor: None,
            }),
            &mut editor,
            &mut pending,
        );
        assert!(matches!(outcome, Outcome::Handled));
        assert!(pending.is_none());
        assert_eq!(editor.buffer.text.to_string(), before);
        assert!(!editor.document_switch_pending());
    }

    #[test]
    fn format_flag_validates_aliases_without_changing_default_invocation() {
        for alias in ["MD", "C++", "csharp", "yml", "plaintext", "csv", "tsv"] {
            assert!(Args::try_parse_from(["editio", "--format", alias, "-"]).is_ok());
        }
        assert_eq!(
            Args::try_parse_from(["editio", "--format", "no-such-format"])
                .err()
                .unwrap()
                .kind(),
            clap::error::ErrorKind::ValueValidation
        );
        let empty = Args::try_parse_from(["editio"]).unwrap();
        assert!(empty.file.is_none() && empty.format.is_none());
    }
    #[test]
    fn home_preferences_use_the_canonical_json_path() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("user");
        let path = home.join(".config/editio/config.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = r#"{"version":1,"theme":"tokyo-night-omarchy","file_types":{"plain text":{"wrap":false,"line_numbers":true,"limit_120":false}}}"#;
        std::fs::write(&path, original).unwrap();
        let mut e = Editor::new(Buffer::new("hello"));
        load_preferences(&mut e, &home).unwrap();
        assert_eq!(e.theme, tapp_ui::theme::Theme::TokyoNightOmarchy);
        assert!(!e.wrap && e.line_numbers);
        assert_eq!(std::fs::read_to_string(path).unwrap(), original);
    }

    #[test]
    fn view_alias_preserves_default_edit_and_version_flag() {
        assert!(!Args::try_parse_from(["editio"]).unwrap().view);
        for flag in ["-v", "--view"] {
            let args = Args::try_parse_from(["editio", flag, "example.md"]).unwrap();
            assert!(args.view);
            assert_eq!(args.file, Some(PathBuf::from("example.md")));
        }
        assert_eq!(
            Args::try_parse_from(["editio", "-V"]).err().unwrap().kind(),
            clap::error::ErrorKind::DisplayVersion
        );
    }
    #[test]
    fn failed_or_empty_clipboard_preserves_selected_text() {
        for result in [Some(Err("unavailable")), None, Some(Ok(String::new()))] {
            let mut editor = Editor::new(Buffer::new("keep this"));
            editor.mode = Mode::Edit;
            editor.act(editio::Action::SelectAll);
            paste_clipboard(&mut editor, result);
            assert_eq!(editor.buffer.text.to_string(), "keep this");
            assert!(!editor.buffer.dirty());
        }
    }
}
