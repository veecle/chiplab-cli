fn main() {
    println!("{}", greeting());
}

fn greeting() -> &'static str {
    "Hello, world!"
}

#[cfg(test)]
mod tests {
    use super::greeting;

    #[test]
    fn greeting_is_not_empty() {
        assert!(!greeting().is_empty());
    }
}
