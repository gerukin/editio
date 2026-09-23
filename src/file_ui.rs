//! File discovery reuses shared search, palette, input, path and modal primitives.
use crate::{
    events::Wake,
    files::{self, Catalog, Hit, Scope},
    links::{self, Completion, Target},
};
use crossterm::event::{Event, KeyCode as K, KeyEventKind, KeyModifiers as M};
use editio::Editor;
use ratatui::{
    prelude::*,
    widgets::{Paragraph, Wrap},
};
use std::{
    io,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use tapp_ui::{
    background::Latest,
    focus::Outcome as UiOutcome,
    input::Input,
    modal,
    palette::{Command, Palette},
    search::{SearchDialog, SearchEvent},
    theme::Role,
};
#[derive(Clone)]
pub struct Open {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub anchor: Option<String>,
}
#[derive(Default)]
pub enum Intent {
    #[default]
    None,
    Open(Open),
    Copy(String),
    FindCurrent(String),
}
enum Job {
    Search(files::Request),
    Complete(u64, Completion),
}
enum Reply {
    Search(u64, Result<files::Results, String>),
    Complete(u64, Completion, Vec<String>),
}
#[derive(Clone)]
enum Choice {
    Scope(Scope),
    Mode(usize),
    Content,
    GitIgnore,
    Clear,
    Case,
    Fuzzy,
    Wildcards,
    Glob,
    SaveGlob,
    Tag(bool),
}
enum Field {
    Glob,
    Tag(bool),
}
struct Menu {
    palette: Palette,
    choices: Vec<Choice>,
}
impl Menu {
    fn rank_picker(&mut self) {
        let query = self.palette.query().to_lowercase();
        let mut commands: Vec<_> = self
            .choices
            .iter()
            .enumerate()
            .filter_map(|(id, choice)| {
                let Choice::Scope(Scope::Tag(label) | Scope::Glob(label)) = choice else {
                    return None;
                };
                Some(Command {
                    id,
                    label: label.clone(),
                    shortcut: String::new(),
                })
            })
            .collect();
        if !query.is_empty() {
            commands.sort_by_cached_key(|command| {
                let label = command.label.to_lowercase();
                let start = label.find(&query).unwrap_or(usize::MAX);
                let rank = if label == query {
                    0
                } else if start == 0 {
                    1
                } else {
                    2
                };
                (rank, start, command.id)
            });
        }
        self.palette.set_commands(commands);
    }
}
struct Suggestions {
    context: Completion,
    values: Vec<String>,
    selected: usize,
}
struct Url {
    text: String,
    target: Option<Target>,
    selected: usize,
    scroll: u16,
    buttons: Vec<(usize, Rect)>,
}
pub struct Navigation {
    pub catalog: Catalog,
    prefs: PathBuf,
    wake: Wake,
    worker: Option<Latest<Job>>,
    receiver: Arc<Mutex<Option<Reply>>>,
    generation: Arc<AtomicU64>,
    pub finder: Option<SearchDialog<usize>>,
    hits: Vec<Hit>,
    scope: Scope,
    content: bool,
    respect_gitignore: bool,
    options: tapp_ui::text::search::SearchOptions,
    menu: Option<Menu>,
    picker: Option<(usize, Menu)>,
    field: Option<(Field, Input)>,
    url: Option<Url>,
    suggestions: Option<Suggestions>,
    deadline: Option<Instant>,
    completion_due: Option<Instant>,
    last_completion: Option<(u64, usize)>,
    base: PathBuf,
    summary: String,
    saved_query: String,
}
impl Navigation {
    pub fn new(wake: Wake) -> io::Result<Self> {
        Self::with_preferences(files::prefs_path()?, wake)
    }
    fn with_preferences(prefs: PathBuf, wake: Wake) -> io::Result<Self> {
        let catalog = Catalog::load(&prefs).unwrap_or_default();
        Ok(Self {
            catalog,
            prefs,
            wake,
            worker: None,
            receiver: Arc::new(Mutex::new(None)),
            generation: Arc::new(AtomicU64::new(0)),
            finder: None,
            hits: vec![],
            scope: Scope::Tag(files::VIEWED.into()),
            content: false,
            respect_gitignore: true,
            options: Default::default(),
            menu: None,
            picker: None,
            field: None,
            url: None,
            suggestions: None,
            deadline: None,
            completion_due: None,
            last_completion: None,
            base: PathBuf::new(),
            summary: String::new(),
            saved_query: String::new(),
        })
    }
    pub fn record(&mut self, path: Option<&Path>, modified: bool) -> io::Result<()> {
        if let Some(path) = path.filter(|p| p.is_file()) {
            self.catalog = Catalog::update(&self.prefs, |c| c.record(path, modified))?;
        }
        Ok(())
    }
    pub fn rename(&mut self, old: &Path, new: &Path) -> io::Result<()> {
        self.catalog = Catalog::update(&self.prefs, |c| {
            c.rename(old, new);
            Ok(())
        })?;
        Ok(())
    }
    fn worker(&mut self) -> &Latest<Job> {
        if self.worker.is_none() {
            let replies = self.receiver.clone();
            let wake = self.wake.clone();
            let generation = self.generation.clone();
            self.worker = Some(Latest::spawn(move |job| {
                let reply = match job {
                    Job::Search(r) => Reply::Search(r.generation, files::search(&r, &generation)),
                    Job::Complete(g, c) => {
                        let values = links::complete(&c).unwrap_or_default();
                        Reply::Complete(g, c, values)
                    }
                };
                *replies.lock().unwrap_or_else(|e| e.into_inner()) = Some(reply);
                wake.notify();
            }));
        }
        self.worker.as_ref().unwrap()
    }
    fn reload_catalog(&mut self, editor: &mut Editor) {
        match Catalog::load(&self.prefs) {
            Ok(catalog) => self.catalog = catalog,
            Err(error) => editor.notify_error(format!("File catalog: {error}")),
        }
    }
    fn recent_scope(&self) -> bool {
        matches!(&self.scope, Scope::Tag(tag) if tag == files::VIEWED || tag == files::MODIFIED)
    }
    pub fn open(&mut self, editor: &mut Editor) {
        if !self.recent_scope() {
            self.reload_catalog(editor);
        }
        self.open_results(editor);
    }
    fn open_results(&mut self, editor: &mut Editor) {
        if self.recent_scope() {
            self.reload_catalog(editor);
        }
        self.completion_due = None;
        editor.prompt = None;
        self.base = files::base(editor.buffer.path.as_deref());

        let dialog = self.finder.get_or_insert_with(|| {
            let mut d = SearchDialog::default();
            d.accept_empty_query = true;
            d.limit = files::LIMIT;
            d
        });
        dialog.focus.active = true;
        dialog.set_query(self.saved_query.clone());
        self.suggestions = None;
        self.schedule();
    }
    fn schedule(&mut self) {
        self.generation.fetch_add(1, Ordering::Relaxed);
        if let Some(d) = &mut self.finder {
            d.invalidate();
        }
        self.deadline = Some(Instant::now() + Duration::from_millis(100));
    }
    pub fn accepts_text(&self) -> bool {
        self.url.is_none()
            && (self.finder.is_some()
                || self.menu.is_some()
                || self.picker.is_some()
                || self.field.is_some())
    }
    pub fn visible(&self) -> bool {
        self.finder.is_some()
            || self.menu.is_some()
            || self.picker.is_some()
            || self.field.is_some()
            || self.url.is_some()
    }
    pub fn next_wakeup(&self) -> Option<Duration> {
        self.deadline
            .into_iter()
            .chain(self.completion_due)
            .min()
            .map(|t| t.saturating_duration_since(Instant::now()))
    }
    pub fn cancel(&mut self) {
        if let Some(dialog) = self.finder.take() {
            self.saved_query = dialog.input.value().to_owned();
        }
        self.menu = None;
        self.picker = None;
        self.field = None;
        self.url = None;
        self.suggestions = None;
        self.deadline = None;
        self.completion_due = None;
        self.generation.fetch_add(1, Ordering::Relaxed);
        if let Some(w) = &self.worker {
            w.cancel_pending();
        }
    }
    pub fn poll(&mut self, editor: &mut Editor) -> bool {
        let mut changed = false;
        if self.deadline.is_some_and(|t| t <= Instant::now()) {
            self.deadline = None;
            if let Some(d) = &self.finder {
                let paths = match &self.scope {
                    Scope::Tag(tag) => self.catalog.paths(tag),
                    _ => vec![],
                };
                let request = files::Request {
                    generation: self.generation.load(Ordering::Relaxed),
                    base: self.base.clone(),
                    scope: self.scope.clone(),
                    paths,
                    activity: match &self.scope {
                        Scope::Tag(tag) if tag == files::VIEWED => self.catalog.viewed.clone(),
                        Scope::Tag(tag) if tag == files::MODIFIED => self.catalog.modified.clone(),
                        _ => Default::default(),
                    },
                    query: d.input.value().into(),
                    content: self.content,
                    respect_gitignore: self.respect_gitignore,
                    options: self.options,
                };
                self.worker().submit(Job::Search(request));
            }
        }
        if self.completion_due.is_some_and(|t| t <= Instant::now()) {
            self.completion_due = None;
            if let Some(c) = links::completion(editor) {
                let generation = self.generation.fetch_add(1, Ordering::Relaxed) + 1;
                self.worker().submit(Job::Complete(generation, c));
            }
        }
        while let Some(reply) = {
            self.receiver
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .take()
        } {
            match reply {
                Reply::Search(g, result) if g == self.generation.load(Ordering::Relaxed) => {
                    if let Some(d) = &mut self.finder {
                        match result {
                            Ok(result) => {
                                self.summary = format!(
                                    "{}{} results{}",
                                    result.hits.len(),
                                    if result.limited { "+" } else { "" },
                                    if result.limited {
                                        " · refine filter"
                                    } else {
                                        ""
                                    }
                                );
                                if !self.respect_gitignore {
                                    self.summary.push_str(" · incl. ignored");
                                }
                                if let Some(reason) = result.cutoff {
                                    self.summary.push_str(&format!(" · {reason}"));
                                }
                                if result.skipped > 0 {
                                    self.summary
                                        .push_str(&format!(" · {} skipped", result.skipped));
                                }
                                self.hits = result.hits;
                                d.viewport.set_heights(
                                    self.hits
                                        .iter()
                                        .map(|h| if h.snippet.is_empty() { 2 } else { 3 }),
                                );
                                d.complete(
                                    d.revision(),
                                    (0..self.hits.len()).collect(),
                                    result.limited,
                                );
                            }
                            Err(e) => {
                                d.fail(d.revision(), e);
                            }
                        }
                        changed = true;
                    }
                }
                Reply::Complete(g, context, values)
                    if g == self.generation.load(Ordering::Relaxed)
                        && !values.is_empty()
                        && links::completion(editor).is_some_and(|c| {
                            c.range == context.range && c.prefix == context.prefix
                        }) =>
                {
                    self.suggestions = Some(Suggestions {
                        context,
                        values,
                        selected: 0,
                    });
                    changed = true;
                }
                _ => {}
            }
        }
        changed
    }
    pub fn after_event(&mut self, editor: &Editor) {
        if self.visible() {
            return;
        }
        let key = (
            editor.buffer.revision(),
            editor.buffer.cursors.first().map(|c| c.head).unwrap_or(0),
        );
        if self.last_completion != Some(key) {
            self.last_completion = Some(key);
            self.suggestions = None;
            self.generation.fetch_add(1, Ordering::Relaxed);
            self.completion_due =
                links::completion(editor).map(|_| Instant::now() + Duration::from_millis(120));
        }
    }
    fn commands_key(editor: &Editor, key: crossterm::event::KeyEvent) -> bool {
        key == editor.options_key
            || key == editor.palette_key
            || (key.code == K::Char('k') && key.modifiers == M::CONTROL)
            || (matches!(key.code, K::Char('p' | 'P')) && key.modifiers == M::CONTROL | M::SHIFT)
    }
    fn current(editor: &Editor) -> bool {
        editor
            .prompt
            .as_ref()
            .is_some_and(|p| matches!(p.kind, editio::PromptKind::Find | editio::PromptKind::Fuzzy))
    }
    fn mode(&self, editor: &Editor) -> usize {
        if Self::current(editor) {
            0
        } else if let Some((mode, _)) = &self.picker {
            *mode
        } else {
            match &self.scope {
                Scope::Tag(t) if t == files::VIEWED => 1,
                Scope::Tag(t) if t == files::MODIFIED => 2,
                Scope::Tag(_) => 3,
                Scope::Glob(_) => 4,
            }
        }
    }
    fn retain_query(&mut self, editor: &Editor) {
        if Self::current(editor) {
            self.options = editor.search_options;
            self.saved_query = editor.prompt.as_ref().unwrap().value.clone();
        } else if let Some(d) = &self.finder {
            self.saved_query = d.input.value().into();
        }
    }
    fn select_scope(&mut self, scope: Scope, editor: &mut Editor) {
        self.retain_query(editor);
        self.cancel();
        self.scope = scope;
        self.open_results(editor);
    }
    fn glob_patterns(&self) -> impl Iterator<Item = String> + '_ {
        const BUILTINS: [&str; 2] = ["*.md", "**/*.md"];
        BUILTINS.into_iter().map(str::to_owned).chain(
            self.catalog
                .globs
                .iter()
                .filter(|g| !BUILTINS.contains(&g.as_str()))
                .cloned(),
        )
    }
    fn show_picker(&mut self, mode: usize) {
        let choices: Vec<_> = if mode == 3 {
            self.catalog
                .tags
                .keys()
                .map(|t| (t.clone(), Choice::Scope(Scope::Tag(t.clone()))))
                .collect()
        } else {
            self.glob_patterns()
                .map(|g| (g.clone(), Choice::Scope(Scope::Glob(g))))
                .collect()
        };
        let commands = choices
            .iter()
            .enumerate()
            .map(|(id, (label, _))| Command {
                id,
                label: label.clone(),
                shortcut: String::new(),
            })
            .collect();
        self.picker = Some((
            mode,
            Menu {
                palette: Palette::new(commands),
                choices: choices.into_iter().map(|(_, c)| c).collect(),
            },
        ));
    }
    fn change_mode(&mut self, mode: usize, editor: &mut Editor) -> Intent {
        self.retain_query(editor);
        self.cancel();
        if mode == 0 {
            editor.search_options = self.options;
            return Intent::FindCurrent(self.saved_query.clone());
        }
        editor.prompt = None;
        self.base = files::base(editor.buffer.path.as_deref());
        match mode {
            1 | 2 => {
                self.scope = Scope::Tag(
                    if mode == 1 {
                        files::VIEWED
                    } else {
                        files::MODIFIED
                    }
                    .into(),
                );
                self.open_results(editor);
            }
            _ => self.show_picker(mode),
        }
        Intent::None
    }
    fn choices(&mut self, editor: &Editor) {
        let current = Self::current(editor);
        let mode = self.mode(editor);
        let options = if current {
            editor.search_options
        } else {
            self.options
        };
        let mut choices = vec![
            (
                if options.case_sensitive {
                    "Use case-insensitive matching"
                } else {
                    "Use case-sensitive matching"
                }
                .into(),
                Choice::Case,
            ),
            (
                if options.fuzzy {
                    "Use exact matching"
                } else {
                    "Use fuzzy matching"
                }
                .into(),
                Choice::Fuzzy,
            ),
            (
                if options.wildcards {
                    "Disable wildcards"
                } else {
                    "Enable wildcards"
                }
                .into(),
                Choice::Wildcards,
            ),
            ("Clear search field".into(), Choice::Clear),
        ];
        if !current {
            choices.insert(
                0,
                (
                    (if self.respect_gitignore {
                        "Include Git-ignored files"
                    } else {
                        "Respect Git ignore rules"
                    })
                    .into(),
                    Choice::GitIgnore,
                ),
            );
        }
        let modes = [
            "Search current document",
            "Search recently viewed",
            "Search recently modified",
            "Choose a tag",
            "Choose a registered glob pattern",
        ];
        let mut scopes = vec![
            (
                format!("Tag: {}", files::VIEWED),
                Choice::Scope(Scope::Tag(files::VIEWED.into())),
            ),
            (
                format!("Tag: {}", files::MODIFIED),
                Choice::Scope(Scope::Tag(files::MODIFIED.into())),
            ),
        ];
        for tag in self.catalog.tags.keys() {
            scopes.push((
                format!("Tag: {tag}"),
                Choice::Scope(Scope::Tag(tag.clone())),
            ));
        }
        for glob in self.glob_patterns() {
            scopes.push((format!("Glob: {glob}"), Choice::Scope(Scope::Glob(glob))));
        }
        scopes.sort_by_key(|(_, c)| {
            !matches!(
                (mode, c),
                (1..=3, Choice::Scope(Scope::Tag(_))) | (4, Choice::Scope(Scope::Glob(_)))
            )
        });
        if !current && !self.content {
            std::mem::swap(&mut choices, &mut scopes);
        }
        choices.extend(scopes);
        if !current {
            choices.insert(
                0,
                (
                    (if self.content {
                        "Search filenames"
                    } else {
                        "Search file contents"
                    })
                    .into(),
                    Choice::Content,
                ),
            );
        }
        for (i, label) in modes.iter().enumerate() {
            if i != mode {
                choices.push(((*label).into(), Choice::Mode(i)));
            }
        }
        choices.push(("Enter temporary glob pattern".into(), Choice::Glob));
        if self.finder.is_some() && matches!(self.scope, Scope::Glob(_)) {
            choices.push(("Remember current glob pattern".into(), Choice::SaveGlob));
        }
        if editor.buffer.path.is_some() {
            choices.extend([
                ("Tag current file".into(), Choice::Tag(false)),
                ("Untag current file".into(), Choice::Tag(true)),
            ]);
        }
        let commands = choices
            .iter()
            .enumerate()
            .map(|(id, (label, choice))| Command {
                id,
                label: label.clone(),
                shortcut: if matches!(choice, Choice::Content) {
                    "Ctrl+L".into()
                } else if matches!(choice, Choice::Clear) {
                    "Ctrl+U".into()
                } else {
                    String::new()
                },
            })
            .collect();
        self.menu = Some(Menu {
            palette: Palette::new(commands),
            choices: choices.into_iter().map(|(_, c)| c).collect(),
        });
    }
    pub fn tag_prompt(&mut self, remove: bool) {
        self.completion_due = None;
        self.suggestions = None;
        self.field = Some((Field::Tag(remove), Input::default()));
    }
    pub fn link(&mut self, url: String, action: u8, editor: &mut Editor) -> Intent {
        let target = match links::resolve(&url, editor.buffer.path.as_deref()) {
            Ok(t) => t,
            Err(e) => {
                editor.notify_error(e.to_string());
                return Intent::None;
            }
        };
        match action {
            0 => {
                self.url = Some(Url {
                    text: url,
                    target: Some(target),
                    selected: 0,
                    scroll: 0,
                    buttons: vec![],
                });
                Intent::None
            }
            1 => {
                if let Err(e) = links::open(&target) {
                    editor.notify_error(e.to_string());
                }
                Intent::None
            }
            _ => match target {
                Target::Local { path, anchor } => Intent::Open(Open {
                    path,
                    anchor,
                    line: 0,
                    column: 0,
                }),
                _ => Intent::None,
            },
        }
    }
    pub fn handle(&mut self, event: &Event, editor: &mut Editor) -> Option<Intent> {
        if matches!(event,Event::Key(k) if k.kind==KeyEventKind::Release) {
            return self.visible().then_some(Intent::None);
        }
        if !self.visible()
            && editor.prompt.is_none()
            && editor.document_search_active()
            && matches!(event, Event::Key(k) if Self::commands_key(editor, *k))
        {
            editor.act(editio::Action::Find);
            self.choices(editor);
            return Some(Intent::None);
        }
        if self.field.is_none()
            && self.url.is_none()
            && (self.finder.is_some() || self.picker.is_some() || Self::current(editor))
            && let Event::Key(k) = event
        {
            if matches!(k.code, K::Tab | K::BackTab)
                && !k.modifiers.intersects(M::CONTROL | M::ALT | M::SUPER)
            {
                let backward = k.code == K::BackTab || k.modifiers.contains(M::SHIFT);
                return Some(self.change_mode(
                    (self.mode(editor) + if backward { 4 } else { 1 }) % 5,
                    editor,
                ));
            }
            if k.code == K::Char('l') && k.modifiers == M::CONTROL && !Self::current(editor) {
                self.content = !self.content;
                if self.finder.is_some() {
                    self.schedule();
                }
                if self.menu.is_some() {
                    self.choices(editor);
                }
                return Some(Intent::None);
            }
            if Self::commands_key(editor, *k) {
                if self.menu.is_some() {
                    self.menu = None;
                } else {
                    self.choices(editor);
                }
                return Some(Intent::None);
            }
        }
        if let Some(url) = &mut self.url {
            let chosen = match event {
                Event::Key(k) => match k.code {
                    K::Esc => {
                        self.url = None;
                        return Some(Intent::None);
                    }
                    K::Left | K::BackTab => {
                        url.selected = url.selected.saturating_sub(1);
                        None
                    }
                    K::Right | K::Tab => {
                        url.selected = (url.selected + 1)
                            % if matches!(url.target, Some(Target::Local { .. })) {
                                3
                            } else {
                                2
                            };
                        None
                    }
                    K::Up => {
                        url.scroll = url.scroll.saturating_sub(1);
                        None
                    }
                    K::Down => {
                        url.scroll = url.scroll.saturating_add(1);
                        None
                    }
                    K::Enter => Some(url.selected),
                    _ => None,
                },
                Event::Mouse(m)
                    if m.kind
                        == crossterm::event::MouseEventKind::Down(
                            crossterm::event::MouseButton::Left,
                        ) =>
                {
                    url.buttons
                        .iter()
                        .find(|(_, r)| r.contains((m.column, m.row).into()))
                        .map(|(i, _)| *i)
                }
                _ => None,
            };
            if let Some(i) = chosen {
                let url = self.url.take().unwrap();
                if i == 0 {
                    return Some(Intent::Copy(url.text));
                } else {
                    return Some(self.link(url.text, i as u8, editor));
                }
            }
            return Some(Intent::None);
        }
        if let Some((kind, input)) = &mut self.field {
            match input.handle(event) {
                UiOutcome::Submit => {
                    let value = input.value().trim().to_owned();
                    let kind = std::mem::replace(kind, Field::Glob);
                    self.field = None;
                    let result = match kind {
                        Field::Glob => files::pattern(&self.base, &value).map(|_| {
                            self.select_scope(Scope::Glob(value), editor);
                        }),
                        Field::Tag(remove) => editor
                            .buffer
                            .path
                            .clone()
                            .ok_or_else(|| io::Error::other("Save the document before tagging"))
                            .and_then(|path| {
                                Catalog::update(&self.prefs, |c| c.tag(&path, &value, remove))
                                    .map(|c| self.catalog = c)
                            }),
                    };
                    if let Err(e) = result {
                        editor.notify_error(e.to_string());
                    } else {
                        if let Some((mode, _)) = &self.picker {
                            let mode = *mode;
                            self.show_picker(mode);
                        }
                        editor.notify_success("Updated");
                    }
                }
                UiOutcome::FocusReleased(_) => self.field = None,
                _ => {}
            }
            return Some(Intent::None);
        }
        if let Some(menu) = &mut self.menu {
            match menu.palette.handle(event) {
                UiOutcome::Submit => {
                    let choice = menu.palette.selected_id().map(|i| menu.choices[i].clone());
                    self.menu = None;
                    match choice {
                        Some(Choice::Mode(mode)) => return Some(self.change_mode(mode, editor)),
                        Some(Choice::Scope(scope)) => self.select_scope(scope, editor),
                        Some(Choice::GitIgnore) => {
                            self.respect_gitignore = !self.respect_gitignore;
                            if self.finder.is_some() {
                                self.schedule();
                            }
                        }
                        Some(Choice::Content) => {
                            self.content = !self.content;
                            if self.finder.is_some() {
                                self.schedule();
                            }
                        }
                        Some(
                            choice @ (Choice::Case
                            | Choice::Fuzzy
                            | Choice::Wildcards
                            | Choice::Clear),
                        ) => {
                            if Self::current(editor) {
                                editor.act(match choice {
                                    Choice::Case => editio::Action::ToggleSearchCase,
                                    Choice::Fuzzy => editio::Action::ToggleSearchFuzzy,
                                    Choice::Wildcards => editio::Action::ToggleSearchWildcards,
                                    _ => editio::Action::ClearFind,
                                });
                            } else {
                                match choice {
                                    Choice::Case => {
                                        self.options.case_sensitive = !self.options.case_sensitive
                                    }
                                    Choice::Fuzzy => self.options.fuzzy = !self.options.fuzzy,
                                    Choice::Wildcards => {
                                        self.options.wildcards = !self.options.wildcards
                                    }
                                    _ => {
                                        if let Some(d) = &mut self.finder {
                                            d.set_query(String::new());
                                        }
                                        if let Some((_, picker)) = &mut self.picker {
                                            picker.palette.set_query(String::new());
                                        }
                                    }
                                }
                                if self.finder.is_some() {
                                    self.schedule();
                                }
                            }
                        }
                        Some(Choice::Glob) => self.field = Some((Field::Glob, Input::default())),
                        Some(Choice::Tag(remove)) => self.tag_prompt(remove),
                        Some(Choice::SaveGlob) => {
                            if let Scope::Glob(glob) = &self.scope {
                                match Catalog::update(&self.prefs, |c| {
                                    c.globs.insert(glob.clone());
                                    Ok(())
                                }) {
                                    Ok(c) => {
                                        self.catalog = c;
                                        editor.notify_success("Pattern saved");
                                    }
                                    Err(e) => editor.notify_error(e.to_string()),
                                }
                            }
                        }
                        _ => {}
                    }
                }
                UiOutcome::FocusReleased(_) => self.menu = None,
                _ => {}
            }
            return Some(Intent::None);
        }
        if let Some((_, picker)) = &mut self.picker {
            let old_query = picker.palette.query().to_owned();
            let outcome = picker.palette.handle(event);
            if picker.palette.query() != old_query {
                picker.rank_picker();
            }
            match outcome {
                UiOutcome::Submit => {
                    if let Some(Choice::Scope(scope)) = picker
                        .palette
                        .selected_id()
                        .map(|i| picker.choices[i].clone())
                    {
                        self.select_scope(scope, editor);
                    }
                }
                UiOutcome::FocusReleased(_) => self.cancel(),
                _ => {}
            }
            return Some(Intent::None);
        }
        if let Some(d) = &mut self.finder {
            match d.handle(event) {
                SearchEvent::Options => self.choices(editor),
                SearchEvent::Dismiss => {
                    let mode = self.mode(editor);
                    if mode >= 3 {
                        self.change_mode(mode, editor);
                    } else {
                        self.cancel();
                    }
                }

                SearchEvent::QueryChanged(_) => self.schedule(),
                SearchEvent::Accept(i) => {
                    if let Some(hit) = self.hits.get(i) {
                        let target = Open {
                            path: hit.path.clone(),
                            line: hit.line,
                            column: hit.column,
                            anchor: None,
                        };
                        self.cancel();
                        return Some(Intent::Open(target));
                    }
                }
                _ => {}
            }
            return Some(Intent::None);
        }
        if let Some(s) = &mut self.suggestions
            && let Event::Key(k) = event
            && k.modifiers.is_empty()
        {
            match k.code {
                K::Up => {
                    s.selected = (s.selected + s.values.len() - 1) % s.values.len();
                    return Some(Intent::None);
                }
                K::Down => {
                    s.selected = (s.selected + 1) % s.values.len();
                    return Some(Intent::None);
                }
                K::Esc => {
                    self.suggestions = None;
                    return Some(Intent::None);
                }
                K::Tab => {
                    let s = self.suggestions.take().unwrap();
                    if links::completion(editor)
                        .is_some_and(|c| c.range == s.context.range && c.prefix == s.context.prefix)
                    {
                        editor.buffer.cursors[0].anchor = s.context.range.start;
                        editor.buffer.cursors[0].head = s.context.range.end;
                        editor.paste(&s.values[s.selected]);
                    }
                    return Some(Intent::None);
                }
                _ => {}
            }
        }
        None
    }
    pub fn copy_url(&self, event: &Event) -> Option<String> {
        match event {
            Event::Key(k) if k.code == K::Char('c') && k.modifiers == M::CONTROL => {
                self.url.as_ref().map(|u| u.text.clone())
            }
            _ => None,
        }
    }
    pub fn draw(&mut self, f: &mut Frame, area: Rect, editor: &Editor) {
        // The editor reserves its last row for the document status bar.
        let area = Rect {
            height: area.height.saturating_sub(1),
            ..area
        };
        let mono = editor.monochrome;
        let theme = editor.theme;
        if self.menu.is_none()
            && let Some(d) = &mut self.finder
        {
            d.monochrome = mono;
            d.theme = theme;
            d.viewport.selection_style =
                Some(tapp_ui::table::RowSelection::for_theme(theme).style(mono));
            let scope = match &self.scope {
                Scope::Tag(s) | Scope::Glob(s) => s,
            };
            let title = format!(
                " Find files · {} · {} ",
                scope,
                if self.content { "content" } else { "names" }
            );
            let hits = &self.hits;
            let status = format!(
                "{} · {}{}{} · Ctrl+L: names/content",
                self.summary,
                if self.options.case_sensitive {
                    "Aa"
                } else {
                    "aa"
                },
                if self.options.fuzzy { " fuzzy" } else { "" },
                if self.options.wildcards { " *" } else { "" }
            );
            d.draw(f, area, &title, &status, |i, _, width| {
                let h = &hits[*i];
                let name = h.path.file_name().unwrap_or_default().to_string_lossy();
                let path = tapp_ui::path::display(&h.path);
                let path = shorten(&safe(&path), width as usize);
                let mut lines = vec![
                    Line::from(Span::styled(safe(&name), Role::Info.style(mono))),
                    Line::from(tapp_ui::path::spans(
                        &path,
                        Style::default(),
                        Role::Muted.style(mono),
                    )),
                ];
                if !h.snippet.is_empty() {
                    let text = safe(&h.snippet);
                    let start = text
                        .char_indices()
                        .nth(h.highlight.start)
                        .map(|(i, _)| i)
                        .unwrap_or(text.len());
                    let end = text
                        .char_indices()
                        .nth(h.highlight.end)
                        .map(|(i, _)| i)
                        .unwrap_or(text.len());
                    lines.push(Line::from(vec![
                        Span::raw(format!("{}: ", h.line + 1)),
                        Span::raw(text[..start].to_owned()),
                        Span::styled(text[start..end].to_owned(), Role::Search.style(mono).bold()),
                        Span::raw(text[end..].to_owned()),
                    ]));
                }
                Text::from(lines)
            });
        }
        if self.menu.is_none()
            && let Some((mode, picker)) = &mut self.picker
        {
            picker.palette.monochrome = mono;
            picker.palette.theme = theme;
            picker.palette.selection = tapp_ui::table::RowSelection::for_theme(theme);
            let title = if *mode == 3 {
                " Find · Choose tag "
            } else {
                " Find · Registered globs "
            };
            let footer = if picker.choices.is_empty() {
                if *mode == 3 {
                    "No tags yet · tag a file using Ctrl+K"
                } else {
                    "No saved patterns · enter and remember one in Ctrl+K"
                }
            } else {
                "Enter: search here · Tab/Shift+Tab: scope · Ctrl+L: names/content"
            };
            picker.palette.draw_search(f, area, title, footer);
            if picker.palette.match_count() == 0 {
                let rect = tapp_ui::layout::search_modal_rect(area);
                let inner = modal::block(title, " Esc ", modal::Category::Find, mono).inner(rect);
                let results = tapp_ui::search::regions(inner).results;
                f.render_widget(ratatui::widgets::Clear, results);
                f.render_widget(
                    Paragraph::new(if *mode == 3 {
                        "No matching user tags"
                    } else {
                        "No matching registered patterns"
                    })
                    .style(Role::Muted.style(mono)),
                    results,
                );
            }
        }
        if let Some(menu) = &mut self.menu {
            menu.palette.monochrome = mono;
            menu.palette.theme = theme;
            menu.palette.selection = tapp_ui::table::RowSelection::for_theme(theme);
            menu.palette
                .draw_search(f, area, " Find · Commands ", "Tab/Shift+Tab: scope");
        }
        if let Some((kind, input)) = &mut self.field {
            let title = match kind {
                Field::Glob => " Glob pattern ",
                Field::Tag(false) => " Tag file ",
                Field::Tag(true) => " Untag file ",
            };
            let rect = modal::content_rect(area, 70, 1);
            let inner = modal::draw_frame(
                f,
                rect,
                modal::block(title, " Esc ", modal::Category::Commands, mono),
            );
            input.monochrome = mono;
            input.draw(f, inner);
        }
        if let Some(url) = &mut self.url {
            let probe = modal::content_rect(area, 100, 1);
            let lines = Paragraph::new(url.text.as_str())
                .wrap(Wrap { trim: false })
                .line_count(probe.width.saturating_sub(4).max(1));
            let rect = modal::content_rect(area, 100, lines.saturating_add(2));

            let inner = modal::draw_frame(
                f,
                rect,
                modal::block(" Full URL ", " Esc ", modal::Category::Info, mono),
            );
            let spans =
                tapp_ui::path::spans(&url.text, Role::Info.style(mono), Role::Muted.style(mono));
            let p = Paragraph::new(Line::from(spans)).wrap(Wrap { trim: false });
            let height = inner.height.saturating_sub(2);
            url.scroll = url
                .scroll
                .min(p.line_count(inner.width).saturating_sub(height as usize) as u16);
            f.render_widget(p.scroll((url.scroll, 0)), Rect { height, ..inner });
            let mut buttons = vec![
                modal::Button {
                    label: "Copy (Ctrl+C)",
                    role: Role::Text,
                },
                modal::Button {
                    label: "Open default app",
                    role: Role::Command,
                },
            ];
            if matches!(url.target, Some(Target::Local { .. })) {
                buttons.push(modal::Button {
                    label: "Open in Editio",
                    role: Role::Command,
                });
            }
            url.buttons = modal::buttons(f, inner, &buttons, url.selected, mono);
        }
        if !self.visible()
            && let Some(s) = &self.suggestions
        {
            let rect = Rect::new(
                area.x,
                area.bottom().saturating_sub(7),
                area.width.min(70),
                area.height.min(6),
            );
            let inner = modal::draw_frame(
                f,
                rect,
                modal::block(
                    " Link completion · Tab ",
                    " Esc ",
                    modal::Category::Commands,
                    mono,
                ),
            );
            let first = s
                .selected
                .saturating_sub(inner.height.saturating_sub(1) as usize);
            for (row, value) in s
                .values
                .iter()
                .enumerate()
                .skip(first)
                .take(inner.height as usize)
            {
                let style = if row == s.selected {
                    Role::Command.style(mono).bold()
                } else {
                    Style::default()
                };
                f.render_widget(
                    Paragraph::new(value.as_str()).style(style),
                    Rect::new(inner.x, inner.y + (row - first) as u16, inner.width, 1),
                );
            }
        }
        if self.visible() || self.suggestions.is_some() {
            theme.apply(f.buffer_mut(), area);
        }
    }
}
fn shorten(text: &str, width: usize) -> String {
    let n = text.chars().count();
    if n <= width {
        text.into()
    } else {
        format!(
            "…{}",
            text.chars()
                .skip(n.saturating_sub(width.saturating_sub(1)))
                .collect::<String>()
        )
    }
}

fn safe(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { '�' } else { c })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;
    #[test]
    fn recent_views_reload_shared_history_and_ignore_git_rules_across_instances() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir(root.join(".git")).unwrap();
        std::fs::create_dir(root.join("ignored")).unwrap();
        std::fs::write(root.join(".gitignore"), "ignored/\n").unwrap();
        let file = root.join("ignored/note.md");
        std::fs::write(&file, "note").unwrap();
        let prefs = root.join("prefs.json");
        let mut first =
            Navigation::with_preferences(prefs.clone(), crate::events::test_wake()).unwrap();
        let mut second = Navigation::with_preferences(prefs, crate::events::test_wake()).unwrap();
        let mut untitled = Editor::new(editio::buffer::Buffer::new(""));
        let mut titled = Editor::new(editio::buffer::Buffer::open(&file).unwrap());
        first.record(Some(&file), false).unwrap();
        first.record(Some(&file), true).unwrap();
        let other = root.join("other.md");
        std::fs::write(&other, "other").unwrap();
        let file = std::fs::canonicalize(file).unwrap();
        let other = std::fs::canonicalize(other).unwrap();
        Catalog::update(&first.prefs, |catalog| {
            catalog.viewed.insert(file.clone(), 100);
            catalog.viewed.insert(other.clone(), 200);
            catalog.modified.insert(file.clone(), 200);
            catalog.modified.insert(other.clone(), 100);
            Ok(())
        })
        .unwrap();
        for mode in [1, 2] {
            for (ui, editor) in [(&mut first, &mut titled), (&mut second, &mut untitled)] {
                ui.change_mode(mode, editor);
                ui.deadline = Some(Instant::now());
                ui.poll(editor);
                let deadline = Instant::now() + Duration::from_secs(3);
                while ui.finder.as_ref().unwrap().loading {
                    assert!(Instant::now() < deadline);
                    ui.poll(editor);
                    std::thread::sleep(Duration::from_millis(1));
                }
                assert_eq!(ui.hits.len(), 2);
                assert_eq!(
                    ui.hits[0].path,
                    if mode == 1 {
                        other.clone()
                    } else {
                        file.clone()
                    }
                );
            }
        }
    }
    #[test]
    fn pickers_rank_exact_then_prefix_then_substring_and_restore_default_order() {
        let dir = tempfile::tempdir().unwrap();
        let mut ui =
            Navigation::with_preferences(dir.path().join("prefs.json"), crate::events::test_wake())
                .unwrap();
        for tag in ["a-work", "work", "workshop"] {
            ui.catalog.tags.insert(tag.into(), Default::default());
        }
        ui.show_picker(3);
        let picker = &mut ui.picker.as_mut().unwrap().1;
        picker.palette.set_query("work");
        picker.rank_picker();
        assert!(
            matches!(&picker.choices[picker.palette.selected_id().unwrap()], Choice::Scope(Scope::Tag(tag)) if tag == "work")
        );
        picker.palette.set_query("");
        picker.rank_picker();
        assert!(
            matches!(&picker.choices[picker.palette.selected_id().unwrap()], Choice::Scope(Scope::Tag(tag)) if tag == "a-work")
        );
    }
    #[test]
    fn file_find_and_contextual_commands_use_selected_theme_status_colors() {
        use ratatui::style::Modifier;
        use tapp_ui::theme::Theme;
        let dir = tempfile::tempdir().unwrap();
        for theme in [Theme::Terminal, Theme::TokyoNightOmarchy] {
            let mut ui = Navigation::with_preferences(
                dir.path().join("prefs.json"),
                crate::events::test_wake(),
            )
            .unwrap();
            let mut editor = Editor::new(editio::buffer::Buffer::new(""));
            editor.theme = theme;
            ui.open(&mut editor);
            for commands in [false, true] {
                if commands {
                    ui.choices(&editor);
                }
                let mut terminal =
                    ratatui::Terminal::new(ratatui::backend::TestBackend::new(40, 18)).unwrap();
                terminal
                    .draw(|frame| ui.draw(frame, frame.area(), &editor))
                    .unwrap();
                let rect = tapp_ui::layout::search_modal_rect(Rect::new(0, 0, 40, 17));
                let inner = modal::block("", "", modal::Category::Find, false).inner(rect);
                let footer = tapp_ui::search::regions(inner).footer;
                let style = theme.status_bar_style(Role::Search, false);
                for x in footer.x..footer.right() {
                    let cell = &terminal.backend().buffer()[(x, footer.y)];
                    assert_eq!(Some(cell.fg), style.fg);
                    assert_eq!(Some(cell.bg), style.bg);
                    assert!(!cell.modifier.intersects(Modifier::DIM | Modifier::REVERSED));
                }
            }
        }
    }
    #[test]
    fn tag_results_prepopulate_without_input_and_modal_fits_offset_small_viewports() {
        let d = tempfile::tempdir().unwrap();
        let file = d.path().join("note.md");
        std::fs::write(&file, "note").unwrap();
        let prefs = d.path().join("config.json");
        Catalog::update(&prefs, |c| c.tag(&file, "work", false)).unwrap();
        let mut ui = Navigation::with_preferences(prefs, crate::events::test_wake()).unwrap();
        let mut e = Editor::new(editio::buffer::Buffer::open(&file).unwrap());
        assert!(ui.worker.is_none());
        assert!(ui.next_wakeup().is_none());
        ui.scope = Scope::Tag("work".into());
        ui.open(&mut e);
        ui.deadline = Some(Instant::now());
        ui.poll(&mut e);
        let deadline = Instant::now() + Duration::from_secs(3);
        while ui.finder.as_ref().unwrap().loading {
            assert!(Instant::now() < deadline);
            ui.poll(&mut e);
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(ui.hits.len(), 1);
        for (width, height) in [(80, 24), (30, 10), (9, 4)] {
            let mut t =
                ratatui::Terminal::new(ratatui::backend::TestBackend::new(width + 4, height + 4))
                    .unwrap();
            t.draw(|f| ui.draw(f, Rect::new(2, 2, width, height), &e))
                .unwrap();
            assert_eq!(t.backend().buffer()[(0, 0)].symbol(), " ");
        }
        assert!(matches!(
            ui.handle(&Event::Key(KeyEvent::new(K::Enter, M::NONE)), &mut e),
            Some(Intent::Open(_))
        ));
        assert!(!ui.visible());
        assert!(ui.next_wakeup().is_none());
    }
    #[test]
    fn completion_acceptance_and_pasted_queries_never_edit_the_wrong_buffer() {
        let d = tempfile::tempdir().unwrap();
        let mut ui =
            Navigation::with_preferences(d.path().join("config.json"), crate::events::test_wake())
                .unwrap();
        let mut e = Editor::new(editio::buffer::Buffer::new(""));
        e.buffer.path = Some(d.path().join("index.md"));
        e.mode = editio::Mode::Edit;
        e.buffer.insert("猫 [note](no");
        let context = links::completion(&e).unwrap();
        ui.suggestions = Some(Suggestions {
            context,
            values: vec!["notes.md".into()],
            selected: 0,
        });
        ui.handle(&Event::Key(KeyEvent::new(K::Tab, M::NONE)), &mut e);
        assert_eq!(e.buffer.text.to_string(), "猫 [note](notes.md");
        let before = e.buffer.text.to_string();
        ui.open(&mut e);
        assert!(ui.accepts_text());
        ui.handle(&Event::Paste("paperclip".into()), &mut e);
        assert_eq!(ui.finder.as_ref().unwrap().input.value(), "paperclip");
        assert_eq!(e.buffer.text.to_string(), before);
    }
    #[test]
    fn find_modes_cycle_both_directions_with_query_and_underlying_mode_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let mut ui = Navigation::with_preferences(
            dir.path().join("config.json"),
            crate::events::test_wake(),
        )
        .unwrap();
        let mut e = Editor::new(editio::buffer::Buffer::new("paperclip"));
        e.mode = editio::Mode::View;
        e.open_find_query("paperclip".into());
        for (key, order) in [
            (K::Tab, vec![1, 2, 3, 4, 0]),
            (K::BackTab, vec![4, 3, 2, 1, 0]),
        ] {
            for _ in 0..3 {
                for &expected in &order {
                    let intent = ui
                        .handle(&Event::Key(KeyEvent::new(key, M::NONE)), &mut e)
                        .unwrap();
                    if let Intent::FindCurrent(query) = intent {
                        e.open_find_query(query);
                    }
                    assert_eq!(ui.mode(&e), expected);
                    let query = if expected == 0 {
                        &e.prompt.as_ref().unwrap().value
                    } else if let Some(d) = &ui.finder {
                        d.input.value()
                    } else {
                        &ui.saved_query
                    };
                    assert_eq!(ui.picker.is_some(), expected >= 3);
                    assert_eq!(query, "paperclip");
                    assert_eq!(e.mode, editio::Mode::View);
                    assert_eq!(e.buffer.text.to_string(), "paperclip");
                }
            }
        }
        assert!(
            ui.worker.is_none(),
            "cycling must not perform a scan until debounce"
        );
    }
    #[test]
    fn unified_commands_are_in_place_contextual_and_name_the_destination_state() {
        let dir = tempfile::tempdir().unwrap();
        let mut ui = Navigation::with_preferences(
            dir.path().join("config.json"),
            crate::events::test_wake(),
        )
        .unwrap();
        let mut e = Editor::new(editio::buffer::Buffer::new("hello"));
        e.open_find_query("hello".into());
        let options = Event::Key(KeyEvent::new(K::Char('k'), M::CONTROL));
        ui.handle(&options, &mut e);
        assert!(matches!(ui.menu.as_ref().unwrap().choices[0], Choice::Case));
        ui.menu
            .as_mut()
            .unwrap()
            .palette
            .set_query("Use case-sensitive");
        assert_eq!(ui.menu.as_ref().unwrap().palette.match_count(), 1);
        ui.handle(&Event::Key(KeyEvent::new(K::Enter, M::NONE)), &mut e);
        assert!(e.search_options.case_sensitive);
        assert_eq!(e.prompt.as_ref().unwrap().value, "hello");
        ui.handle(&options, &mut e);
        ui.menu
            .as_mut()
            .unwrap()
            .palette
            .set_query("Use case-insensitive");
        assert_eq!(ui.menu.as_ref().unwrap().palette.match_count(), 1);
        ui.handle(&Event::Key(KeyEvent::new(K::Esc, M::NONE)), &mut e);
        assert!(ui.menu.is_none());
        assert_eq!(e.prompt.as_ref().unwrap().value, "hello");
        ui.handle(&Event::Key(KeyEvent::new(K::Tab, M::NONE)), &mut e);
        ui.handle(&options, &mut e);
        assert!(matches!(
            ui.menu.as_ref().unwrap().choices[0],
            Choice::Content
        ));
        for (width, height) in [(80, 24), (30, 10), (9, 4)] {
            let mut t =
                ratatui::Terminal::new(ratatui::backend::TestBackend::new(width + 4, height + 4))
                    .unwrap();
            t.draw(|f| ui.draw(f, Rect::new(2, 2, width, height), &e))
                .unwrap();
            assert_eq!(t.backend().buffer()[(0, 0)].symbol(), " ");
            let rect = tapp_ui::layout::search_modal_rect(Rect::new(
                2,
                2,
                width,
                height.saturating_sub(1),
            ));
            assert_eq!(t.backend().buffer()[(rect.x, rect.y)].symbol(), "┌");
        }
    }
    #[test]
    fn tag_and_registered_glob_pickers_require_selection_and_reopen_all_choices() {
        let dir = tempfile::tempdir().unwrap();
        let mut ui = Navigation::with_preferences(
            dir.path().join("config.json"),
            crate::events::test_wake(),
        )
        .unwrap();
        ui.catalog.tags.insert("work".into(), Default::default());
        ui.catalog
            .tags
            .insert("personal".into(), Default::default());
        ui.catalog
            .globs
            .extend(["**/*.md".into(), "notes/*.txt".into()]);
        let mut e = Editor::new(editio::buffer::Buffer::new("paperclip"));
        e.open_find_query("paperclip".into());
        for mode in [3, 4] {
            ui.change_mode(mode, &mut e);
            assert!(ui.finder.is_none());
            assert!(ui.next_wakeup().is_none());
            assert_eq!(
                ui.picker.as_ref().unwrap().1.choices.len(),
                if mode == 3 { 2 } else { 3 }
            );
            let filter = if mode == 3 { "work" } else { "notes" };
            ui.handle(&Event::Paste(filter.into()), &mut e);
            assert_eq!(ui.picker.as_ref().unwrap().1.palette.match_count(), 1);
            ui.handle(&Event::Key(KeyEvent::new(K::Enter, M::NONE)), &mut e);
            assert!(ui.picker.is_none());
            assert_eq!(ui.finder.as_ref().unwrap().input.value(), "paperclip");
            assert_eq!(ui.mode(&e), mode);
            let before = ui.content;
            ui.handle(&Event::Key(KeyEvent::new(K::Char('l'), M::CONTROL)), &mut e);
            assert_eq!(ui.content, !before);
            assert_eq!(ui.mode(&e), mode);
            ui.handle(&Event::Key(KeyEvent::new(K::Esc, M::NONE)), &mut e);
            assert!(ui.finder.is_none());
            assert!(ui.next_wakeup().is_none());
            assert_eq!(
                ui.picker.as_ref().unwrap().1.palette.match_count(),
                if mode == 3 { 2 } else { 3 }
            );
            assert_eq!(ui.saved_query, "paperclip");
            for (w, h) in [(80, 24), (20, 8)] {
                let mut t =
                    ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
                t.draw(|f| ui.draw(f, f.area(), &e)).unwrap();
            }
        }
    }
    #[test]
    fn built_in_globs_are_always_listed_once_before_user_patterns() {
        let dir = tempfile::tempdir().unwrap();
        let mut ui = Navigation::with_preferences(
            dir.path().join("config.json"),
            crate::events::test_wake(),
        )
        .unwrap();
        assert_eq!(ui.glob_patterns().collect::<Vec<_>>(), ["*.md", "**/*.md"]);
        ui.catalog.globs.extend(["*.md".into(), "src/*.rs".into()]);
        ui.show_picker(4);
        let patterns: Vec<_> = ui
            .picker
            .as_ref()
            .unwrap()
            .1
            .choices
            .iter()
            .filter_map(|c| {
                if let Choice::Scope(Scope::Glob(g)) = c {
                    Some(g.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(patterns, ["*.md", "**/*.md", "src/*.rs"]);
        assert!(ui.next_wakeup().is_none());
    }
    #[test]
    fn tall_find_viewport_shows_twenty_complete_three_line_results() {
        let dir = tempfile::tempdir().unwrap();
        let mut ui = Navigation::with_preferences(
            dir.path().join("config.json"),
            crate::events::test_wake(),
        )
        .unwrap();
        let mut e = Editor::new(editio::buffer::Buffer::new(""));
        ui.open(&mut e);
        ui.hits = (0..30)
            .map(|i| Hit {
                path: dir.path().join(format!("item{i}.md")),
                line: 0,
                column: 0,
                snippet: "matched content".into(),
                highlight: 0..7,
                score: 0,
                recency: 0,
            })
            .collect();
        let d = ui.finder.as_mut().unwrap();
        d.viewport.set_heights([3; 30]);
        d.complete(d.revision(), (0..30).collect(), false);
        for height in [80, 81] {
            let mut t =
                ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, height)).unwrap();
            t.draw(|f| ui.draw(f, f.area(), &e)).unwrap();
            let viewport = &ui.finder.as_ref().unwrap().viewport;
            let rect = viewport.area();
            assert!(rect.height >= 60);
            assert_eq!(viewport.hit(rect.x, rect.y + 59), Some(19));
        }
    }
    #[test]
    fn git_ignore_toggle_is_temporary_and_labels_the_destination() {
        let dir = tempfile::tempdir().unwrap();
        let prefs = dir.path().join("config.json");
        let mut ui =
            Navigation::with_preferences(prefs.clone(), crate::events::test_wake()).unwrap();
        let mut e = Editor::new(editio::buffer::Buffer::new(""));
        ui.open(&mut e);
        ui.choices(&e);
        ui.menu
            .as_mut()
            .unwrap()
            .palette
            .set_query("Include Git-ignored files");
        assert_eq!(ui.menu.as_ref().unwrap().palette.match_count(), 1);
        ui.handle(&Event::Key(KeyEvent::new(K::Enter, M::NONE)), &mut e);
        assert!(!ui.respect_gitignore);
        ui.choices(&e);
        ui.menu
            .as_mut()
            .unwrap()
            .palette
            .set_query("Respect Git ignore rules");
        assert_eq!(ui.menu.as_ref().unwrap().palette.match_count(), 1);
        ui.cancel();
        assert!(!ui.respect_gitignore);
        assert!(!prefs.exists());
        let fresh = Navigation::with_preferences(prefs, crate::events::test_wake()).unwrap();
        assert!(fresh.respect_gitignore);
    }
    #[test]
    fn ordinary_editing_has_no_file_worker_or_deadline() {
        let d = tempfile::tempdir().unwrap();
        let mut ui =
            Navigation::with_preferences(d.path().join("config.json"), crate::events::test_wake())
                .unwrap();
        let mut e = Editor::new(editio::buffer::Buffer::new(""));
        e.buffer.path = Some(d.path().join("notes.md"));
        e.mode = editio::Mode::Edit;
        for _ in 0..1000 {
            e.buffer.insert("x");
            ui.after_event(&e);
            assert!(ui.next_wakeup().is_none());
        }
        assert!(ui.worker.is_none());
    }
}
