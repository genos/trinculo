pub fn quote() -> &'static str {
    "What have we here? A man or a fish? Dead or alive?"
}

pub fn quote_binary() -> &'static [u8] {
    b"What have we here? A man or a fish? Dead or alive?"
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quote_test() {
        insta::assert_snapshot!(quote(), @"What have we here? A man or a fish? Dead or alive?");
    }

    #[test]
    fn quote_binary_test() {
        insta::assert_binary_snapshot!("quote.bin", quote_binary().to_vec());
    }
}
