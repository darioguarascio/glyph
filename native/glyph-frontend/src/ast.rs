#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub globals: Vec<Global>,
    pub funcs: Vec<Func>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Global {
    pub name: String,
    pub size: u32,
    pub byte: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Func {
    pub name: String,
    pub params: Vec<Param>,
    pub ret: Option<String>,
    pub body: Body,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub typ: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Body {
    Expr(Expr),
    Block(Block),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let { name: String, expr: Expr },
    While { cond: Expr, body: Block },
    Return(Expr),
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(String),
    Str(String),
    Var(String),
    BinOp { op: String, left: Box<Expr>, right: Box<Expr> },
    Unary { op: String, expr: Box<Expr> },
    Ternary { cond: Box<Expr>, then_: Box<Expr>, else_: Box<Expr> },
    Call { name: String, args: Vec<Expr> },
    Assign { name: String, expr: Box<Expr> },
    Index { name: String, idx: Box<Expr> },
    IndexAssign { name: String, idx: Box<Expr>, expr: Box<Expr> },
    Block(Block),
}
