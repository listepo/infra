//! Binary-only member of the ci-rust.yml fixture workspace.

fn greeting() -> &'static str {
    "hello"
}

fn main() {
    println!("{}", greeting());
}

#[cfg(test)]
mod tests {
    #[test]
    fn greets() {
        assert_eq!(super::greeting(), "hello");
    }
}
