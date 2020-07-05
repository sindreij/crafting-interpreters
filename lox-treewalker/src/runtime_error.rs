use crate::{
    error_reporter::format_err,
    token::{Token, TokenType},
};

#[derive(Debug)]
pub struct RuntimeError {
    token: Token,
    message: String,
}

impl RuntimeError {
    pub fn new(token: Token, message: impl Into<String>) -> RuntimeError {
        RuntimeError {
            token,
            message: message.into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, RuntimeError>;

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.token.typ == TokenType::EOF {
            write!(
                f,
                "{}",
                format_err(self.token.line, " at end", &self.message)
            )
        } else {
            write!(
                f,
                "{}",
                format_err(
                    self.token.line,
                    &format!(" at '{}'", self.token.lexeme),
                    &self.message
                )
            )
        }
    }
}

impl std::error::Error for RuntimeError {}
