use crate::{
    ast::{Expr, Stmt},
    error_reporter::ErrorReporter,
    interpreter::Interpreter,
    token::Token,
};
use std::collections::HashMap;

pub struct Resolver<'a> {
    interpreter: &'a mut Interpreter,
    scopes: Vec<HashMap<String, bool>>,
    errors: ErrorReporter,
}

impl<'a> Resolver<'a> {
    fn new(interpreter: &'a mut Interpreter, errors: ErrorReporter) -> Self {
        Resolver {
            interpreter,
            scopes: Vec::new(),
            errors,
        }
    }

    fn resolve(&mut self, statements: &[Stmt]) {
        for statement in statements {
            self.resolve_stmt(statement);
        }
    }

    fn resolve_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Block(statements) => {
                self.begin_scope();
                self.resolve(statements);
                self.end_scope();
            }
            Stmt::Expression(_) => {}
            Stmt::Function { name, params, body } => {
                self.declare(name);
                self.define(name);
                self.resolve_function(name, params, body);
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {}
            Stmt::Return { keyword, value } => {}
            Stmt::Print(_) => {}
            Stmt::Var { name, initializer } => {
                self.declare(name);
                if let Some(initializer) = initializer {
                    self.resolve_expr(initializer);
                }
                self.define(name)
            }
            Stmt::While { condition, body } => {}
        }
    }

    fn resolve_function(&mut self, name: &Token, params: &[Token], body: &[Stmt]) {
        self.begin_scope();
        for param in params {
            self.declare(param);
            self.define(param);
        }
        self.resolve(body);
        self.end_scope();
    }

    fn resolve_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Assign { name, value } => {
                self.resolve_expr(value);
                self.resolve_local(expr, name);
            }
            Expr::Binary {
                left,
                operator,
                right,
            } => {}
            Expr::Call {
                callee,
                paren,
                arguments,
            } => {}
            Expr::Grouping(_) => {}
            Expr::Literal(_) => {}
            Expr::Logical {
                left,
                operator,
                right,
            } => {}
            Expr::Unary { operator, right } => {}
            Expr::Variable { name } => {
                if let Some(scope) = self.scopes.last() {
                    if scope.get(&name.lexeme) == Some(&false) {
                        self.errors.error(
                            name.line,
                            "Cannot read local variable in its own initializer".to_owned(),
                        );
                    }
                }

                self.resolve_local(expr, name)
            }
        }
    }

    fn resolve_local(&mut self, expr: &Expr, name: &Token) {
        for (index, scope) in itertools::rev(&self.scopes).enumerate() {
            if scope.contains_key(&name.lexeme) {
                self.interpreter
                    .resolve(expr, self.scopes.len() - 1 - index);
                return;
            }
        }
    }

    fn declare(&mut self, name: &Token) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.lexeme.clone(), false);
        }
    }

    fn define(&mut self, name: &Token) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.lexeme.clone(), true);
        }
    }

    fn begin_scope(&mut self) {
        self.scopes.push(HashMap::new())
    }

    fn end_scope(&mut self) {
        self.scopes.pop();
    }
}
