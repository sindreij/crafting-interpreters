use crate::token::Token;

#[derive(Clone)]
pub enum Expr {
    Assign {
        name: Token,
        value: Box<Expr>,
        expr_id: usize,
    },
    Binary {
        left: Box<Expr>,
        operator: Token,
        right: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        paren: Token,
        arguments: Vec<Expr>,
    },
    Get {
        object: Box<Expr>,
        name: Token,
    },
    Grouping(Box<Expr>),
    Literal(Literal),
    Logical {
        left: Box<Expr>,
        operator: Token,
        right: Box<Expr>,
    },
    Set {
        object: Box<Expr>,
        name: Token,
        value: Box<Expr>,
    },
    Unary {
        operator: Token,
        right: Box<Expr>,
    },
    Variable {
        name: Token,
        expr_id: usize,
    },
}

#[derive(Clone)]
pub struct StmtFunction {
    pub name: Token,
    pub params: Vec<Token>,
    pub body: Vec<Stmt>,
}

#[derive(Clone)]
pub enum Stmt {
    Block(Vec<Stmt>),
    Class {
        name: Token,
        methods: Vec<StmtFunction>,
    },
    Expression(Expr),
    Function(StmtFunction),
    If {
        condition: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },
    Return {
        keyword: Token,
        value: Expr,
    },
    Print(Expr),
    Var {
        name: Token,
        initializer: Option<Expr>,
    },
    While {
        condition: Expr,
        body: Box<Stmt>,
    },
}

#[derive(Clone)]
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

// impl std::fmt::Display for Expr {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Expr::Binary {
//                 left,
//                 operator,
//                 right,
//             } => write_parenthesize(f, &operator.lexeme, &[left, right]),
//             Expr::Grouping(expression) => write_parenthesize(f, "group", &[expression]),
//             Expr::Literal(literal) => write!(f, "{}", literal),
//             Expr::Unary { operator, right } => write_parenthesize(f, &operator.lexeme, &[right]),
//         }
//     }
// }

// fn write_parenthesize(
//     f: &mut std::fmt::Formatter<'_>,
//     name: &str,
//     exprs: &[&Expr],
// ) -> std::fmt::Result {
//     write!(f, "({}", name)?;
//     for expr in exprs {
//         write!(f, " {}", expr)?;
//     }
//     write!(f, ")")?;

//     Ok(())
// }
