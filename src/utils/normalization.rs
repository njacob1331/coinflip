use std::borrow::Cow;

use regex::Regex;

pub struct Normalizer {
    regex: Regex,
}

impl Normalizer {
    pub fn new() -> Self {
        Self {
            regex: Regex::new(r"[^\p{L}\p{N}]+").expect("failed to build regex"),
        }
    }

    pub fn normalize(&self, input: &str) -> String {
        self.regex.replace_all(input, " ").to_string()
    }
}
