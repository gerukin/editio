//! Preparation versus cached painting; no comparison engine is used for fences.
use ratatui::{Terminal, backend::TestBackend};
use std::time::Instant;
use tapp_ui::renderers::{
    RenderOptions,
    preview::ViewState,
    viewer::{Format, prepare},
};
fn main() {
    let language = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "diff-text".into());
    let text=format!("```{language}\n-old description with some repeated words\n+new description with some repeated words\n```\n\n").repeat(1000);
    let mut samples = Vec::new();
    for i in 0..11 {
        let start = Instant::now();
        let p = prepare(&text, &Format::Markdown, 80, &RenderOptions::default()).unwrap();
        let elapsed = start.elapsed();
        assert!(
            p.diff.is_none(),
            "fences cannot prepare a document comparison"
        );
        if i > 0 {
            samples.push(elapsed);
        }
    }
    samples.sort();
    let mut p = prepare(&text, &Format::Markdown, 80, &RenderOptions::default()).unwrap();
    let mut state = ViewState::default();
    let (mut scroll, mut horizontal) = (0, 0);
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let start = Instant::now();
    for _ in 0..1000 {
        terminal
            .draw(|f| {
                state.draw(
                    &mut p,
                    f,
                    f.area(),
                    &mut scroll,
                    &mut horizontal,
                    false,
                    false,
                    None,
                )
            })
            .unwrap();
    }
    println!(
        "{language}: {} bytes, 1000 fences; preparation median {:?}; 1000 cached frames {:?}",
        text.len(),
        samples[5],
        start.elapsed()
    );
}
