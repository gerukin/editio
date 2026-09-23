//! Explicit link actions: path resolution is pure; only the host opens applications.
use std::{
    io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Local {
        path: PathBuf,
        anchor: Option<String>,
    },
    External(String),
}
pub fn resolve(value: &str, document: Option<&Path>) -> io::Result<Target> {
    if value.chars().any(char::is_control) {
        return Err(io::Error::other("Link contains control characters"));
    }
    let base = crate::files::base(document);
    let base = if base.is_absolute() {
        base
    } else {
        std::env::current_dir()?.join(base)
    };
    let parsed = if let Some(fragment) = value.strip_prefix('#') {
        let path = document
            .ok_or_else(|| io::Error::other("Save the document before following a local anchor"))?;
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()?.join(path)
        };
        let mut url =
            url::Url::from_file_path(path).map_err(|_| io::Error::other("Invalid file path"))?;
        url.set_fragment(Some(fragment));
        url
    } else if cfg!(windows) && value.as_bytes().get(1) == Some(&b':') {
        url::Url::from_file_path(value).map_err(|_| io::Error::other("Invalid Windows path"))?
    } else {
        url::Url::options()
            .base_url(Some(
                &url::Url::from_directory_path(base)
                    .map_err(|_| io::Error::other("Invalid link base"))?,
            ))
            .parse(value)
            .map_err(io::Error::other)?
    };
    if parsed.scheme() == "file" {
        let path = parsed
            .to_file_path()
            .map_err(|_| io::Error::other("Only local file URLs can open in Editio"))?;
        return Ok(Target::Local {
            path,
            anchor: parsed.fragment().map(decode),
        });
    }
    if matches!(parsed.scheme(), "javascript" | "data" | "vbscript") {
        return Err(io::Error::other(
            "Executable link protocols are not supported",
        ));
    }
    Ok(Target::External(parsed.to_string()))
}
fn decode(value: &str) -> String {
    let mut bytes = Vec::with_capacity(value.len());
    let input = value.as_bytes();
    let mut i = 0;
    while i < input.len() {
        if input[i] == b'%'
            && i + 2 < input.len()
            && let (Some(a), Some(b)) = (
                (input[i + 1] as char).to_digit(16),
                (input[i + 2] as char).to_digit(16),
            )
        {
            bytes.push((a * 16 + b) as u8);
            i += 3;
            continue;
        }
        bytes.push(input[i]);
        i += 1;
    }
    String::from_utf8_lossy(&bytes).into_owned()
}
pub fn open(target: &Target) -> io::Result<()> {
    let value = match target {
        Target::External(value) => value.clone(),
        Target::Local { path, .. } => url::Url::from_file_path(path)
            .map_err(|_| io::Error::other("Invalid local path"))?
            .to_string(),
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut c = Command::new("open");
        c.arg(&value);
        c
    };
    #[cfg(windows)]
    let mut command = {
        let mut c = Command::new("rundll32.exe");
        c.args(["url.dll,FileProtocolHandler", &value]);
        c
    };
    #[cfg(not(any(target_os = "macos", windows)))]
    let mut command = {
        let mut c = Command::new("xdg-open");
        c.arg(&value);
        c
    };
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}
#[derive(Clone, Debug)]
pub struct Completion {
    pub range: std::ops::Range<usize>,
    pub prefix: String,
    pub base: PathBuf,
}
pub fn completion(editor: &editio::Editor) -> Option<Completion> {
    if editor.mode != editio::Mode::Edit
        || editor.buffer.cursors.len() != 1
        || editor.prompt.is_some()
    {
        return None;
    }
    let path = editor.buffer.path.as_deref()?;
    if !matches!(
        path.extension().and_then(|s| s.to_str()),
        Some("md" | "markdown" | "mdown")
    ) {
        return None;
    }
    let cursor = &editor.buffer.cursors[0];
    if cursor.anchor != cursor.head {
        return None;
    }
    let line = editor.buffer.text.char_to_line(cursor.head);
    let start = editor.buffer.text.line_to_char(line);
    let prefix = editor.buffer.text.slice(start..cursor.head);
    if prefix.len_chars() > 4096 {
        return None;
    }
    let text = prefix.to_string();
    let marker = text.rfind("](")? + 2;
    let typed = &text[marker..];
    if typed.contains([')', '\n', '\r', '#', ':']) || typed.starts_with(['/', '~']) {
        return None;
    }
    let typed = typed.strip_prefix('<').unwrap_or(typed);
    let offset = text[..marker].chars().count() + usize::from(text[marker..].starts_with('<'));
    Some(Completion {
        range: start + offset..cursor.head,
        prefix: typed.into(),
        base: crate::files::base(Some(path)),
    })
}
pub fn complete(c: &Completion) -> io::Result<Vec<String>> {
    let prefix = decode(&c.prefix);
    let (directory, fragment) = prefix
        .rsplit_once('/')
        .map(|(d, f)| (format!("{d}/"), f))
        .unwrap_or((String::new(), prefix.as_str()));
    let folded_fragment = fragment.to_lowercase();
    let mut result = vec![];
    let start = std::time::Instant::now();
    for (i, entry) in std::fs::read_dir(c.base.join(&directory))?.enumerate() {
        if i >= 10_000 || start.elapsed() > std::time::Duration::from_millis(100) {
            break;
        }
        let Ok(entry) = entry else {
            continue;
        };
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if !name.to_lowercase().starts_with(&folded_fragment) {
            continue;
        }
        let kind = entry.file_type()?;
        if !kind.is_file() && !kind.is_dir() {
            continue;
        }
        let mut candidate = format!("{directory}{name}");
        if kind.is_dir() {
            candidate.push('/');
        }
        candidate = candidate
            .replace('%', "%25")
            .replace(' ', "%20")
            .replace('(', "%28")
            .replace(')', "%29")
            .replace('#', "%23");
        if candidate == c.prefix || candidate.chars().any(char::is_control) {
            continue;
        }
        result.push(candidate);
        if result.len() == 64 {
            break;
        }
    }
    result.sort();
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn relative_and_protocol_links() {
        assert_eq!(decode("a%20b%"), "a b%");
        assert_eq!(decode("%X%QQ"), "%X%QQ");
        let p = Path::new("/tmp/notes/start.md");
        assert_eq!(
            resolve("../other%20note.md#chapter", Some(p)).unwrap(),
            Target::Local {
                path: "/tmp/other note.md".into(),
                anchor: Some("chapter".into())
            }
        );
        assert!(matches!(
            resolve("mailto:a@example.org", Some(p)).unwrap(),
            Target::External(_)
        ));
        assert!(resolve("javascript:alert(1)", Some(p)).is_err());
    }
}

#[cfg(test)]
mod completion_tests {
    use super::*;
    #[test]
    fn completion_is_relative_bounded_and_preserves_unicode_cursor_offsets() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("notes")).unwrap();
        std::fs::write(dir.path().join("notes/coffee budget.md"), "coffee").unwrap();
        let mut e = editio::Editor::new(editio::buffer::Buffer::new(""));
        e.buffer.path = Some(dir.path().join("index.md"));
        e.mode = editio::Mode::Edit;
        e.buffer.insert("猫 [coffee](notes/co");
        let c = completion(&e).unwrap();
        assert_eq!(e.buffer.text.slice(c.range.clone()).to_string(), "notes/co");
        assert_eq!(complete(&c).unwrap(), vec!["notes/coffee%20budget.md"]);
        e.buffer.insert(":");
        assert!(completion(&e).is_none());
    }
}
