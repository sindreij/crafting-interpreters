#[derive(Copy, Clone, Debug)]
pub enum Value {
    Number(f64),
    Nil,
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(value) => write!(f, "{}", value),
            Value::Nil => write!(f, "nil"),
        }
    }
}
