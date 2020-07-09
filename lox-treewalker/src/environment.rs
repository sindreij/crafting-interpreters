use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    runtime_error::{Result, RuntimeError},
    token::Token,
    value::Value,
};

pub struct Environment {
    values: HashMap<String, Value>,
    enclosing: Option<Rc<RefCell<Environment>>>,
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            values: HashMap::new(),
            enclosing: None,
        }
    }

    pub fn new_with_enclosing(enclosing: &Rc<RefCell<Environment>>) -> Self {
        Self {
            values: HashMap::new(),
            enclosing: Some(Rc::clone(enclosing)),
        }
    }

    pub fn define(&mut self, name: &str, value: Value) {
        self.values.insert(name.to_owned(), value);
    }

    pub fn assign(&mut self, name: &Token, value: Value) -> Result<()> {
        if self.values.contains_key(&name.lexeme) {
            self.values.insert(name.lexeme.clone(), value);
            Ok(())
        } else {
            if let Some(enclosing) = self.enclosing.as_ref() {
                enclosing.borrow_mut().assign(name, value)
            } else {
                Err(RuntimeError::new(
                    name.clone(),
                    format!("Undefined variable '{}'", name.lexeme),
                ))
            }
        }
    }

    pub fn get(&self, name: &Token) -> Result<Value> {
        if let Some(value) = self.values.get(&name.lexeme) {
            Ok(value.clone())
        } else {
            if let Some(enclosing) = self.enclosing.as_ref() {
                enclosing.borrow().get(name)
            } else {
                Err(RuntimeError::new(
                    name.clone(),
                    format!("Undefined variable '{}'", name.lexeme),
                ))
            }
        }
    }
}

pub fn get_at(environment: Rc<RefCell<Environment>>, distance: usize, name: &str) -> Value {
    anchestor(environment, distance).borrow().values[name].clone()
}

pub fn assign_at(
    environment: Rc<RefCell<Environment>>,
    distance: usize,
    name: &Token,
    value: Value,
) {
    anchestor(environment, distance)
        .borrow_mut()
        .values
        .insert(name.lexeme.clone(), value);
}

fn anchestor(environment: Rc<RefCell<Environment>>, distance: usize) -> Rc<RefCell<Environment>> {
    let mut environment = environment;
    for _ in 0..distance {
        let next_env = environment
            .borrow()
            .enclosing
            .as_ref()
            .expect("Could not find environment at distance")
            .clone();

        environment = next_env;
    }

    environment
}
