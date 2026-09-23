//! Local release measurements. Run each case separately for meaningful peak RSS.
use editio::{Editor, Mode, buffer::Buffer};
use ratatui::{Terminal, backend::TestBackend};
use std::time::{Duration, Instant};
use tapp_ui::renderers::{RenderOptions, diff};
fn memory() -> String {
    std::fs::read_to_string("/proc/self/status")
        .unwrap_or_default()
        .lines()
        .filter(|l| l.starts_with("VmRSS:") || l.starts_with("VmHWM:"))
        .collect::<Vec<_>>()
        .join("; ")
}
fn main() {
    let case = std::env::args().nth(1).unwrap_or_else(|| "small".into());
    println!("case={case}; baseline {}", memory());
    if case == "plain" {
        let source = "ordinary text with no diff marks\n".repeat(2000);
        let mut b = Buffer::new(&source);
        b.path = Some("plain.txt".into());
        let mut e = Editor::new(b);
        e.mode = Mode::Edit;
        e.wrap = false;
        let mut t = Terminal::new(TestBackend::new(80, 24)).unwrap();
        t.draw(|f| e.draw(f, f.area())).unwrap();
        let start = Instant::now();
        for _ in 0..2000 {
            t.draw(|f| e.draw(f, f.area())).unwrap();
        }
        println!(
            "2000 ordinary frames {:?}; worker={}; {}",
            start.elapsed(),
            e.has_background_work(),
            memory()
        );
        let start = Instant::now();
        for _ in 0..2000 {
            assert!(!e.is_diff());
        }
        println!("2000 cached detection calls {:?}", start.elapsed());
        let start = Instant::now();
        for _ in 0..200 {
            e.buffer.insert("a");
            assert!(!e.is_diff());
            e.buffer.undo();
        }
        println!(
            "200 edit+detect+undo revisions {:?}; {}",
            start.elapsed(),
            memory()
        );
        return;
    }
    let count = match case.as_str() {
        "small" => 2000,
        "medium" => 35_000,
        "dense" => 4000,
        "repeated" => 35_000,
        "ambiguous" => 35_000,
        "large" => 80_000,
        _ => 2000,
    };
    let old = if case == "ambiguous" {
        (0..count).map(|i| format!("record {}\n", i % 64)).collect()
    } else if case == "repeated" {
        "same repeated line\n".repeat(count)
    } else {
        (0..count)
            .map(|n| format!("let value_{n} = {n}; // record data\n"))
            .collect()
    };
    let new = if case == "ambiguous" {
        (0..count)
            .map(|i| format!("record {}\n", (i * 17 + 13) % 64))
            .collect()
    } else if case == "repeated" {
        "entirely different\n".repeat(count)
    } else if case == "dense" {
        (0..count)
            .map(|n| format!("let replacement_{n} = {n};\n"))
            .collect()
    } else {
        old.replace("value_100 =", "edited_100 =")
            .replace("value_1500 =", "edited_1500 =")
    };
    let start = Instant::now();
    let result = diff::analyze(&old, &new);
    println!(
        "bytes={}/{} lines={count}; analyze {:?}; {}",
        old.len(),
        new.len(),
        start.elapsed(),
        memory()
    );
    match result {
        Ok(a) => {
            println!("blocks={} coarse={}", a.changes.len(), a.coarse);
            let edited = new.replacen("record data", "changed data", 1);
            let start = Instant::now();
            let updated = diff::update(&old, &new, &edited, &a);
            println!(
                "incremental {:?}; success={}; {}",
                start.elapsed(),
                updated.is_ok(),
                memory()
            );
            let start = Instant::now();
            let p = diff::comparison_preview(
                &new,
                "rust",
                &a,
                false,
                79,
                &RenderOptions {
                    wrap: false,
                    ..Default::default()
                },
            );
            println!(
                "prepare {:?}; rows={}; notice={:?}; {}",
                start.elapsed(),
                p.lines.len(),
                p.notice,
                memory()
            );
        }
        Err(e) => println!("fallback={e}"),
    }
    assert!(diff::analyze_with_budget("a", "b", Duration::ZERO).is_err());
}
