use std::collections::HashMap;

use crate::{
    runtime_error::{Result, RuntimeError},
    token::Token,
    value::Value,
};

pub struct Environment {
    values: HashMap<String, Value>,
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            values: HashMap::new(),
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
            Err(RuntimeError::new(
                name.clone(),
                format!("Undefined variable '{}'", name.lexeme),
            ))
        }
    }

    pub fn get(&mut self, name: &Token) -> Result<Value> {
        if let Some(value) = self.values.get(&name.lexeme) {
            Ok(value.clone())
        } else {
            Err(RuntimeError::new(
                name.clone(),
                format!("Undefined variable '{}'", name.lexeme),
            ))
        }
    }
}
