use crate::ast::*;
use crate::lex::{LexError, Lexer, Token};

pub struct Parser<'a> {
    lex: Lexer<'a>,
    cur: Token,
    peek: Option<Token>,
}

#[derive(Debug)]
pub enum ParseError {
    Lex(LexError),
    Expected { line: usize, msg: String },
    Unexpected { line: usize, tok: Token },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lex(e) => write!(f, "{e}"),
            Self::Expected { line, msg } => write!(f, "line {line}: expected {msg}"),
            Self::Unexpected { line, tok } => write!(f, "line {line}: unexpected {tok:?}"),
        }
    }
}

impl std::error::Error for ParseError {}

impl From<LexError> for ParseError {
    fn from(e: LexError) -> Self {
        Self::Lex(e)
    }
}

impl<'a> Parser<'a> {
    pub fn parse(src: &'a str) -> Result<Program, ParseError> {
        let mut p = Self {
            lex: Lexer::new(src),
            cur: Token::Eof,
            peek: None,
        };
        p.cur = p.advance()?;
        p.parse_program()
    }

    fn line(&self) -> usize {
        self.lex.line()
    }

    fn advance(&mut self) -> Result<Token, ParseError> {
        if let Some(t) = self.peek.take() {
            Ok(t)
        } else {
            self.lex.next_token().map_err(ParseError::from)
        }
    }

    fn lookahead(&mut self) -> Result<Token, ParseError> {
        if self.peek.is_none() {
            self.peek = Some(self.lex.next_token().map_err(ParseError::from)?);
        }
        Ok(self.peek.clone().unwrap())
    }

    fn bump(&mut self) -> Result<(), ParseError> {
        self.cur = self.advance()?;
        Ok(())
    }

    fn eat(&mut self, want: Token) -> Result<(), ParseError> {
        if std::mem::discriminant(&self.cur) == std::mem::discriminant(&want) {
            self.bump()?;
            Ok(())
        } else {
            Err(ParseError::Expected {
                line: self.line(),
                msg: format!("{want:?}"),
            })
        }
    }

    fn at(&self, t: &Token) -> bool {
        std::mem::discriminant(&self.cur) == std::mem::discriminant(t)
    }

    fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut globals = Vec::new();
        let mut funcs = Vec::new();
        let mut script = Vec::new();

        while !self.at(&Token::Eof) {
            if self.at(&Token::G) {
                globals.push(self.parse_global(false)?);
            } else if self.at(&Token::Gb) {
                globals.push(self.parse_global(true)?);
            } else if self.at(&Token::F) {
                funcs.push(self.parse_func()?);
            } else {
                script.push(self.parse_stmt()?);
            }
        }

        if !script.is_empty() && !funcs.iter().any(|f| f.name == "main") {
            funcs.push(Func {
                name: "main".into(),
                params: vec![],
                ret: Some("v".into()),
                body: Body::Block(Block { stmts: script }),
            });
        }

        Ok(Program { globals, funcs })
    }

    fn parse_global(&mut self, byte: bool) -> Result<Global, ParseError> {
        if byte {
            self.eat(Token::Gb)?;
        } else {
            self.eat(Token::G)?;
        }
        let Token::Ident(name) = self.cur.clone() else {
            return Err(ParseError::Expected {
                line: self.line(),
                msg: "identifier".into(),
            });
        };
        self.bump()?;
        self.eat(Token::Colon)?;
        let Token::Number(size) = self.cur.clone() else {
            return Err(ParseError::Expected {
                line: self.line(),
                msg: "number".into(),
            });
        };
        self.bump()?;
        Ok(Global {
            name,
            size: size.parse().unwrap_or(0),
            byte,
        })
    }

    fn parse_func(&mut self) -> Result<Func, ParseError> {
        self.eat(Token::F)?;
        let Token::Ident(name) = self.cur.clone() else {
            return Err(ParseError::Expected {
                line: self.line(),
                msg: "function name".into(),
            });
        };
        self.bump()?;
        self.eat(Token::LP)?;
        let mut params = Vec::new();
        if let Token::Ident(_) = &self.cur {
            params = self.parse_params()?;
        }
        self.eat(Token::RP)?;
        let mut ret = None;
        if self.at(&Token::Colon) {
            self.bump()?;
            if let Token::Ident(t) = self.cur.clone() {
                ret = Some(t);
                self.bump()?;
            }
        }
        let body = if self.at(&Token::Eq) {
            self.bump()?;
            Body::Expr(self.parse_ternary()?)
        } else {
            Body::Block(self.parse_block()?)
        };
        Ok(Func {
            name,
            params,
            ret,
            body,
        })
    }

    fn parse_params(&mut self) -> Result<Vec<Param>, ParseError> {
        let mut ps = vec![self.parse_param()?];
        while self.at(&Token::Comma) {
            self.bump()?;
            ps.push(self.parse_param()?);
        }
        Ok(ps)
    }

    fn parse_param(&mut self) -> Result<Param, ParseError> {
        let Token::Ident(name) = self.cur.clone() else {
            return Err(ParseError::Expected {
                line: self.line(),
                msg: "parameter name".into(),
            });
        };
        self.bump()?;
        let mut typ = None;
        if self.at(&Token::Colon) {
            self.bump()?;
            if let Token::Ident(t) = self.cur.clone() {
                typ = Some(t);
                self.bump()?;
            }
        }
        Ok(Param { name, typ })
    }

    fn parse_block(&mut self) -> Result<Block, ParseError> {
        self.eat(Token::LB)?;
        let mut stmts = Vec::new();
        while !self.at(&Token::RB) {
            stmts.push(self.parse_stmt()?);
        }
        self.eat(Token::RB)?;
        Ok(Block { stmts })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        if self.at(&Token::L) {
            self.bump()?;
            let Token::Ident(name) = self.cur.clone() else {
                return Err(ParseError::Expected {
                    line: self.line(),
                    msg: "identifier".into(),
                });
            };
            self.bump()?;
            self.eat(Token::Eq)?;
            let expr = if self.at(&Token::Q) {
                self.bump()?;
                let cond = self.parse_assign()?;
                let then_ = self.parse_branch()?;
                let else_ = self.parse_branch()?;
                Expr::Ternary {
                    cond: Box::new(cond),
                    then_: Box::new(then_),
                    else_: Box::new(else_),
                }
            } else {
                self.parse_assign()?
            };
            return Ok(Stmt::Let { name, expr });
        }
        if self.at(&Token::W) {
            self.bump()?;
            let cond = self.parse_ternary()?;
            let body = self.parse_block()?;
            return Ok(Stmt::While { cond, body });
        }
        if self.at(&Token::Ret) {
            self.bump()?;
            return Ok(Stmt::Return(self.parse_ternary()?));
        }
        Ok(Stmt::Expr(self.parse_ternary()?))
    }

    fn parse_branch(&mut self) -> Result<Expr, ParseError> {
        if self.at(&Token::LB) {
            return Ok(Expr::Block(self.parse_block()?));
        }
        if self.at(&Token::Q) {
            return self.parse_ternary();
        }
        self.parse_assign()
    }

    fn parse_ternary(&mut self) -> Result<Expr, ParseError> {
        if self.at(&Token::Q) {
            self.bump()?;
            let cond = self.parse_assign()?;
            let then_ = self.parse_branch()?;
            let else_ = self.parse_branch()?;
            return Ok(Expr::Ternary {
                cond: Box::new(cond),
                then_: Box::new(then_),
                else_: Box::new(else_),
            });
        }
        let e = self.parse_assign()?;
        if self.at(&Token::Q) {
            self.bump()?;
            let then_ = self.parse_assign()?;
            let else_ = self.parse_assign()?;
            return Ok(Expr::Ternary {
                cond: Box::new(e),
                then_: Box::new(then_),
                else_: Box::new(else_),
            });
        }
        Ok(e)
    }

    fn parse_assign(&mut self) -> Result<Expr, ParseError> {
        if let Token::Ident(name) = self.cur.clone() {
            if self.lookahead()? == Token::Eq {
                self.bump()?;
                self.bump()?;
                let expr = self.parse_assign()?;
                return Ok(Expr::Assign {
                    name,
                    expr: Box::new(expr),
                });
            }
        }
        self.parse_compare()
    }

    fn parse_compare(&mut self) -> Result<Expr, ParseError> {
        let mut e = self.parse_add()?;
        loop {
            let op = match &self.cur {
                Token::Lt => "<",
                Token::Gt => ">",
                Token::Le => "<=",
                Token::Ge => ">=",
                Token::EqEq => "==",
                Token::Ne => "!=",
                _ => break,
            };
            self.bump()?;
            e = Expr::BinOp {
                op: op.into(),
                left: Box::new(e),
                right: Box::new(self.parse_add()?),
            };
        }
        Ok(e)
    }

    fn parse_add(&mut self) -> Result<Expr, ParseError> {
        let mut e = self.parse_mul()?;
        loop {
            let op = match &self.cur {
                Token::Plus => "+",
                Token::Minus => "-",
                _ => break,
            };
            self.bump()?;
            e = Expr::BinOp {
                op: op.into(),
                left: Box::new(e),
                right: Box::new(self.parse_mul()?),
            };
        }
        Ok(e)
    }

    fn parse_mul(&mut self) -> Result<Expr, ParseError> {
        let mut e = self.parse_unary()?;
        loop {
            let op = match &self.cur {
                Token::Star => "*",
                Token::Slash => "/",
                Token::Percent => "%",
                _ => break,
            };
            self.bump()?;
            e = Expr::BinOp {
                op: op.into(),
                left: Box::new(e),
                right: Box::new(self.parse_unary()?),
            };
        }
        Ok(e)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if self.at(&Token::Minus) {
            self.bump()?;
            return Ok(Expr::Unary {
                op: "-".into(),
                expr: Box::new(self.parse_unary()?),
            });
        }
        if self.at(&Token::Bang) {
            self.bump()?;
            return Ok(Expr::Unary {
                op: "!".into(),
                expr: Box::new(self.parse_unary()?),
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut e = self.parse_primary()?;
        loop {
            if self.at(&Token::LBr) {
                self.bump()?;
                let idx = self.parse_assign()?;
                self.eat(Token::RBr)?;
                let name = match e {
                    Expr::Var(n) => n,
                    _ => {
                        return Err(ParseError::Unexpected {
                            line: self.line(),
                            tok: Token::LBr,
                        })
                    }
                };
                if self.at(&Token::Eq) {
                    self.bump()?;
                    let val = self.parse_assign()?;
                    e = Expr::IndexAssign {
                        name,
                        idx: Box::new(idx),
                        expr: Box::new(val),
                    };
                } else {
                    e = Expr::Index {
                        name,
                        idx: Box::new(idx),
                    };
                }
            } else if self.at(&Token::LP) {
                self.bump()?;
                let mut args = Vec::new();
                if !self.at(&Token::RP) {
                    args.push(self.parse_assign()?);
                    while self.at(&Token::Comma) {
                        self.bump()?;
                        args.push(self.parse_assign()?);
                    }
                }
                self.eat(Token::RP)?;
                let name = match e {
                    Expr::Var(n) => n,
                    _ => {
                        return Err(ParseError::Unexpected {
                            line: self.line(),
                            tok: Token::LP,
                        })
                    }
                };
                e = Expr::Call { name, args };
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        if self.at(&Token::At) {
            return self.parse_at_call();
        }
        if let Token::Number(v) = self.cur.clone() {
            self.bump()?;
            return Ok(Expr::Num(v));
        }
        if let Token::Str(v) = self.cur.clone() {
            self.bump()?;
            return Ok(Expr::Str(v));
        }
        if let Token::Ident(v) = self.cur.clone() {
            self.bump()?;
            return Ok(Expr::Var(v));
        }
        if self.at(&Token::LP) {
            self.bump()?;
            let e = self.parse_ternary()?;
            self.eat(Token::RP)?;
            return Ok(e);
        }
        Err(ParseError::Unexpected {
            line: self.line(),
            tok: self.cur.clone(),
        })
    }

    fn parse_at_call(&mut self) -> Result<Expr, ParseError> {
        self.eat(Token::At)?;
        if self.at(&Token::LP) {
            self.bump()?;
            let Token::Ident(name) = self.cur.clone() else {
                return Err(ParseError::Expected {
                    line: self.line(),
                    msg: "function name".into(),
                });
            };
            self.bump()?;
            let mut args = Vec::new();
            if self.at(&Token::Comma) {
                self.bump()?;
                if !self.at(&Token::RP) {
                    args.push(self.parse_assign()?);
                    while self.at(&Token::Comma) {
                        self.bump()?;
                        args.push(self.parse_assign()?);
                    }
                }
            }
            self.eat(Token::RP)?;
            return Ok(Expr::Call { name, args });
        }
        let Token::Ident(name) = self.cur.clone() else {
            return Err(ParseError::Expected {
                line: self.line(),
                msg: "function name".into(),
            });
        };
        self.bump()?;
        let mut args = Vec::new();
        if self.at(&Token::Comma) {
            self.bump()?;
            args.push(self.parse_assign()?);
            while self.at(&Token::Comma) {
                self.bump()?;
                args.push(self.parse_assign()?);
            }
        }
        Ok(Expr::Call { name, args })
    }
}
