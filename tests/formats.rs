use editio::{Editor, Mode, buffer::Buffer};
use std::path::PathBuf;

#[test]
fn explicit_format_controls_rendering_and_preferences_without_a_fake_save_path() {
    for (alias, kind, rendered) in [
        ("md", "markdown", true),
        ("markdown", "markdown", true),
        ("json", "json", true),
        ("csv", "csv", true),
        ("tsv", "tsv", true),
        ("plaintext", "plain text", false),
        ("text", "plain text", false),
        ("rs", "rust", false),
        ("yml", "yaml", false),
    ] {
        let mut editor =
            Editor::with_format(Buffer::new("# detected as markdown\n"), alias).unwrap();
        editor.mode = Mode::View;
        assert!(editor.buffer.path.is_none());
        assert!(!editor.buffer.dirty());
        assert_eq!(editor.preference_type(), kind, "{alias}");
        assert_eq!(editor.rendered_view(), rendered, "{alias}");
        editor.buffer.path = Some(PathBuf::from("saved.txt"));
        assert_eq!(
            editor.preference_type(),
            kind,
            "override survives saving: {alias}"
        );
        assert_eq!(editor.rendered_view(), rendered, "{alias}");
    }
    assert!(Editor::with_format(Buffer::new(""), "not-a-real-format").is_err());
}

#[test]
fn every_bundled_github_fence_alias_is_accepted() {
    for line in include_str!("../../tapp-ui/crates/renderers/assets/github-languages.tsv")
        .lines()
        .filter(|l| !l.starts_with('#'))
    {
        let alias = line.split('\t').next().unwrap();
        assert!(
            Editor::with_format(Buffer::new(""), alias).is_ok(),
            "{alias}"
        );
    }
}
