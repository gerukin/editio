//! Application-owned, bounded EditorConfig lookup. No watchers or per-key parsing.
use ec4rs::PropertiesSource;
use editio::{Editor, Indentation};
use std::{
    io::{self, Read},
    path::{Path, PathBuf},
};

#[derive(Default)]
pub struct State {
    key: Option<(Option<PathBuf>, u64)>,
}
impl State {
    pub fn sync(&mut self, editor: &mut Editor) {
        editor.sync_indentation_preferences();
        let refresh = editor.indentation_refresh();
        if self
            .key
            .as_ref()
            .is_some_and(|(path, n)| path == &editor.buffer.path && *n == refresh)
        {
            return;
        }
        self.key = Some((editor.buffer.path.clone(), refresh));
        let preference = editor.indentation_preference();
        let result = editor
            .buffer
            .path
            .as_deref()
            .map(|path| resolve(path, preference));
        match result {
            Some(Ok((policy, source))) => editor.set_resolved_indentation(policy, source),
            Some(Err(error)) => {
                editor.set_resolved_indentation(
                    preference,
                    "User preference (EditorConfig unavailable)".into(),
                );
                editor.notify_warning(format!("EditorConfig: {error}"));
            }
            None => {
                editor.set_resolved_indentation(preference, "User preference or default".into())
            }
        }
    }
}
fn properties(path: &Path) -> io::Result<ec4rs::Properties> {
    let absolute = std::path::absolute(path)?;
    let mut files = Vec::new();
    let mut total = 0;
    for dir in absolute.parent().into_iter().flat_map(Path::ancestors) {
        let config = dir.join(".editorconfig");
        let Some(file) = tapp_ui::storage::open_regular_file(&config)? else {
            continue;
        };
        let mut bytes = Vec::new();
        file.take(256 * 1024 + 1).read_to_end(&mut bytes)?;
        total += bytes.len();
        if bytes.len() > 256 * 1024 || total > 1024 * 1024 {
            return Err(io::Error::other(
                "configuration exceeds 256 KiB/file or 1 MiB total",
            ));
        }
        let parser =
            ec4rs::ConfigParser::new_with_path(io::Cursor::new(bytes), Some(config.as_path()))
                .map_err(io::Error::other)?;
        let root = parser.is_root;
        files.push((dir.to_owned(), parser));
        if root {
            break;
        }
    }
    let mut properties = ec4rs::Properties::new();
    for (dir, mut parser) in files.into_iter().rev() {
        parser
            .apply_to(&mut properties, absolute.strip_prefix(dir).unwrap())
            .map_err(io::Error::other)?;
    }
    Ok(properties)
}
pub fn resolve(path: &Path, mut policy: Indentation) -> io::Result<(Indentation, String)> {
    let mut properties = properties(path)?;
    properties.use_fallbacks();
    let raw = |key| properties.get_raw_for_key(key).filter_unset().into_option();
    let number = |key| {
        raw(key)
            .and_then(|v| v.parse::<u8>().ok())
            .filter(|n| (1..=32).contains(n))
    };
    let mut used = Vec::new();
    if let Some(value @ ("space" | "tab")) = raw("indent_style") {
        policy.tabs = value == "tab";
        used.push("indent_style");
    }
    if let Some(width) = number("tab_width") {
        policy.tab_width = width;
        used.push("tab_width");
    }
    if let Some(size) = number("indent_size") {
        policy.size = size;
        used.push("indent_size");
    } else if raw("indent_size") == Some("tab") {
        policy.size = policy.tab_width;
        used.push("indent_size");
    }
    let source = if used.is_empty() {
        "User preference or default".into()
    } else {
        used.iter()
            .map(|key| {
                let raw = properties.get_raw_for_key(key);
                match raw.source() {
                    Some((path, line)) => format!("{key}: {}:{line}", path.display()),
                    None => format!("{key}: EditorConfig fallback"),
                }
            })
            .collect::<Vec<_>>()
            .join("; ")
    };
    Ok((policy, source))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inheritance_unset_globs_and_nonexistent_targets() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir(d.path().join("child")).unwrap();
        std::fs::write(d.path().join(".editorconfig"), "root = true\n[*]\nindent_style = space\nindent_size = 4\n[*.{md,txt}]\nindent_size = 2\n").unwrap();
        std::fs::write(d.path().join("child/.editorconfig"), "[*.md]\nindent_style = unset\nindent_size = tab\ntab_width = 3\ntrim_trailing_whitespace = true\n").unwrap();
        let (p, _) = resolve(
            &d.path().join("child/missing/new.md"),
            Indentation::default(),
        )
        .unwrap();
        assert_eq!(
            p,
            Indentation {
                tabs: true,
                size: 3,
                tab_width: 3
            }
        );
        let (p, _) = resolve(&d.path().join("other.rs"), Indentation::default()).unwrap();
        assert_eq!(
            p,
            Indentation {
                tabs: false,
                size: 4,
                tab_width: 4
            }
        );
    }
    #[test]
    fn idle_sync_does_not_reread_config_and_overrides_win() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join(".editorconfig");
        std::fs::write(&path, "root=true\n[*]\nindent_size=3\n").unwrap();
        let mut e = Editor::new(editio::buffer::Buffer::new(""));
        e.buffer.path = Some(d.path().join("a.md"));
        let mut state = State::default();
        state.sync(&mut e);
        assert_eq!(e.indentation().size, 3);
        std::fs::write(path, "root=true\n[*]\nindent_size=8\n").unwrap();
        state.sync(&mut e);
        assert_eq!(e.indentation().size, 3);
        e.override_indentation(Indentation {
            size: 5,
            ..Default::default()
        });
        e.act(editio::Action::IndentReload);
        state.sync(&mut e);
        assert_eq!(e.indentation().size, 5);
        e.act(editio::Action::IndentAuto);
        assert_eq!(e.indentation().size, 8);
    }
}
