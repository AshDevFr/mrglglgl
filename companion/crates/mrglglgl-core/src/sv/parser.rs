use super::lexer::{Lexer, Token};
use super::value::{Key, Table, Value};
use super::{ParseError, ParseErrorKind, SavedVariables};

/// Tables nested deeper than this are rejected. Real journals nest about eight levels; the limit
/// keeps a malformed or hostile file from exhausting the stack.
pub const MAX_DEPTH: usize = 128;

pub(crate) struct Parser<'a> {
    lexer: Lexer<'a>,
    current: Token,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(src: &'a str) -> Result<Self, ParseError> {
        let mut lexer = Lexer::new(src);
        let current = lexer.next_token()?;
        Ok(Self { lexer, current })
    }

    fn advance(&mut self) -> Result<Token, ParseError> {
        let next = self.lexer.next_token()?;
        Ok(std::mem::replace(&mut self.current, next))
    }

    fn expect(&mut self, token: Token) -> Result<(), ParseError> {
        if self.current == token {
            self.advance()?;
            Ok(())
        } else {
            Err(self.unexpected())
        }
    }

    fn unexpected(&self) -> ParseError {
        let kind = match &self.current {
            Token::Eof => ParseErrorKind::UnexpectedEnd,
            other => ParseErrorKind::UnexpectedToken(format!("{other:?}")),
        };
        self.lexer.error(kind)
    }

    /// A file is a sequence of `Name = value` assignments.
    pub(crate) fn file(&mut self) -> Result<SavedVariables, ParseError> {
        let mut file = SavedVariables::default();
        while self.current != Token::Eof {
            let Token::Name(name) = self.advance()? else {
                return Err(self.unexpected());
            };
            self.expect(Token::Equals)?;
            let value = self.value(0)?;
            if self.current == Token::Semicolon {
                self.advance()?;
            }
            file.insert(name, value);
        }
        Ok(file)
    }

    fn value(&mut self, depth: usize) -> Result<Value, ParseError> {
        match self.advance()? {
            Token::Str(s) => Ok(Value::String(s)),
            Token::Integer(i) => Ok(Value::Integer(i)),
            Token::Float(f) => Ok(Value::Float(f)),
            Token::Minus => match self.advance()? {
                Token::Integer(i) => Ok(Value::Integer(-i)),
                Token::Float(f) => Ok(Value::Float(-f)),
                _ => Err(self.lexer.error(ParseErrorKind::InvalidNumber)),
            },
            Token::Name(name) => match name.as_str() {
                "true" => Ok(Value::Bool(true)),
                "false" => Ok(Value::Bool(false)),
                "nil" => Ok(Value::Nil),
                _ => Err(self
                    .lexer
                    .error(ParseErrorKind::UnexpectedToken(format!("Name({name:?})")))),
            },
            Token::LBrace => self.table(depth + 1),
            _ => Err(self.unexpected()),
        }
    }

    fn table(&mut self, depth: usize) -> Result<Value, ParseError> {
        if depth > MAX_DEPTH {
            return Err(self.lexer.error(ParseErrorKind::TooDeep));
        }
        let mut table = Table::new();
        let mut next_index = 1;
        while self.current != Token::RBrace {
            if self.current == Token::LBracket {
                self.advance()?;
                let key = self.key(depth)?;
                self.expect(Token::RBracket)?;
                self.expect(Token::Equals)?;
                let value = self.value(depth)?;
                table.insert(key, value);
            } else if matches!(self.current, Token::Name(ref n) if !matches!(n.as_str(), "true" | "false" | "nil"))
            {
                // `name = value`, the shorthand for `["name"] = value`.
                let Token::Name(name) = self.advance()? else {
                    unreachable!("checked above")
                };
                self.expect(Token::Equals)?;
                let value = self.value(depth)?;
                table.insert(Key::String(name), value);
            } else {
                let value = self.value(depth)?;
                table.insert(Key::Integer(next_index), value);
                next_index += 1;
            }
            match self.current {
                Token::Comma | Token::Semicolon => {
                    self.advance()?;
                }
                Token::RBrace => {}
                _ => return Err(self.unexpected()),
            }
        }
        self.advance()?;
        Ok(Value::Table(table))
    }

    fn key(&mut self, depth: usize) -> Result<Key, ParseError> {
        match self.value(depth)? {
            Value::String(s) => Ok(Key::String(s)),
            Value::Integer(i) => Ok(Key::Integer(i)),
            // Lua normalises a float key with no fractional part to the integer key.
            Value::Float(f) if f.fract() == 0.0 && f.is_finite() => Ok(Key::Integer(f as i64)),
            Value::Float(f) => Ok(Key::Float(f.to_bits())),
            Value::Bool(b) => Ok(Key::Bool(b)),
            Value::Nil | Value::Table(_) => Err(self.lexer.error(ParseErrorKind::InvalidKey)),
        }
    }
}
