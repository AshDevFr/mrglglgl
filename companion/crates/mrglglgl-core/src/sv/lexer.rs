use super::ParseError;
use super::ParseErrorKind;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token {
    Name(String),
    Str(String),
    Integer(i64),
    Float(f64),
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Equals,
    Comma,
    Semicolon,
    Minus,
    Eof,
}

/// Splits SavedVariables text into tokens. Works on bytes: Lua strings are byte strings, and a
/// `\ddd` escape may produce bytes that are not UTF-8 on their own.
pub(crate) struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
    line: usize,
    line_start: usize,
}

impl<'a> Lexer<'a> {
    pub(crate) fn new(src: &'a str) -> Self {
        Self {
            src: src.as_bytes(),
            pos: 0,
            line: 1,
            line_start: 0,
        }
    }

    pub(crate) fn error(&self, kind: ParseErrorKind) -> ParseError {
        ParseError {
            line: self.line,
            column: self.pos - self.line_start + 1,
            kind,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.src.get(self.pos + offset).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.pos += 1;
        if byte == b'\n' {
            self.line += 1;
            self.line_start = self.pos;
        }
        Some(byte)
    }

    fn skip_trivia(&mut self) -> Result<(), ParseError> {
        loop {
            match self.peek() {
                Some(b' ' | b'\t' | b'\r' | b'\n') => {
                    self.bump();
                }
                // A UTF-8 byte order mark some editors add at the start of the file.
                Some(0xEF) if self.pos == 0 && self.src.starts_with(&[0xEF, 0xBB, 0xBF]) => {
                    self.pos = 3;
                    self.line_start = 3;
                }
                Some(b'-') if self.peek_at(1) == Some(b'-') => {
                    self.pos += 2;
                    if self.peek() == Some(b'[') && self.long_bracket_level().is_some() {
                        self.skip_long_comment()?;
                    } else {
                        while !matches!(self.peek(), None | Some(b'\n')) {
                            self.bump();
                        }
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    /// For `[[` or `[==[` at the current position, the number of `=` signs.
    fn long_bracket_level(&self) -> Option<usize> {
        let mut level = 0;
        while self.peek_at(1 + level) == Some(b'=') {
            level += 1;
        }
        (self.peek_at(1 + level) == Some(b'[')).then_some(level)
    }

    fn skip_long_comment(&mut self) -> Result<(), ParseError> {
        let level = self.long_bracket_level().unwrap_or(0);
        for _ in 0..level + 2 {
            self.bump();
        }
        loop {
            match self.bump() {
                None => return Err(self.error(ParseErrorKind::UnterminatedComment)),
                Some(b']') => {
                    let mut closing = 0;
                    while self.peek() == Some(b'=') {
                        closing += 1;
                        self.bump();
                    }
                    if closing == level && self.peek() == Some(b']') {
                        self.bump();
                        return Ok(());
                    }
                }
                Some(_) => {}
            }
        }
    }

    pub(crate) fn next_token(&mut self) -> Result<Token, ParseError> {
        self.skip_trivia()?;
        let Some(byte) = self.peek() else {
            return Ok(Token::Eof);
        };
        let token = match byte {
            b'{' => Token::LBrace,
            b'}' => Token::RBrace,
            b'[' => Token::LBracket,
            b']' => Token::RBracket,
            b'=' => Token::Equals,
            b',' => Token::Comma,
            b';' => Token::Semicolon,
            b'-' => Token::Minus,
            b'"' | b'\'' => return self.string(byte),
            b'0'..=b'9' | b'.' => return self.number(),
            b if b == b'_' || b.is_ascii_alphabetic() => return Ok(self.name()),
            other => return Err(self.error(ParseErrorKind::UnexpectedCharacter(other as char))),
        };
        self.bump();
        Ok(token)
    }

    fn name(&mut self) -> Token {
        let start = self.pos;
        while matches!(self.peek(), Some(b) if b == b'_' || b.is_ascii_alphanumeric()) {
            self.bump();
        }
        Token::Name(String::from_utf8_lossy(&self.src[start..self.pos]).into_owned())
    }

    fn number(&mut self) -> Result<Token, ParseError> {
        let start = self.pos;
        if self.peek() == Some(b'0') && matches!(self.peek_at(1), Some(b'x' | b'X')) {
            self.pos += 2;
            let digits = self.pos;
            while matches!(self.peek(), Some(b) if b.is_ascii_hexdigit()) {
                self.bump();
            }
            let text = std::str::from_utf8(&self.src[digits..self.pos]).unwrap_or("");
            return i64::from_str_radix(text, 16)
                .map(Token::Integer)
                .map_err(|_| self.error(ParseErrorKind::InvalidNumber));
        }
        let mut is_float = false;
        while let Some(b) = self.peek() {
            match b {
                b'0'..=b'9' => {}
                b'.' => is_float = true,
                b'e' | b'E' => {
                    is_float = true;
                    if matches!(self.peek_at(1), Some(b'+' | b'-')) {
                        self.bump();
                    }
                }
                _ => break,
            }
            self.bump();
        }
        let text = std::str::from_utf8(&self.src[start..self.pos]).unwrap_or("");
        if !is_float && let Ok(i) = text.parse::<i64>() {
            return Ok(Token::Integer(i));
        }
        text.parse::<f64>()
            .map(Token::Float)
            .map_err(|_| self.error(ParseErrorKind::InvalidNumber))
    }

    fn string(&mut self, quote: u8) -> Result<Token, ParseError> {
        // An unclosed string is reported where it opens, which is where the fix belongs.
        let opened = self.error(ParseErrorKind::UnterminatedString);
        self.bump();
        let mut bytes = Vec::new();
        loop {
            match self.bump() {
                None | Some(b'\n') => return Err(opened),
                Some(b) if b == quote => break,
                Some(b'\\') => self.escape(&mut bytes)?,
                Some(b) => bytes.push(b),
            }
        }
        // Lua strings are bytes; invalid UTF-8 (rare, from a \ddd escape) is replaced rather
        // than rejected, so one odd string never makes a whole journal unreadable.
        Ok(Token::Str(String::from_utf8_lossy(&bytes).into_owned()))
    }

    fn escape(&mut self, out: &mut Vec<u8>) -> Result<(), ParseError> {
        let Some(b) = self.bump() else {
            return Err(self.error(ParseErrorKind::UnterminatedString));
        };
        let byte = match b {
            b'n' => b'\n',
            b't' => b'\t',
            b'r' => b'\r',
            b'a' => 0x07,
            b'b' => 0x08,
            b'f' => 0x0C,
            b'v' => 0x0B,
            b'\\' | b'"' | b'\'' => b,
            // A backslash before a real line break stands for the line break (Lua's %q writes
            // newlines this way).
            b'\n' => b'\n',
            b'\r' => {
                if self.peek() == Some(b'\n') {
                    self.bump();
                }
                b'\n'
            }
            b'0'..=b'9' => {
                let mut value = u32::from(b - b'0');
                for _ in 0..2 {
                    match self.peek() {
                        Some(d @ b'0'..=b'9') => {
                            self.bump();
                            value = value * 10 + u32::from(d - b'0');
                        }
                        _ => break,
                    }
                }
                u8::try_from(value).map_err(|_| self.error(ParseErrorKind::InvalidEscape))?
            }
            _ => return Err(self.error(ParseErrorKind::InvalidEscape)),
        };
        out.push(byte);
        Ok(())
    }
}
