#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    F,
    G,
    Gb,
    L,
    W,
    Ret,
    Q,
    At,
    LP,
    RP,
    LB,
    RB,
    LBr,
    RBr,
    Comma,
    Colon,
    Eq,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Lt,
    Gt,
    Le,
    Ge,
    EqEq,
    Ne,
    Bang,
    Ident(String),
    Number(String),
    Str(String),
    Eof,
}

pub struct Lexer<'a> {
    src: &'a str,
    chars: std::vec::IntoIter<(usize, char)>,
    line: usize,
    peek: Option<(usize, char)>,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        let chars = src.char_indices().collect::<Vec<_>>().into_iter();
        let mut lex = Self {
            src,
            chars,
            line: 1,
            peek: None,
        };
        lex.skip_ws();
        lex
    }

    fn bump(&mut self) -> Option<char> {
        let (_, c) = if let Some(p) = self.peek.take() {
            p
        } else {
            self.chars.next()?
        };
        if c == '\n' {
            self.line += 1;
        }
        Some(c)
    }

    fn peek_char(&mut self) -> Option<char> {
        if self.peek.is_none() {
            self.peek = self.chars.next();
        }
        self.peek.map(|(_, c)| c)
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek_char(), Some(' ' | '\t' | '\r' | '\n')) {
            self.bump();
        }
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_ws();
        let c = match self.bump() {
            Some(c) => c,
            None => return Ok(Token::Eof),
        };

        if c.is_ascii_alphabetic() || c == '_' {
            let start = self.line;
            let mut s = String::new();
            s.push(c);
            while matches!(self.peek_char(), Some(ch) if ch.is_ascii_alphanumeric() || ch == '_') {
                s.push(self.bump().unwrap());
            }
            return Ok(match s.as_str() {
                "f" => Token::F,
                "g" => Token::G,
                "gb" => Token::Gb,
                "l" => Token::L,
                "w" => Token::W,
                _ => Token::Ident(s),
            });
        }

        if c.is_ascii_digit() {
            let mut s = String::new();
            s.push(c);
            while matches!(self.peek_char(), Some(ch) if ch.is_ascii_digit()) {
                s.push(self.bump().unwrap());
            }
            if self.peek_char() == Some('.') {
                s.push(self.bump().unwrap());
                while matches!(self.peek_char(), Some(ch) if ch.is_ascii_digit()) {
                    s.push(self.bump().unwrap());
                }
            }
            return Ok(Token::Number(s));
        }

        if c == '"' {
            let start_line = self.line;
            let mut s = String::new();
            loop {
                match self.bump() {
                    Some('"') => break,
                    Some('\\') => {
                        let esc = self.bump().ok_or(LexError::UnterminatedString { line: start_line })?;
                        s.push(match esc {
                            'n' => '\n',
                            'r' => '\r',
                            't' => '\t',
                            '\\' => '\\',
                            '"' => '"',
                            other => other,
                        });
                    }
                    Some(ch) => s.push(ch),
                    None => return Err(LexError::UnterminatedString { line: self.line }),
                }
            }
            return Ok(Token::Str(s));
        }

        if c == '<' && self.peek_char() == Some('=') {
            self.bump();
            return Ok(Token::Le);
        }
        if c == '>' && self.peek_char() == Some('=') {
            self.bump();
            return Ok(Token::Ge);
        }
        if c == '=' && self.peek_char() == Some('=') {
            self.bump();
            return Ok(Token::EqEq);
        }
        if c == '!' && self.peek_char() == Some('=') {
            self.bump();
            return Ok(Token::Ne);
        }

        Ok(match c {
            '(' => Token::LP,
            ')' => Token::RP,
            '{' => Token::LB,
            '}' => Token::RB,
            '[' => Token::LBr,
            ']' => Token::RBr,
            ',' => Token::Comma,
            ':' => Token::Colon,
            '=' => Token::Eq,
            '+' => Token::Plus,
            '-' => Token::Minus,
            '*' => Token::Star,
            '/' => Token::Slash,
            '%' => Token::Percent,
            '<' => Token::Lt,
            '>' => Token::Gt,
            '?' => Token::Q,
            '@' => Token::At,
            '!' => Token::Ret,
            _ => return Err(LexError::UnexpectedChar { line: self.line, ch: c }),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LexError {
    UnexpectedChar { line: usize, ch: char },
    UnterminatedString { line: usize },
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedChar { line, ch } => write!(f, "line {line}: unexpected '{ch}'"),
            Self::UnterminatedString { line } => write!(f, "line {line}: unterminated string"),
        }
    }
}

impl std::error::Error for LexError {}
