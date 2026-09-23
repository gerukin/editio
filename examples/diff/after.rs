fn welcome(name: &str) -> String {
    format!("Welcome, {name}!")
}

fn main() {
    let guests = ["Tokyo", "Paris", "New York"];
    for guest in guests {
        println!("{}", welcome(guest));
    }
}
