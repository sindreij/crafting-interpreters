pub struct Scanner<'a> {
    source: &'a str,
}

impl<'a> Scanner<'a> {
    pub fn new(source: &str) -> Scanner {
        Scanner { source }
    }

    pub fn scanTokens(&self) -> Vec<String> {
        unimplemented!()
    }
}
