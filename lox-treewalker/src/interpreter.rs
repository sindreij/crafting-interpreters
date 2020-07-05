use crate::{
    ast::{Expr, Literal, Stmt},
    error_reporter::format_err,
    token::{Token, TokenType},
};

#[derive(Debug, PartialEq)]
pub enum Value {
    String(String),
    Bool(bool),
    Number(f64),
    Nil,
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(val) => write!(f, "{}", val),
            Value::Bool(val) => write!(f, "{}", val),
            Value::Number(val) => write!(f, "{}", val),
            Value::Nil => write!(f, "nil"),
        }
    }
}

#[derive(Debug)]
pub struct RuntimeError {
    token: Token,
    message: &'static str,
}

impl RuntimeError {
    fn new(token: Token, message: &'static str) -> RuntimeError {
        RuntimeError { token, message }
    }
}

type Result<T> = std::result::Result<T, RuntimeError>;

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

pub struct Interpreter;

impl Interpreter {
    pub fn new() -> Self {
        Interpreter
    }

    pub fn interpret(&mut self, statements: &[Stmt]) -> Result<()> {
        for stmt in statements {
            self.execute(stmt)?;
        }
        Ok(())
    }

    fn execute(&mut self, stmt: &Stmt) -> Result<()> {
        match stmt {
            Stmt::Expression(expr) => {
                self.evaluate(expr)?;
            }
            Stmt::Print(expr) => {
                let value = self.evaluate(expr)?;
                println!("{}", value);
            }
        }

        Ok(())
    }

    fn evaluate(&mut self, expr: &Expr) -> Result<Value> {
        Ok(match expr {
            Expr::Literal(literal) => match literal {
                Literal::Bool(value) => Value::Bool(*value),
                Literal::String(value) => Value::String(value.clone()),
                Literal::Number(value) => Value::Number(*value),
                Literal::Nil => Value::Nil,
            },
            Expr::Binary {
                left,
                operator,
                right,
            } => {
                let left = self.evaluate(left)?;
                let right = self.evaluate(right)?;

                use TokenType::*;

                match (left, right) {
                    (Value::String(left), Value::String(right)) => match &operator.typ {
                        Plus => Value::String(left + &right),
                        Greater => Value::Bool(left > right),
                        GreaterEqual => Value::Bool(left >= right),
                        Less => Value::Bool(left < right),
                        LessEqual => Value::Bool(left <= right),
                        BangEqual => Value::Bool(left != right),
                        EqualEqual => Value::Bool(left == right),

                        _ => Err(RuntimeError::new(
                            operator.clone(),
                            "I can't do that operation on two strings",
                        ))?,
                    },
                    (Value::Number(left), Value::Number(right)) => match &operator.typ {
                        Plus => Value::Number(left + right),
                        Minus => Value::Number(left - right),
                        Star => Value::Number(left * right),
                        Slash => Value::Number(left / right),
                        Greater => Value::Bool(left > right),
                        GreaterEqual => Value::Bool(left >= right),
                        Less => Value::Bool(left < right),
                        LessEqual => Value::Bool(left <= right),
                        BangEqual => Value::Bool(left != right),
                        EqualEqual => Value::Bool(left == right),

                        _ => Err(RuntimeError::new(
                            operator.clone(),
                            "I can't do that operation on two numbers",
                        ))?,
                    },
                    (Value::Bool(left), Value::Bool(right)) => match &operator.typ {
                        BangEqual => Value::Bool(left != right),
                        EqualEqual => Value::Bool(left == right),

                        _ => Err(RuntimeError::new(
                            operator.clone(),
                            "I can't do that operation on two booleans",
                        ))?,
                    },
                    (Value::Nil, Value::Nil) => match &operator.typ {
                        BangEqual => Value::Bool(false),
                        EqualEqual => Value::Bool(true),
                        _ => Err(RuntimeError::new(
                            operator.clone(),
                            "I can't do that operation on two 'NIL'",
                        ))?,
                    },
                    _ => match &operator.typ {
                        BangEqual => Value::Bool(true),
                        EqualEqual => Value::Bool(false),
                        _ => Err(RuntimeError::new(
                            operator.clone(),
                            "I can't do that operation on two values with different type",
                        ))?,
                    },
                }
            }
            Expr::Grouping(expr) => self.evaluate(expr)?,
            Expr::Unary { operator, right } => {
                let right = self.evaluate(&right)?;
                match operator.typ {
                    TokenType::Minus => match right {
                        Value::Number(value) => Value::Number(-value),
                        _ => {
                            panic!("Tried to use unary operator on something that is not a number")
                        }
                    },
                    TokenType::Bang => Value::Bool(!is_truthy(&right)),
                    _ => panic!("Invalid type for unary -, {}", operator),
                }
            }
        })
    }
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Bool(value) => *value,
        Value::Nil => false,
        _ => true,
    }
}
