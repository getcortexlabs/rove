use super::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Word,
    Quoted,
    String,
    Punctuation(char),
    Newline,
}

#[derive(Debug)]
pub(super) struct Token {
    pub text: String,
    pub kind: Kind,
    pub line: usize,
    pub column: usize,
}

impl Token {
    pub fn is(&self, word: &str) -> bool {
        self.kind == Kind::Word && self.text.eq_ignore_ascii_case(word)
    }

    pub fn punct(&self, symbol: char) -> bool {
        self.kind == Kind::Punctuation(symbol)
    }

    pub fn error(&self, message: impl Into<String>) -> Error {
        Error {
            line: self.line,
            column: self.column,
            message: message.into(),
        }
    }

    pub fn label(&self) -> Result<String, Error> {
        if !matches!(self.kind, Kind::Word | Kind::Quoted) || self.text.trim().is_empty() {
            return Err(self.error("expected a name; quote names containing punctuation"));
        }
        Ok(self.text.replace('_', " "))
    }
}

pub(super) fn lex(input: &str) -> Result<Vec<Token>, Error> {
    let mut lexer = Lexer {
        chars: input.chars().peekable(),
        line: 1,
        column: 1,
    };
    let mut tokens = Vec::new();
    if lexer.chars.peek() == Some(&'\u{feff}') {
        lexer.next();
    }
    while let Some(&c) = lexer.chars.peek() {
        let mut token = Token {
            text: String::new(),
            kind: Kind::Word,
            line: lexer.line,
            column: lexer.column,
        };
        match c {
            '[' => lexer.comment()?,
            '\n' | '\r' => {
                lexer.next();
                token.kind = Kind::Newline;
                tokens.push(token);
            }
            c if c.is_whitespace() => {
                lexer.next();
            }
            '\'' | '"' => {
                token.kind = if c == '\'' {
                    Kind::Quoted
                } else {
                    Kind::String
                };
                lexer.next();
                loop {
                    match lexer.next() {
                        None => return Err(token.error("unterminated quoted text")),
                        Some(ch) if ch == c => {
                            if c == '\'' && lexer.chars.peek() == Some(&c) {
                                lexer.next();
                                token.text.push(c);
                            } else {
                                break;
                            }
                        }
                        Some(ch) => token.text.push(ch),
                    }
                }
                tokens.push(token);
            }
            c if punctuation(c) => {
                lexer.next();
                token.kind = Kind::Punctuation(c);
                token.text.push(c);
                tokens.push(token);
            }
            _ => {
                while let Some(&ch) = lexer.chars.peek() {
                    if ch == '[' {
                        // NEXUS comments can appear inside a word without splitting it.
                        lexer.comment()?;
                    } else if ch.is_whitespace() || punctuation(ch) || ch == '\'' || ch == '"' {
                        break;
                    } else {
                        token.text.push(ch);
                        lexer.next();
                    }
                }
                tokens.push(token);
            }
        }
    }
    Ok(tokens)
}

fn punctuation(c: char) -> bool {
    "(){}]/\\,;:=*`<>^+-".contains(c)
}

struct Lexer<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    line: usize,
    column: usize,
}

impl Lexer<'_> {
    fn next(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        if c == '\r' {
            if self.chars.peek() == Some(&'\n') {
                self.chars.next();
            }
            self.line += 1;
            self.column = 1;
            Some('\n')
        } else {
            if c == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            Some(c)
        }
    }

    fn comment(&mut self) -> Result<(), Error> {
        let (line, column) = (self.line, self.column);
        self.next();
        let mut depth = 1;
        while let Some(c) = self.next() {
            match c {
                '[' => depth += 1,
                ']' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(());
                    }
                }
                _ => {}
            }
        }
        Err(Error {
            line,
            column,
            message: "unterminated comment".into(),
        })
    }
}
