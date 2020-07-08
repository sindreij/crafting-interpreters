use std::{
    cell::RefCell,
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};

// TODO: Change to having environment as a parameter to the function

use crate::{
    ast::{Expr, Literal, Stmt},
    environment::Environment,
    runtime_error::{Result, RuntimeError},
    token::TokenType,
    value::Value,
};

pub struct Interpreter {
    environment: Rc<RefCell<Environment>>,
    globals: Rc<RefCell<Environment>>,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut globals = Environment::new();

        globals.define(
            "clock",
            Value::BuiltinCallable {
                arity: 0,
                fun: |_, _| {
                    Value::Number(
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .expect("time went backward!")
                            .as_millis() as f64,
                    )
                },
            },
        );

        let globals = Rc::new(RefCell::new(globals));

        Interpreter {
            environment: globals.clone(),
            globals,
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
            Stmt::Block(statements) => {
                self.execute_block(
                    statements,
                    Rc::new(RefCell::new(Environment::new_with_enclosing(
                        &self.environment,
                    ))),
                )?;
            }
            Stmt::Expression(expr) => {
                self.evaluate(expr)?;
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                if is_truthy(&self.evaluate(condition)?) {
                    self.execute(then_branch)?;
                } else if let Some(else_branch) = else_branch {
                    self.execute(else_branch)?;
                }
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

                self.environment.borrow_mut().define(&name.lexeme, value);
            }
            Stmt::While { condition, body } => {
                while is_truthy(&self.evaluate(condition)?) {
                    self.execute(body)?;
                }
            }
        }

        Ok(())
    }

    fn execute_block(
        &mut self,
        statements: &[Stmt],
        environment: Rc<RefCell<Environment>>,
    ) -> Result<()> {
        let previous = self.environment.clone();
        self.environment = environment;

        for statement in statements {
            if let Err(err) = self.execute(statement) {
                // Poor-mans finally. Should probably use drop in some way
                self.environment = previous;
                return Err(err);
            }
        }

        self.environment = previous;

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
            Expr::Variable { name } => self.environment.borrow().get(name)?,
            Expr::Assign { name, value } => {
                let value = self.evaluate(value)?;

                self.environment.borrow_mut().assign(name, value.clone())?;

                value
            }
            Expr::Logical {
                left,
                operator,
                right,
            } => {
                let left = self.evaluate(left)?;
                let return_left = match operator.typ {
                    TokenType::Or => is_truthy(&left),
                    TokenType::And => !is_truthy(&left),
                    _ => panic!("Invalid operator in Logical, {:?}", operator),
                };
                if return_left {
                    left
                } else {
                    self.evaluate(right)?
                }
            }
            Expr::Call {
                callee,
                paren,
                arguments,
            } => {
                let callee = self.evaluate(callee)?;

                let arguments = arguments
                    .iter()
                    .map(|arg| self.evaluate(arg))
                    .collect::<Result<Vec<_>>>()?;

                match callee {
                    // TODO: Implement things that are callable
                    Value::BuiltinCallable { arity, fun } => {
                        if arguments.len() != arity {
                            Err(RuntimeError::new(
                                paren.clone(),
                                format!(
                                    "Expected {} arguments, but got {}.",
                                    arity,
                                    arguments.len()
                                ),
                            ))?
                        }
                        fun(self, arguments)
                    }
                    _ => Err(RuntimeError::new(
                        paren.clone(),
                        "Can only call functions and classes.".to_owned(),
                    ))?,
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
