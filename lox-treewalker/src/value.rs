use crate::{ast::Stmt, interpreter::Interpreter, token::Token};

#[derive(Clone)]
pub enum Value {
    String(String),
    Bool(bool),
    Number(f64),
    Nil,
    BuiltinCallable {
        arity: usize,
        fun: fn(intepreter: &mut Interpreter, arguments: Vec<Value>) -> Value,
    },
    Function {
        name: String,
        params: Vec<Token>,
        body: Vec<Stmt>,
    },
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(val) => write!(f, "{}", val),
            Value::Bool(val) => write!(f, "{}", val),
            Value::Number(val) => write!(f, "{}", val),
            Value::Nil => write!(f, "nil"),
            Value::BuiltinCallable { .. } => write!(f, "[Builtin callable]"),
            Value::Function { name, .. } => write!(f, "[Function {}]", name),
        }
    }
}
