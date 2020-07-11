use crate::{
    ast::{Expr, Literal, Stmt, StmtFunction},
    error_reporter::ErrorReporter,
    interpreter::Interpreter,
    token::Token,
    value::Value,
};
use std::collections::HashMap;

pub struct Resolver<'a> {
    interpreter: &'a mut Interpreter,
    scopes: Vec<HashMap<String, bool>>,
    errors: &'a mut ErrorReporter,
    current_function: FunctionType,
    current_class: ClassType,
}

#[derive(Clone, Copy, Debug)]
enum FunctionType {
    None,
    Function,
    Initializer,
    Method,
}

#[derive(Clone, Copy, Debug)]
enum ClassType {
    None,
    Class,
}

impl<'a> Resolver<'a> {
    pub fn new(interpreter: &'a mut Interpreter, errors: &'a mut ErrorReporter) -> Self {
        Resolver {
            interpreter,
            scopes: Vec::new(),
            errors,
            current_function: FunctionType::None,
            current_class: ClassType::None,
        }
    }

    pub fn resolve(&mut self, statements: &[Stmt]) {
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
            Stmt::Class { name, methods } => {
                let enclosing_class = self.current_class;
                self.current_class = ClassType::Class;
                self.declare(name);
                self.define(name);

                self.begin_scope();
                self.scopes
                    .last_mut()
                    .unwrap()
                    .insert("this".to_owned(), true);

                for method in methods {
                    let declaration = if method.name.lexeme == "init" {
                        FunctionType::Initializer
                    } else {
                        FunctionType::Method
                    };
                    self.resolve_function(method, declaration);
                }

                self.end_scope();

                self.current_class = enclosing_class;
            }
            Stmt::Expression(stmt) => self.resolve_expr(stmt),
            Stmt::Function(fun) => {
                self.declare(&fun.name);
                self.define(&fun.name);
                self.resolve_function(fun, FunctionType::Function);
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.resolve_expr(condition);
                self.resolve_stmt(then_branch);
                if let Some(else_branch) = else_branch {
                    self.resolve_stmt(else_branch);
                }
            }
            Stmt::Return { value, keyword } => {
                if let FunctionType::None = self.current_function {
                    self.errors
                        .error(keyword.line, "Cannot return from top-level code".to_owned())
                }
                if let FunctionType::Initializer = self.current_function {
                    if value != &Expr::Literal(Literal::Nil) {
                        self.errors.error(
                            keyword.line,
                            "Cannot return from inside an initiaizer.".to_owned(),
                        );
                    }
                }
                self.resolve_expr(value)
            }
            Stmt::Print(stmt) => self.resolve_expr(stmt),
            Stmt::Var { name, initializer } => {
                self.declare(name);
                if let Some(initializer) = initializer {
                    self.resolve_expr(initializer);
                }
                self.define(name)
            }
            Stmt::While { condition, body } => {
                self.resolve_expr(condition);
                self.resolve_stmt(body);
            }
        }
    }

    fn resolve_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Assign {
                expr_id,
                name,
                value,
            } => {
                self.resolve_expr(value);
                self.resolve_local(*expr_id, name);
            }
            Expr::Binary { left, right, .. } => {
                self.resolve_expr(left);
                self.resolve_expr(right);
            }
            Expr::Call {
                callee, arguments, ..
            } => {
                self.resolve_expr(callee);
                for arg in arguments {
                    self.resolve_expr(arg);
                }
            }
            Expr::Get { object, .. } => self.resolve_expr(object),
            Expr::Grouping(expr) => self.resolve_expr(expr),
            Expr::Literal(..) => { /* Nothing to do */ }
            Expr::Logical { left, right, .. } => {
                self.resolve_expr(left);
                self.resolve_expr(right);
            }
            Expr::Unary { right, .. } => self.resolve_expr(right),
            Expr::Variable { name, expr_id } => {
                if let Some(scope) = self.scopes.last() {
                    if scope.get(&name.lexeme) == Some(&false) {
                        self.errors.error(
                            name.line,
                            "Cannot read local variable in its own initializer".to_owned(),
                        );
                    }
                }

                self.resolve_local(*expr_id, name)
            }
            Expr::Set { object, value, .. } => {
                self.resolve_expr(value);
                self.resolve_expr(object);
            }
            Expr::This { keyword, expr_id } => {
                if let ClassType::None = self.current_class {
                    self.errors.error(
                        keyword.line,
                        "Cannout use 'this' outside of a class".to_owned(),
                    );
                    return;
                }

                self.resolve_local(*expr_id, keyword);
            }
        }
    }

    fn resolve_function(&mut self, fun: &StmtFunction, typ: FunctionType) {
        let enclosing_function = self.current_function;
        self.current_function = typ;
        self.begin_scope();
        for param in &fun.params {
            self.declare(param);
            self.define(param);
        }
        self.resolve(&fun.body);
        self.end_scope();
        self.current_function = enclosing_function;
    }

    fn resolve_local(&mut self, expr_id: usize, name: &Token) {
        for (index, scope) in self.scopes.iter().enumerate() {
            if scope.contains_key(&name.lexeme) {
                self.interpreter
                    .resolve(expr_id, self.scopes.len() - 1 - index);
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
