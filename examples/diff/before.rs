fn welcome(name: &str) -> String {
    format!("Hello, {name}")
}

fn old_helper() -> usize {
    42
}

fn main() {
    println!("{}", welcome("world"));
}
