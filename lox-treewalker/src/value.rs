use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    ast::Stmt, environment::Environment, interpreter::Interpreter, runtime_error::RuntimeError,
    token::Token,
};

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
    Function(Rc<Function>),
    Class(Rc<Class>),
    Instance(Rc<Instance>),
}

pub struct Function {
    pub closure: Rc<RefCell<Environment>>,
    pub name: String,
    pub params: Vec<Token>,
    pub body: Vec<Stmt>,
}

pub struct Class {
    name: String,
    methods: HashMap<String, Rc<Function>>,
}

impl Class {
    pub fn new(name: &str, methods: HashMap<String, Rc<Function>>) -> Self {
        Self {
            name: name.to_owned(),
            methods,
        }
    }
}

pub struct Instance {
    class: Rc<Class>,
    fields: RefCell<HashMap<String, Value>>,
}

impl Instance {
    pub fn new(class: Rc<Class>) -> Self {
        Self {
            class,
            fields: RefCell::new(HashMap::new()),
        }
    }

    pub fn get(&self, name: &Token) -> Result<Value, RuntimeError> {
        if let Some(value) = self.fields.borrow().get(&name.lexeme) {
            Ok(value.clone())
        } else if let Some(method) = self.class.methods.get(&name.lexeme) {
            Ok(Value::Function(method.clone()))
        } else {
            Err(RuntimeError::new(
                name.clone(),
                format!("Undefined property '{}'", name.lexeme),
            ))
        }
    }

    pub fn set(&self, name: &Token, value: Value) {
        self.fields.borrow_mut().insert(name.lexeme.clone(), value);
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(val) => write!(f, "{}", val),
            Value::Bool(val) => write!(f, "{}", val),
            Value::Number(val) => write!(f, "{}", val),
            Value::Nil => write!(f, "nil"),
            Value::BuiltinCallable { .. } => write!(f, "[Builtin callable]"),
            Value::Function(function) => write!(f, "[Function {}]", function.name),
            Value::Class(class) => write!(f, "[Class {}]", class.name),
            Value::Instance(instance) => write!(f, "[Instance of Class {}]", instance.class.name),
        }
    }
}
