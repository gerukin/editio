//! Local release evidence: cargo run --release --example measure_navigation
use std::time::Instant;
use tapp_ui::renderers::navigation::Index;
fn rss() -> String {
    std::fs::read_to_string("/proc/self/status")
        .unwrap_or_default()
        .lines()
        .find(|line| line.starts_with("VmHWM:"))
        .unwrap_or("peak RSS unavailable")
        .to_owned()
}
fn main() {
    for (name, text) in [
        (
            "essay",
            include_str!("markdown/35-ai-human-resources.md").to_owned(),
        ),
        (
            "2000 headings and links",
            (0..2000)
                .map(|i| format!("## Section {i}\n\n[reference](#section-{i})\n\n"))
                .collect(),
        ),
        ("10000 repeated headings", "# Same\n\n".repeat(10000)),
        (
            "2 MiB mostly prose",
            format!("# Start\n\n{}", "ordinary prose\n\n".repeat(125000)),
        ),
        ("oversized", "x".repeat(2 * 1024 * 1024 + 1)),
    ] {
        let before = rss();
        let start = Instant::now();
        let result = Index::build(&text);
        println!(
            "{name}: {} bytes, {:?}, {} -> {}, {}",
            text.len(),
            start.elapsed(),
            before,
            rss(),
            result
                .as_ref()
                .map(|i| format!("{} entries", i.entries.len()))
                .unwrap_or_else(|e| e.clone())
        );
    }
}
