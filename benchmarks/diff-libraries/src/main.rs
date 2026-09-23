use std::{
    hint::black_box,
    ops::Range,
    time::{Duration, Instant},
};
use tapp_ui_renderers::diff;

type Changes = Vec<(Range<usize>, Range<usize>)>;

fn lines(mut s: &str) -> impl Iterator<Item = &str> {
    std::iter::from_fn(move || {
        if s.is_empty() {
            return None;
        }
        let end = s
            .char_indices()
            .find_map(|(i, c)| {
                matches!(
                    c,
                    '\n' | '\r' | '\u{b}' | '\u{c}' | '\u{85}' | '\u{2028}' | '\u{2029}'
                )
                .then(|| {
                    i + c.len_utf8()
                        + usize::from(c == '\r' && s.as_bytes().get(i + 1) == Some(&b'\n'))
                })
            })
            .unwrap_or(s.len());
        let (line, rest) = s.split_at(end);
        s = rest;
        Some(line)
    })
}

fn calculate(engine: &str, old: &str, new: &str) -> Result<Changes, &'static str> {
    if engine == "custom" {
        return diff::analyze(old, new)
            .map(|a| a.changes.into_iter().map(|c| (c.old, c.new)).collect());
    }
    let deadline = Instant::now() + Duration::from_millis(100);
    if engine.starts_with("imara") {
        let mut input = imara_diff::InternedInput::default();
        input.update_before(lines(old));
        input.update_after(lines(new));
        let mut d = imara_diff::Diff::compute(imara_diff::Algorithm::Histogram, &input);
        if engine != "imara-raw" {
            d.postprocess_lines(&input);
        }
        Ok(d.hunks()
            .map(|h| {
                (
                    h.before.start as usize..h.before.end as usize,
                    h.after.start as usize..h.after.end as usize,
                )
            })
            .collect())
    } else {
        let old: Vec<_> = lines(old).collect();
        let new: Vec<_> = lines(new).collect();
        let algo = if engine == "similar-histogram" {
            similar::Algorithm::Histogram
        } else {
            similar::Algorithm::Myers
        };
        Ok(
            similar::capture_diff_slices_deadline(algo, &old, &new, Some(deadline))
                .into_iter()
                .filter(|op| op.tag() != similar::DiffTag::Equal)
                .map(|op| (op.old_range(), op.new_range()))
                .collect(),
        )
    }
}

fn fixture(case: &str) -> (String, String) {
    let count = match case {
        "small" => 2000,
        "dense" => 4000,
        "long-lines" => 100,
        "large" => 80000,
        _ => 35000,
    };
    let old: String = match case {
        "repeated" => "same repeated line\n".repeat(count),
        "ambiguous" => (0..count).map(|i| format!("record {}\n", i % 64)).collect(),
        "long-lines" => (0..count)
            .map(|i| format!("{i}:{}\n", "abcdef0123456789".repeat(1000)))
            .collect(),
        "real-source" => std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../tapp-ui/crates/editor/src/lib.rs"
        ))
        .unwrap(),
        "unicode" => "日本語 café 👩‍💻\r\nfirst\u{2028}last\n".repeat(2000),
        _ => (0..count)
            .map(|i| format!("let value_{i} = {i}; // record data\n"))
            .collect(),
    };
    let new = match case {
        "identical" => old.clone(),
        "dense" => (0..count)
            .map(|i| format!("let replacement_{i} = {i};\n"))
            .collect(),
        "repeated" => "entirely different\n".repeat(count),
        "ambiguous" => (0..count)
            .map(|i| format!("record {}\n", (i * 17 + 13) % 64))
            .collect(),
        "reversed" => lines(&old).rev_collect(),
        "moved" => {
            let v: Vec<_> = lines(&old).collect();
            [v[count / 2..].concat(), v[..count / 2].concat()].concat()
        }
        "long-lines" => old.replacen("abcdef", "UVWXYZ", 1),
        "real-source" => old
            .replace("Mode::View", "Mode::Preview")
            .replace("pub fn ", "pub(crate) fn "),
        "unicode" => old.replacen("first", "changed", 1),
        _ => old
            .replace("value_100 =", "edited_100 =")
            .replace("value_1500 =", "edited_1500 ="),
    };
    (old, new)
}
trait ReverseCollect<'a>: Iterator<Item = &'a str> + Sized {
    fn rev_collect(self) -> String {
        let mut v: Vec<_> = self.collect();
        v.reverse();
        v.concat()
    }
}
impl<'a, T: Iterator<Item = &'a str>> ReverseCollect<'a> for T {}

fn verify(old: &str, new: &str, changes: &Changes) -> usize {
    let a: Vec<_> = lines(old).collect();
    let b: Vec<_> = lines(new).collect();
    let (mut ai, mut bi, mut edits) = (0, 0, 0);
    for (x, y) in changes {
        assert!(x.start >= ai && y.start >= bi && x.end <= a.len() && y.end <= b.len());
        assert_eq!(&a[ai..x.start], &b[bi..y.start]);
        ai = x.end;
        bi = y.end;
        edits += x.len() + y.len();
    }
    assert_eq!(&a[ai..], &b[bi..]);
    edits
}
fn rss() -> usize {
    std::fs::read_to_string("/proc/self/status")
        .unwrap()
        .lines()
        .find(|l| l.starts_with("VmHWM:"))
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap()
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let engine = &args[1];
    let case = &args[2];
    let (old, new) = fixture(case);
    let before = rss();
    let mut first_peak = before;
    let mut samples = Vec::new();
    let mut failures = 0;
    let mut blocks = 0;
    let mut edits = 0;
    for i in 0..34 {
        let start = Instant::now();
        let result = black_box(calculate(engine, black_box(&old), black_box(&new)));
        let elapsed = start.elapsed().as_secs_f64() * 1000.;
        if i == 0 {
            first_peak = rss();
        }
        if i >= 3 {
            samples.push(elapsed);
        }
        match result {
            Ok(changes) => {
                edits = verify(&old, &new, &changes);
                blocks = changes.len();
            }
            Err(_) => failures += 1,
        }
    }
    samples.sort_by(f64::total_cmp);
    println!(
        "{engine},{case},{},{},{:.4},{:.4},{},{},{blocks},{edits},{failures}",
        old.len(),
        new.len(),
        samples[15],
        samples[29],
        before,
        first_peak
    );
}
