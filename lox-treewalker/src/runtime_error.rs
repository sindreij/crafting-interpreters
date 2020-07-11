use crate::{
    error_reporter::format_err,
    token::{Token, TokenType},
    value::Value,
};

#[derive(Debug)]
pub enum RuntimeError {
    Error { token: Token, message: String },
    Return(Value),
}

impl RuntimeError {
    pub fn new(token: Token, message: impl Into<String>) -> RuntimeError {
        RuntimeError::Error {
            token,
            message: message.into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, RuntimeError>;

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::Error { token, message } => {
                if token.typ == TokenType::EOF {
                    write!(f, "{}", format_err(token.line, " at end", &message))
                } else {
                    write!(
                        f,
                        "{}",
                        format_err(token.line, &format!(" at '{}'", token.lexeme), &message)
                    )
                }
            }
            RuntimeError::Return(..) => write!(f, "Return"),
        }
    }
}

impl std::error::Error for RuntimeError {}
