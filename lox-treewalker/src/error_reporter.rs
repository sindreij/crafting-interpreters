pub struct ErrorReporter {
    pub had_error: bool,
}

impl ErrorReporter {
    pub fn error(&mut self, line: u32, message: String) {
        self.report(line, "", &message);
    }

    fn report(&mut self, line: u32, location: &str, message: &str) {
        println!("{}", format_err(line, location, message));
        self.had_error = true;
    }
}

pub fn format_err(line: u32, location: &str, message: &str) -> String {
    format!(
        "[line {line}] Error{location}: {message}",
        line = line,
        location = location,
        message = message
    )
}
