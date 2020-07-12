use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    ast::Stmt,
    environment::{get_at, Environment},
    interpreter::Interpreter,
    runtime_error::RuntimeError,
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

impl Value {
    pub fn arity(&self, token: &Token) -> Result<usize, RuntimeError> {
        Ok(match &self {
            Value::Function(function) => function.arity(),
            Value::BuiltinCallable { arity, .. } => *arity,
            Value::Class(class) => match class.find_method("init") {
                Some(method) => method.arity(),
                None => 0,
            },
            _ => Err(RuntimeError::new(
                token.clone(),
                "Can only call functions and classes.".to_owned(),
            ))?,
        })
    }

    pub fn call(
        &self,
        interpreter: &mut Interpreter,
        token: &Token,
        arguments: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        let arity = self.arity(token)?;

        if arguments.len() != arity {
            Err(RuntimeError::new(
                token.clone(),
                format!("Expected {} arguments, but got {}.", arity, arguments.len()),
            ))?
        }

        Ok(match self {
            Value::Function(function) => function.call(interpreter, arguments)?,
            Value::Class(class) => {
                let instance = Rc::new(Instance::new(class.clone()));
                if let Some(initializer) = class.find_method("init") {
                    initializer
                        .bind(instance.clone())
                        .call(interpreter, arguments)?;
                }
                Value::Instance(instance)
            }
            Value::BuiltinCallable { fun, .. } => fun(interpreter, arguments),
            _ => Err(RuntimeError::new(
                token.clone(),
                "Can only call functions and classes.".to_owned(),
            ))?,
        })
    }
}

pub struct Function {
    pub closure: Rc<RefCell<Environment>>,
    pub name: String,
    pub params: Vec<Token>,
    pub body: Vec<Stmt>,
    pub is_initializer: bool,
}

impl Function {
    pub fn bind(&self, instance: Rc<Instance>) -> Self {
        let mut environment = Environment::new_with_enclosing(&self.closure);
        environment.define("this", Value::Instance(instance));
        Self {
            closure: Rc::new(RefCell::new(environment)),
            name: self.name.clone(),
            params: self.params.clone(),
            body: self.body.clone(),
            is_initializer: self.is_initializer,
        }
    }

    pub fn arity(&self) -> usize {
        self.params.len()
    }

    pub fn call(
        &self,
        interpreter: &mut Interpreter,
        arguments: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        let mut environment = Environment::new_with_enclosing(&self.closure);
        for (param, argument) in self.params.iter().zip(arguments) {
            environment.define(&param.lexeme, argument);
        }

        let result = match interpreter.execute_block(&self.body, Rc::new(RefCell::new(environment)))
        {
            Ok(()) => Value::Nil,
            Err(RuntimeError::Return(value)) => value,
            Err(err) => Err(err)?,
        };

        if self.is_initializer {
            Ok(get_at(self.closure.clone(), 0, "this"))
        } else {
            Ok(result)
        }
    }
}

pub struct Class {
    name: String,
    methods: HashMap<String, Rc<Function>>,
    superclass: Option<Rc<Class>>,
}

impl Class {
    pub fn new(
        name: &str,
        methods: HashMap<String, Rc<Function>>,
        superclass: Option<Rc<Class>>,
    ) -> Self {
        Self {
            name: name.to_owned(),
            methods,
            superclass,
        }
    }

    pub fn find_method(&self, name: &str) -> Option<Rc<Function>> {
        self.methods.get(name).cloned().or(self
            .superclass
            .as_ref()
            .and_then(|superclass| superclass.find_method(name)))
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

    pub fn get(self: Rc<Self>, name: &Token) -> Result<Value, RuntimeError> {
        if let Some(value) = self.fields.borrow().get(&name.lexeme) {
            Ok(value.clone())
        } else if let Some(method) = self.class.find_method(&name.lexeme) {
            Ok(Value::Function(Rc::new(method.clone().bind(self.clone()))))
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

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::String(val) => write!(f, "{:?}", val),
            Value::Bool(val) => write!(f, "{:?}", val),
            Value::Number(val) => write!(f, "{:?}", val),
            Value::Nil => write!(f, "nil"),
            Value::BuiltinCallable { .. } => write!(f, "[Builtin callable]"),
            Value::Function(function) => write!(f, "[Function {}]", function.name),
            Value::Class(class) => write!(f, "[Class {}]", class.name),
            Value::Instance(instance) => write!(f, "[Instance of Class {}]", instance.class.name),
        }
    }
}
