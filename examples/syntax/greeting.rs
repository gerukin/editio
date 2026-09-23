// Display-only Rust syntax sample.
fn greet(name: &str) -> String {
    format!("Hello, {name}!")
}

fn main() {
    let names = ["Ada", "Grace"];
    for name in names { println!("{}", greet(name)); }
}
