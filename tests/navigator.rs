use tapp_ui::renderers::{navigation::Index, presentation};
#[test]
fn essay_outline_and_anchors_agree_with_renderer_including_hidden_sections() {
    let source = include_str!("../examples/markdown/35-ai-human-resources.md");
    let index = Index::build(source).unwrap();
    let (_, rendered_headings, details) = presentation::prepare(source, &Default::default());
    let slugs: Vec<_> = index
        .entries
        .iter()
        .filter_map(|entry| entry.slug.as_deref())
        .collect();
    assert_eq!(
        slugs,
        rendered_headings
            .iter()
            .map(|entry| entry.slug.as_str())
            .collect::<Vec<_>>()
    );
    assert!(slugs.contains(&"human-oversight-1"));
    assert!(slugs.contains(&"a-puddle-named-barbara"));
    assert_eq!(slugs.len(), 19);
    for entry in &index.entries {
        if let Some(fragment) = entry.url.as_deref().and_then(|url| url.strip_prefix('#')) {
            assert!(slugs.contains(&fragment), "missing {fragment}");
        }
        assert!(!entry.title.contains("Decoy"));
    }
    for level in 1..=6 {
        assert!(index.entries.iter().any(|entry| entry.level == level));
    }
    for detail in index.details {
        assert!(details.contains_key(&detail.key));
    }
}
#[test]
fn dot_closed_frontmatter_and_unicode_offsets_are_supported() {
    let source = "\u{feff}---\n# not a heading\n...\n\n# 猫\n\n<details>\n<summary>Hidden</summary>\n\n## 犬\n\n</details>\n";
    let index = Index::build(source).unwrap();
    let headings: Vec<_> = index
        .entries
        .iter()
        .filter(|entry| entry.level > 0)
        .collect();
    assert_eq!(headings.len(), 2);
    assert!(source[headings[1].source..].starts_with("## 犬"));
    assert_eq!(index.details[0].key, 1);
}
