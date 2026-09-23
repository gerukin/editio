//! App-owned file catalog and bounded, on-demand searches. No index or watcher.
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tapp_ui::text::search::SearchOptions;
pub const LIMIT: usize = 50;
pub const HISTORY_LIMIT: usize = 256;
pub const ENTRY_LIMIT: usize = 50_000;
// Filename matching does no content reads; allow ordinary SDK/vendor trees.
pub const NAME_ENTRY_LIMIT: usize = 200_000;
pub const BYTE_LIMIT: usize = 32 * 1024 * 1024;
pub const FILE_LIMIT: usize = 2 * 1024 * 1024;
pub const VIEWED: &str = "Recently viewed";
pub const MODIFIED: &str = "Recently modified";
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Catalog {
    pub tags: BTreeMap<String, BTreeSet<PathBuf>>,
    pub globs: BTreeSet<String>,
    pub viewed: BTreeMap<PathBuf, u64>,
    pub modified: BTreeMap<PathBuf, u64>,
}
pub fn prefs_path() -> io::Result<PathBuf> {
    tapp_ui::editor::preferences::default_path()
        .ok_or_else(|| io::Error::other("Cannot locate user preferences"))
}
fn document(bytes: Option<&[u8]>) -> io::Result<serde_json::Value> {
    let value = match bytes {
        Some(b) => serde_json::from_slice(b).map_err(io::Error::other)?,
        None => serde_json::json!({"version":1}),
    };
    if !value.is_object() || value.get("version").and_then(|v| v.as_u64()) != Some(1) {
        return Err(io::Error::other("Unsupported preferences; not overwritten"));
    }
    Ok(value)
}
impl Catalog {
    pub fn load(path: &Path) -> io::Result<Self> {
        let bytes = tapp_ui::storage::read(path)?;
        let value = document(bytes.as_deref())?;
        value
            .get("navigation")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map(|v| v.unwrap_or_default())
            .map_err(io::Error::other)
    }
    pub fn update(
        path: &Path,
        change: impl FnOnce(&mut Self) -> io::Result<()>,
    ) -> io::Result<Self> {
        let mut result = None;
        tapp_ui::storage::update(path, |bytes| {
            let mut value = document(bytes)?;
            let mut data: Self = value
                .get("navigation")
                .cloned()
                .map(serde_json::from_value)
                .transpose()
                .map_err(io::Error::other)?
                .unwrap_or_default();
            change(&mut data)?;
            value["navigation"] = serde_json::to_value(&data).map_err(io::Error::other)?;
            result = Some(data);
            let mut bytes = serde_json::to_vec_pretty(&value).map_err(io::Error::other)?;
            bytes.push(b'\n');
            Ok(bytes)
        })?;
        Ok(result.unwrap())
    }
    pub fn tag(&mut self, path: &Path, tag: &str, remove: bool) -> io::Result<()> {
        let tag = tag.trim();
        if tag.is_empty()
            || tag.len() > 80
            || tag.chars().any(char::is_control)
            || tag.eq_ignore_ascii_case(VIEWED)
            || tag.eq_ignore_ascii_case(MODIFIED)
        {
            return Err(io::Error::other(
                "Choose a nonempty user tag (system history tags are reserved)",
            ));
        }
        let path = fs::canonicalize(path)?;
        if !path.is_file() {
            return Err(io::Error::other("Tags apply to regular files"));
        }
        if remove {
            if let Some(files) = self.tags.get_mut(tag) {
                files.remove(&path);
                if files.is_empty() {
                    self.tags.remove(tag);
                }
            }
        } else {
            self.tags.entry(tag.into()).or_default().insert(path);
        }
        Ok(())
    }
    pub fn record(&mut self, path: &Path, modified: bool) -> io::Result<()> {
        let path = fs::canonicalize(path)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let history = if modified {
            &mut self.modified
        } else {
            &mut self.viewed
        };
        history.insert(path, now);
        while history.len() > HISTORY_LIMIT {
            let oldest = history
                .iter()
                .min_by_key(|(_, t)| *t)
                .map(|(p, _)| p.clone())
                .unwrap();
            history.remove(&oldest);
        }
        Ok(())
    }
    pub fn rename(&mut self, old: &Path, new: &Path) {
        for files in self.tags.values_mut() {
            if files.remove(old) {
                files.insert(new.into());
            }
        }
        for history in [&mut self.viewed, &mut self.modified] {
            if let Some(t) = history.remove(old) {
                history.insert(new.into(), t);
            }
        }
    }
    pub fn paths(&self, tag: &str) -> Vec<PathBuf> {
        match tag {
            VIEWED => self.viewed.keys().cloned().collect(),
            MODIFIED => self.modified.keys().cloned().collect(),
            _ => self
                .tags
                .get(tag)
                .map(|v| v.iter().cloned().collect())
                .unwrap_or_default(),
        }
    }
}
#[derive(Clone, Debug)]
pub enum Scope {
    Tag(String),
    Glob(String),
}
#[derive(Clone, Debug)]
pub struct Request {
    pub generation: u64,
    pub base: PathBuf,
    pub scope: Scope,
    pub paths: Vec<PathBuf>,
    pub activity: BTreeMap<PathBuf, u64>,
    pub query: String,
    pub content: bool,
    pub respect_gitignore: bool,
    pub options: SearchOptions,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub snippet: String,
    pub highlight: std::ops::Range<usize>,
    pub score: usize,
    /// Activity time for recent views; filesystem modification time for other scopes.
    pub recency: u64,
}
#[derive(Default)]
pub struct Results {
    pub hits: Vec<Hit>,
    pub limited: bool,
    pub cutoff: Option<&'static str>,
    pub skipped: usize,
}
pub fn base(path: Option<&Path>) -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let parent = path
        .and_then(Path::parent)
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(&cwd);
    if parent.is_absolute() {
        parent.into()
    } else {
        cwd.join(parent)
    }
}
pub fn pattern(base: &Path, pattern: &str) -> io::Result<(PathBuf, globset::GlobMatcher)> {
    if pattern.is_empty() || pattern.len() > 4096 {
        return Err(io::Error::other(
            "Enter a glob pattern (at most 4096 bytes)",
        ));
    }
    let expanded = tapp_ui::path::parse(pattern);
    let absolute = if expanded.is_absolute() {
        expanded
    } else {
        base.join(expanded)
    };
    let mut root = PathBuf::new();
    for component in absolute.components() {
        if component
            .as_os_str()
            .to_string_lossy()
            .contains(['*', '?', '[', '{'])
        {
            break;
        }
        root.push(component);
    }
    if root == absolute {
        root.pop();
    }
    if root.as_os_str().is_empty() {
        root = base.into();
    }
    let text = absolute.to_string_lossy().replace('\\', "/");
    let matcher = globset::GlobBuilder::new(&text)
        .literal_separator(true)
        .backslash_escape(false)
        .build()
        .map_err(io::Error::other)?
        .compile_matcher();
    Ok((root, matcher))
}
fn git_walk(root: &Path, respect: bool) -> ignore::WalkBuilder {
    let mut builder = ignore::WalkBuilder::new(root);
    builder
        .standard_filters(false)
        .parents(respect)
        .git_ignore(respect)
        .git_exclude(respect)
        .git_global(respect)
        .require_git(true)
        .follow_links(false);
    builder
}
// Matchers are created only by a search worker, cached for this request, then dropped.
// The explicit document directory is a traversal root: its own ignored status
// does not block it, but ignore rules for descendants still apply.
struct GitFilter {
    base: PathBuf,
    matchers: BTreeMap<PathBuf, ignore::IncrementalIgnore>,
}
impl GitFilter {
    fn new(base: &Path) -> Self {
        Self {
            base: base.into(),
            matchers: BTreeMap::new(),
        }
    }
    fn ignored(&mut self, path: &Path, directory: bool) -> bool {
        let canonical;
        let path = if path
            .components()
            .any(|c| c == std::path::Component::ParentDir)
        {
            canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
            &canonical
        } else {
            path
        };
        let root = if path.starts_with(&self.base) {
            self.base.clone()
        } else {
            path.ancestors()
                .find(|p| p.join(".git").exists())
                .unwrap_or_else(|| path.parent().unwrap_or(path))
                .to_path_buf()
        };
        let matcher = self
            .matchers
            .entry(root.clone())
            .or_insert_with(|| git_walk(&root, true).build_matchers().pop().unwrap());
        let Some(relative) = matcher.normalize(path) else {
            return false;
        };
        matcher.matched(relative, directory).is_ignore()
    }
}
pub fn search(request: &Request, generation: &AtomicU64) -> Result<Results, String> {
    let start = Instant::now();
    let mut out = Results::default();
    let entry_limit = if request.content {
        ENTRY_LIMIT
    } else {
        NAME_ENTRY_LIMIT
    };
    let mut visited = 0;
    let mut read = 0;
    if request.query.len() > 1024 {
        return Err("Search query is too long".into());
    }
    let matcher = tapp_ui::text::search::compile(&request.query, request.options)?;
    let ignores = std::cell::RefCell::new(GitFilter::new(&request.base));
    let mut inspect = |path: PathBuf| -> bool {
        visited += 1;
        if generation.load(Ordering::Relaxed) != request.generation {
            return false;
        }
        if visited > entry_limit
            || start.elapsed() > Duration::from_millis(750)
            || read >= BYTE_LIMIT
        {
            out.cutoff = Some(if visited > entry_limit {
                "entry limit"
            } else if read >= BYTE_LIMIT {
                "content limit"
            } else {
                "time limit"
            });
            out.limited = true;
            return false;
        }
        if request.respect_gitignore
            && matches!(&request.scope, Scope::Tag(tag) if tag != VIEWED && tag != MODIFIED)
            && ignores.borrow_mut().ignored(&path, false)
        {
            return true;
        }
        let Ok(meta) = fs::metadata(&path) else {
            out.skipped += 1;
            return true;
        };
        if !meta.is_file() {
            return true;
        }
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let modified = if matches!(&request.scope, Scope::Tag(tag) if tag == VIEWED || tag == MODIFIED)
        {
            request.activity.get(&path).copied().unwrap_or(0)
        } else {
            modified
        };
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if !request.content {
            if request.query.is_empty() || matcher.is_match(&name) {
                let score = matcher
                    .find(&name)
                    .map(|m| {
                        let rank = if m.start() == 0 && m.end() == name.len() {
                            0
                        } else if m.start() == 0 {
                            1
                        } else {
                            2
                        };
                        rank * 1_000_000
                            + m.len().saturating_sub(request.query.len()) * 1000
                            + m.start()
                    })
                    .unwrap_or(0);
                out.hits.push(Hit {
                    path,
                    line: 0,
                    column: 0,
                    snippet: String::new(),
                    highlight: 0..0,
                    score,
                    recency: modified,
                });
            }
        } else if !request.query.is_empty() {
            if meta.len() > FILE_LIMIT as u64 {
                out.skipped += 1;
                return true;
            }
            let Ok(Some(file)) = tapp_ui::storage::open_regular_file(&path) else {
                out.skipped += 1;
                return true;
            };
            let mut bytes = Vec::new();
            if file
                .take((FILE_LIMIT + 1) as u64)
                .read_to_end(&mut bytes)
                .is_err()
                || bytes.len() > FILE_LIMIT
            {
                out.skipped += 1;
                return true;
            }
            read += bytes.len();
            if bytes.contains(&0) {
                out.skipped += 1;
                return true;
            }
            let Ok(text) = std::str::from_utf8(&bytes) else {
                out.skipped += 1;
                return true;
            };
            for (line, text) in text.lines().enumerate() {
                if line % 128 == 0
                    && (generation.load(Ordering::Relaxed) != request.generation
                        || start.elapsed() > Duration::from_millis(750))
                {
                    out.cutoff = Some("time limit");
                    out.limited = true;
                    return false;
                }
                if text.len() > 64 * 1024 {
                    out.skipped += 1;
                    continue;
                }
                if let Some(m) = matcher.find(text) {
                    let column = text[..m.start()].chars().count();
                    let snippet = text
                        .chars()
                        .skip(column.saturating_sub(32))
                        .take(160)
                        .collect();
                    out.hits.push(Hit {
                        path: path.clone(),
                        line,
                        column,
                        snippet,
                        highlight: column.min(32)
                            ..(column.min(32) + text[m.range()].chars().count()).min(160),
                        score: m.len().saturating_sub(request.query.len()),
                        recency: modified,
                    });
                }
                if out.hits.len() >= LIMIT * 2 {
                    trim(&mut out);
                }
            }
        }
        if out.hits.len() >= LIMIT * 2 {
            trim(&mut out);
        }
        true
    };
    match &request.scope {
        Scope::Tag(_) => {
            for path in &request.paths {
                if !inspect(path.clone()) {
                    break;
                }
            }
        }
        Scope::Glob(glob) => {
            let (root, matcher) = pattern(&request.base, glob).map_err(|e| e.to_string())?;
            // Prioritize this directory without collecting/sorting huge directories.
            // The deeper pass skips these files, and both passes share the budget.
            if request.respect_gitignore && ignores.borrow_mut().ignored(&root, true) {
                return Ok(out);
            }
            let nearby = git_walk(&root, request.respect_gitignore)
                .max_depth(Some(1))
                .build()
                .map(|e| (true, e));
            let walk = git_walk(&root, request.respect_gitignore)
                .max_depth(Some(if glob.contains("**") {
                    64
                } else {
                    std::path::Path::new(glob).components().count().max(1)
                }))
                .build();
            for (n, (nearby, entry)) in nearby.chain(walk.map(|e| (false, e))).enumerate() {
                if n >= entry_limit || start.elapsed() > Duration::from_millis(750) {
                    out.cutoff = Some(if n >= entry_limit {
                        "entry limit"
                    } else {
                        "time limit"
                    });
                    out.limited = true;
                    break;
                }
                if generation.load(Ordering::Relaxed) != request.generation {
                    break;
                }
                let Ok(entry) = entry else {
                    continue;
                };
                if (nearby || entry.depth() > 1)
                    && entry.file_type().is_some_and(|t| t.is_file())
                    && matcher.is_match(entry.path())
                    && !inspect(entry.path().into())
                {
                    break;
                }
            }
        }
    }
    trim(&mut out);
    Ok(out)
}
fn trim(out: &mut Results) {
    out.hits.sort_unstable_by(|a, b| {
        a.score
            .cmp(&b.score)
            .then(b.recency.cmp(&a.recency))
            .then(a.path.cmp(&b.path))
            .then(a.line.cmp(&b.line))
    });
    if out.hits.len() > LIMIT {
        out.limited = true;
        out.hits.truncate(LIMIT);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn history_activity_orders_before_capping_but_match_quality_stays_first() {
        let dir = tempfile::tempdir().unwrap();
        let mut paths = vec![];
        let mut activity = BTreeMap::new();
        for i in 0..60 {
            let path = dir.path().join(format!("note-{i:02}.md"));
            fs::write(&path, "needle\nneedle").unwrap();
            fs::File::open(&path)
                .unwrap()
                .set_times(
                    fs::FileTimes::new().set_modified(UNIX_EPOCH + Duration::from_secs(1000 - i)),
                )
                .unwrap();
            activity.insert(path.clone(), i);
            paths.push(path);
        }
        let mut request = Request {
            generation: 0,
            base: dir.path().into(),
            scope: Scope::Tag(VIEWED.into()),
            paths,
            activity,
            query: String::new(),
            content: false,
            respect_gitignore: true,
            options: Default::default(),
        };
        let generation = AtomicU64::new(0);
        for tag in [VIEWED, MODIFIED] {
            request.scope = Scope::Tag(tag.into());
            request.query.clear();
            let result = search(&request, &generation).unwrap();
            assert_eq!(result.hits.len(), 50);
            assert!(result.limited);
            assert_eq!(result.hits[0].path, request.paths[59]);
            assert_eq!(result.hits[49].path, request.paths[10]);
            request.query = "note-00.md".into();
            assert_eq!(
                search(&request, &generation).unwrap().hits[0].path,
                request.paths[0]
            );
            request.query = "needle".into();
            request.content = true;
            let result = search(&request, &generation).unwrap();
            assert_eq!(result.hits[0].path, request.paths[59]);
            assert_eq!(result.hits[0].line, 0);
            assert_eq!(result.hits[1].line, 1);
            request.content = false;
        }
        // A stronger filename match wins even when its activity is older.
        let extra = dir.path().join("note-00.md-backup");
        fs::write(&extra, "needle").unwrap();
        request.paths.push(extra.clone());
        request.activity.insert(extra, 9999);
        request.query = "note-00.md".into();
        assert_eq!(
            search(&request, &generation).unwrap().hits[0].path,
            request.paths[0]
        );
        request.paths.pop();
        request.query.clear();
        for scope in [Scope::Tag("work".into()), Scope::Glob("*.md".into())] {
            request.scope = scope;
            assert_eq!(
                search(&request, &generation).unwrap().hits[0].path,
                request.paths[0]
            );
        }
        request.scope = Scope::Tag(VIEWED.into());
        request.activity.values_mut().for_each(|value| *value = 1);
        assert_eq!(
            search(&request, &generation).unwrap().hits[0].path,
            request.paths[0]
        );
    }

    #[test]
    fn fifty_results_are_exact_and_fifty_one_are_capped() {
        let dir = tempfile::tempdir().unwrap();
        let request = Request {
            generation: 0,
            base: dir.path().to_owned(),
            scope: Scope::Glob("**/*.md".into()),
            paths: vec![],
            query: String::new(),
            content: false,
            respect_gitignore: false,
            activity: Default::default(),
            options: Default::default(),
        };
        let generation = AtomicU64::new(0);
        for i in 0..50 {
            fs::write(dir.path().join(format!("{i}.md")), "test").unwrap();
        }
        let result = search(&request, &generation).unwrap();
        assert_eq!(result.hits.len(), 50);
        assert!(!result.limited);
        fs::write(dir.path().join("extra.md"), "test").unwrap();
        let result = search(&request, &generation).unwrap();
        assert_eq!(result.hits.len(), 50);
        assert!(result.limited);
    }

    #[test]
    fn git_ignores_apply_from_nested_files_and_can_be_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join(".git/info")).unwrap();
        fs::create_dir(root.join("sub")).unwrap();
        fs::write(
            root.join(".gitignore"),
            "/ignored/\n*.skip.md\nsub/child.md\n!sub/keep.skip.md\n",
        )
        .unwrap();
        fs::write(root.join(".git/info/exclude"), "excluded.md\n").unwrap();
        fs::write(root.join("sub/.gitignore"), "/nested.md\n!local.skip.md\n").unwrap();
        fs::write(root.join("sub/.ignore"), "*.md\n").unwrap();
        for name in [
            "normal.md",
            "blocked.skip.md",
            "keep.skip.md",
            "local.skip.md",
            "nested.md",
            "child.md",
            "excluded.md",
            ".hidden.md",
        ] {
            fs::write(root.join("sub").join(name), "needle").unwrap();
        }
        let mut r = Request {
            generation: 0,
            base: root.join("sub"),
            scope: Scope::Glob("**/*.md".into()),
            paths: vec![],
            query: String::new(),
            content: false,
            respect_gitignore: true,
            activity: Default::default(),
            options: Default::default(),
        };
        let generation = AtomicU64::new(0);
        for content in [false, true] {
            r.content = content;
            r.query = if content { "needle" } else { "" }.into();
            let result = search(&r, &generation).unwrap();
            assert_eq!(result.hits.len(), 4, "{:?}", result.hits);
            assert!(!result.limited);
        }
        r.respect_gitignore = false;
        assert_eq!(search(&r, &generation).unwrap().hits.len(), 8);
        r.respect_gitignore = true;
        r.scope = Scope::Tag("work".into());
        r.paths = vec![root.join("sub/normal.md"), root.join("sub/blocked.skip.md")];
        assert_eq!(search(&r, &generation).unwrap().hits.len(), 1);
        r.respect_gitignore = false;
        assert_eq!(search(&r, &generation).unwrap().hits.len(), 2);
    }
    #[test]
    fn an_already_ignored_directory_is_searchable_but_descendant_rules_still_apply() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir(root.join(".git")).unwrap();
        fs::create_dir_all(root.join("ignored/nested")).unwrap();
        fs::write(root.join(".gitignore"), "/ignored/\n").unwrap();
        fs::write(root.join("ignored/.gitignore"), "nested/\n").unwrap();
        fs::write(root.join("ignored/direct.md"), "content").unwrap();
        fs::write(root.join("ignored/nested/drop.md"), "content").unwrap();
        let mut r = Request {
            generation: 0,
            base: root.join("ignored"),
            scope: Scope::Glob("**/*.md".into()),
            paths: vec![],
            query: String::new(),
            content: false,
            respect_gitignore: true,
            activity: Default::default(),
            options: Default::default(),
        };
        let generation = AtomicU64::new(0);
        let result = search(&r, &generation).unwrap();
        assert_eq!(result.hits.len(), 1, "{:?}", result.hits);
        assert!(result.hits[0].path.ends_with("direct.md"));
        r.base = root.into();
        r.scope = Scope::Glob("ignored/**/*.md".into());
        assert!(search(&r, &generation).unwrap().hits.is_empty());
        r.respect_gitignore = false;
        assert_eq!(search(&r, &generation).unwrap().hits.len(), 2);
    }
    #[test]
    fn gitignore_is_not_applied_outside_a_git_repository() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".gitignore"), "*.md\n").unwrap();
        fs::write(dir.path().join("note.md"), "note").unwrap();
        let r = Request {
            generation: 0,
            base: dir.path().into(),
            scope: Scope::Glob("*.md".into()),
            paths: vec![],
            query: String::new(),
            content: false,
            respect_gitignore: true,
            activity: Default::default(),
            options: Default::default(),
        };
        assert_eq!(search(&r, &AtomicU64::new(0)).unwrap().hits.len(), 1);
    }
    #[test]
    fn catalog_preserves_prefs_and_protects_system_tags() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("prefs.json");
        fs::write(&p, r#"{"version":1,"theme":"terminal","custom":42}"#).unwrap();
        let f = d.path().join("note.md");
        fs::write(&f, "hi").unwrap();
        let c = Catalog::update(&p, |c| {
            c.tag(&f, "work", false)?;
            c.record(&f, false)
        })
        .unwrap();
        assert_eq!(c.paths("work"), vec![fs::canonicalize(&f).unwrap()]);
        assert!(Catalog::update(&p, |c| c.tag(&f, VIEWED, true)).is_err());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&fs::read(p).unwrap()).unwrap()["custom"],
            42
        );
    }
    #[test]
    fn globs_name_content_and_bounds() {
        let d = tempfile::tempdir().unwrap();
        fs::create_dir(d.path().join("sub")).unwrap();
        fs::write(d.path().join("one.md"), "Needle").unwrap();
        fs::write(d.path().join("sub/two.md"), "a needle here").unwrap();
        let mut r = Request {
            generation: 0,
            base: d.path().into(),
            scope: Scope::Glob("*.md".into()),
            paths: vec![],
            query: String::new(),
            content: false,
            respect_gitignore: true,
            activity: Default::default(),
            options: Default::default(),
        };
        let g = AtomicU64::new(0);
        assert_eq!(search(&r, &g).unwrap().hits.len(), 1);
        r.scope = Scope::Glob("**/*.md".into());
        assert_eq!(search(&r, &g).unwrap().hits.len(), 2);
        r.content = true;
        r.query = "needle".into();
        assert_eq!(search(&r, &g).unwrap().hits.len(), 2);
        r.options.case_sensitive = true;
        assert_eq!(search(&r, &g).unwrap().hits.len(), 1);
        g.store(1, Ordering::Relaxed);
        assert!(search(&r, &g).unwrap().hits.is_empty());
    }
}

#[cfg(test)]
mod scale_tests {
    use super::*;
    #[test]
    fn bounded_results_sort_quality_then_file_modification_and_skip_large_content() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..600 {
            fs::write(dir.path().join(format!("note-{i:04}.md")), "paperclip\n").unwrap();
        }
        let exact = dir.path().join("note");
        fs::write(&exact, "paperclip\n").unwrap();
        let mut request = Request {
            generation: 0,
            base: dir.path().into(),
            scope: Scope::Glob("*".into()),
            paths: vec![],
            query: "note".into(),
            content: false,
            respect_gitignore: true,
            activity: Default::default(),
            options: Default::default(),
        };
        let generation = AtomicU64::new(0);
        let found = search(&request, &generation).unwrap();
        assert_eq!(found.hits.len(), LIMIT);
        assert!(found.limited);
        assert_eq!(found.hits[0].path, exact);
        let big = dir.path().join("large.md");
        fs::File::create(&big)
            .unwrap()
            .set_len(FILE_LIMIT as u64 + 1)
            .unwrap();
        request.paths = vec![big];
        request.scope = Scope::Tag("test".into());
        request.content = true;
        request.query = "x".into();
        let found = search(&request, &generation).unwrap();
        assert!(found.hits.is_empty());
        assert_eq!(found.skipped, 1);
    }
    #[test]
    fn system_history_is_bounded_and_rename_moves_user_tags() {
        let dir = tempfile::tempdir().unwrap();
        let mut c = Catalog::default();
        for i in 0..300 {
            let p = dir.path().join(i.to_string());
            fs::write(&p, "").unwrap();
            c.record(&p, false).unwrap();
        }
        assert_eq!(c.viewed.len(), HISTORY_LIMIT);
        let old = dir.path().join("299");
        let new = dir.path().join("renamed");
        c.tag(&old, "work", false).unwrap();
        c.rename(&old, &new);
        assert!(c.tags["work"].contains(&new));
        assert!(!c.tags["work"].contains(&old));
    }
    #[test]
    #[ignore = "large filesystem regression: creates 50,100 nonmatching files"]
    fn filename_search_survives_a_large_nonmatching_tool_tree() {
        let dir = tempfile::tempdir().unwrap();
        let tools = dir.path().join("tools");
        fs::create_dir(&tools).unwrap();
        for i in 0..50_100 {
            fs::write(tools.join(format!("{i}.dat")), "").unwrap();
        }
        fs::write(dir.path().join("root.md"), "root").unwrap();
        fs::write(tools.join("nested.md"), "nested").unwrap();
        let r = Request {
            generation: 0,
            base: dir.path().into(),
            scope: Scope::Glob("**/*.md".into()),
            paths: vec![],
            query: String::new(),
            content: false,
            respect_gitignore: true,
            activity: Default::default(),
            options: Default::default(),
        };
        let found = search(&r, &AtomicU64::new(0)).unwrap();
        assert_eq!(found.hits.len(), 2);
        assert!(!found.limited, "unexpected cutoff: {:?}", found.cutoff);
        assert!(found.hits.iter().any(|h| h.path.ends_with("root.md")));
        assert!(found.hits.iter().any(|h| h.path.ends_with("nested.md")));
    }
    #[test]
    #[ignore = "read-only reproduction against the local research directory"]
    fn measure_research_directory() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".local");
        let request = Request {
            generation: 0,
            base,
            scope: Scope::Glob("**/*.md".into()),
            paths: vec![],
            query: String::new(),
            content: false,
            respect_gitignore: true,
            activity: Default::default(),
            options: Default::default(),
        };
        let start = Instant::now();
        let result = search(&request, &AtomicU64::new(0)).unwrap();
        eprintln!(
            "{:?}: {} results, limited={}",
            start.elapsed(),
            result.hits.len(),
            result.limited
        );
        for hit in result.hits {
            eprintln!("{}", hit.path.display());
        }
    }
    #[test]
    #[ignore = "manual local search measurement"]
    fn measure_file_search() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..10_000 {
            fs::write(
                dir.path().join(format!("note-{i:05}.md")),
                "The paperclip is mightier than the inbox.\n",
            )
            .unwrap();
        }
        let mut request = Request {
            generation: 0,
            base: dir.path().into(),
            scope: Scope::Glob("*.md".into()),
            paths: vec![],
            query: "note".into(),
            content: false,
            respect_gitignore: true,
            activity: Default::default(),
            options: Default::default(),
        };
        let generation = AtomicU64::new(0);
        for content in [false, true] {
            request.content = content;
            request.query = if content { "paperclip" } else { "note" }.into();
            let started = Instant::now();
            let r = search(&request, &generation).unwrap();
            eprintln!(
                "content={content}: {:?}, {} retained, bounded={}",
                started.elapsed(),
                r.hits.len(),
                r.limited
            );
            assert!(r.hits.len() <= LIMIT);
        }
        if let Ok(s) = fs::read_to_string("/proc/self/status") {
            for line in s.lines().filter(|l| l.starts_with("VmHWM:")) {
                eprintln!("{line}");
            }
        }
    }
}
