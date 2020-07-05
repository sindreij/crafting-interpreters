use crate::token::Token;

pub enum Expr {
    Binary {
        left: Box<Expr>,
        operator: Token,
        right: Box<Expr>,
    },
    Grouping(Box<Expr>),
    Literal(Literal),
    Unary {
        operator: Token,
        right: Box<Expr>,
    },
}

pub enum Stmt {
    Expression(Expr),
    Print(Expr),
}

pub enum Literal {
    Number(f64),
    String(String),
    Bool(bool),
    Nil,
}

impl std::fmt::Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Literal::Number(number) => write!(f, "{}", number),
            Literal::String(string) => write!(f, "\"{}\"", string),
            Literal::Bool(bool) => write!(f, "{}", bool),
            Literal::Nil => write!(f, "nil"),
        }
    }
}

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Binary {
                left,
                operator,
                right,
            } => write_parenthesize(f, &operator.lexeme, &[left, right]),
            Expr::Grouping(expression) => write_parenthesize(f, "group", &[expression]),
            Expr::Literal(literal) => write!(f, "{}", literal),
            Expr::Unary { operator, right } => write_parenthesize(f, &operator.lexeme, &[right]),
        }
    }
}

fn write_parenthesize(
    f: &mut std::fmt::Formatter<'_>,
    name: &str,
    exprs: &[&Expr],
) -> std::fmt::Result {
    write!(f, "({}", name)?;
    for expr in exprs {
        write!(f, " {}", expr)?;
    }
    write!(f, ")")?;

    Ok(())
}
