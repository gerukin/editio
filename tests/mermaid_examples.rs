use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
#[test]
fn mermaid_feature_examples_match_reviewable_unicode_snapshots() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut count = 0;
    for entry in std::fs::read_dir(root.join("examples/markdown")).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_stem().unwrap().to_str().unwrap();
        if !("13-".."21-").contains(&name) {
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap();
        let mut code = None;
        let mut index = 0;
        for event in Parser::new(&source) {
            match event {
                Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info)))
                    if info.as_ref() == "mermaid" =>
                {
                    code = Some(String::new())
                }
                Event::Text(text) if code.is_some() => code.as_mut().unwrap().push_str(&text),
                Event::End(TagEnd::CodeBlock) if code.is_some() => {
                    index += 1;
                    count += 1;
                    let input = code.take().unwrap();
                    let result = merman::ascii::HeadlessAsciiRenderer::new()
                        .with_strict_parsing()
                        .with_ascii_options(merman::ascii::AsciiRenderOptions::unicode())
                        .render_ascii_sync(&input);
                    if name.starts_with("20-") {
                        assert!(
                            result.is_err(),
                            "{name}/{index} should be an explicit unsupported case"
                        );
                        continue;
                    }
                    let text = result
                        .unwrap_or_else(|e| panic!("{name}/{index}: {e}\n{input}"))
                        .unwrap();
                    assert!(!text.trim().is_empty(), "{name}/{index}");
                    let target = root.join(format!("tests/snapshots/{name}-{index}.txt"));
                    if std::env::var_os("UPDATE_MERMAID_SNAPSHOTS").is_some() {
                        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
                        std::fs::write(&target, &text).unwrap();
                    }
                    assert_eq!(
                        text,
                        std::fs::read_to_string(&target).unwrap(),
                        "{name}/{index}"
                    );
                }
                _ => {}
            }
        }
    }
    assert!(count >= 30, "feature gallery should remain comprehensive");
}
