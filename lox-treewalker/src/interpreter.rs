use crate::{
    ast::{Expr, Literal, Stmt},
    environment::Environment,
    runtime_error::{Result, RuntimeError},
    token::TokenType,
    value::Value,
};

pub struct Interpreter {
    environment: Environment,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            environment: Environment::new(),
        }
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
            Stmt::Var { name, initializer } => {
                let value = initializer
                    .as_ref()
                    .map(|expr| self.evaluate(expr))
                    .unwrap_or(Ok(Value::Nil))?;

                self.environment.define(&name.lexeme, value);
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
                            "I can't do that operation on two strings".to_owned(),
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
            Expr::Variable { name } => self.environment.get(name)?,
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
