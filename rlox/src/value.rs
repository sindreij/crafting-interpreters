use crate::object::{ObjHeap, ObjPointer};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Value {
    Nil,
    Number(f64),
    Bool(bool),
    Obj(ObjPointer),
}

impl Value {
    pub fn is_falsey(&self) -> bool {
        match self {
            Value::Nil => true,
            Value::Bool(inner) => !inner,
            _ => false,
        }
    }

    pub fn to_string(&self, heap: &ObjHeap) -> String {
        match self {
            Value::Number(value) => format!("{}", value),
            Value::Bool(value) => format!("{}", value),
            Value::Nil => format!("nil"),
            Value::Obj(pointer) => format!("{} ({})", pointer.borrow(heap).to_string(), pointer.0),
        }
    }

    pub fn eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Nil, Value::Nil) => true,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Obj(a), Value::Obj(b)) => a == b,
            _ => false,
        }
    }
}
