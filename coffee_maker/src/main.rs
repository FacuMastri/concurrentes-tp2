use util::hello_world;

fn main() {
    print!("{}", hello_world());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_world() {
        assert_eq!(hello_world(), "Hello World");
    }
}
