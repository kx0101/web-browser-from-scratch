#[derive(Debug)]
pub struct ParserError {
    pub message: String,
    pub pos: usize,
}

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parse error at byte {}: {}", self.pos, self.message)
    }
}

pub type ParserResult<T> = Result<T, ParserError>;

pub struct Parser {
    pub pos: usize,
    pub input: String,
}

impl Parser {
    pub fn new(input: String) -> Self {
        Parser { pos: 0, input }
    }

    pub fn next(&self) -> ParserResult<char> {
        self.input[self.pos..]
            .chars()
            .next()
            .ok_or_else(|| self.err("Unexpected end of input"))
    }

    pub fn starts_with(&self, s: &str) -> bool {
        self.input[self.pos..].starts_with(s)
    }

    pub fn expect(&mut self, s: &str) -> ParserResult<()> {
        if !self.starts_with(s) {
            return Err(self.err(format!("Expected {:?}", s)));
        }

        self.pos += s.len();
        Ok(())
    }

    pub fn expect_char(&mut self, expected: char) -> ParserResult<()> {
        let c = self.consume()?;
        if c != expected {
            return Err(self.err(format!("Expected {:?}, got {:?}", expected, c)));
        }

        Ok(())
    }

    pub fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    pub fn consume(&mut self) -> ParserResult<char> {
        let c = self.next()?;
        self.pos += c.len_utf8();

        Ok(c)
    }

    pub fn consume_while(&mut self, predicate: impl Fn(char) -> bool) -> ParserResult<String> {
        let mut result = String::new();
        while !self.eof() && predicate(self.next()?) {
            result.push(self.consume()?);
        }

        Ok(result)
    }

    pub fn consume_whitespace(&mut self) -> ParserResult<()> {
        self.consume_while(char::is_whitespace)?;

        Ok(())
    }

    pub fn err(&self, message: impl Into<String>) -> ParserError {
        ParserError {
            message: message.into(),
            pos: self.pos,
        }
    }
}
