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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_returns_current_char() {
        let p = Parser::new("hello".to_string());
        assert_eq!(p.next().unwrap(), 'h');
    }

    #[test]
    fn next_on_empty_input_returns_error() {
        let p = Parser::new("".to_string());
        assert!(p.next().is_err());
    }

    #[test]
    fn consume_advances_position() {
        let mut p = Parser::new("ab".to_string());
        assert_eq!(p.consume().unwrap(), 'a');
        assert_eq!(p.consume().unwrap(), 'b');
        assert!(p.eof());
    }

    #[test]
    fn consume_while_collects_matching_chars() {
        let mut p = Parser::new("abc123".to_string());
        let result = p.consume_while(|c| c.is_alphabetic()).unwrap();
        assert_eq!(result, "abc");
        assert_eq!(p.pos, 3);
    }

    #[test]
    fn consume_while_returns_empty_when_no_match() {
        let mut p = Parser::new("123".to_string());
        let result = p.consume_while(|c| c.is_alphabetic()).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn starts_with_checks_prefix() {
        let p = Parser::new("<div>".to_string());
        assert!(p.starts_with("<"));
        assert!(p.starts_with("<div"));
        assert!(!p.starts_with("div"));
    }

    #[test]
    fn expect_consumes_matching_string() {
        let mut p = Parser::new("</div>".to_string());
        assert!(p.expect("</").is_ok());
        assert_eq!(p.pos, 2);
    }

    #[test]
    fn expect_fails_on_mismatch() {
        let mut p = Parser::new("hello".to_string());
        assert!(p.expect("world").is_err());
    }

    #[test]
    fn expect_char_consumes_matching_char() {
        let mut p = Parser::new("{".to_string());
        assert!(p.expect_char('{').is_ok());
        assert!(p.eof());
    }

    #[test]
    fn expect_char_fails_on_mismatch() {
        let mut p = Parser::new("a".to_string());
        assert!(p.expect_char('b').is_err());
    }

    #[test]
    fn consume_whitespace_skips_spaces_and_newlines() {
        let mut p = Parser::new("  \n\t hello".to_string());
        p.consume_whitespace().unwrap();
        assert_eq!(p.next().unwrap(), 'h');
    }

    #[test]
    fn eof_is_true_when_fully_consumed() {
        let mut p = Parser::new("a".to_string());
        p.consume().unwrap();
        assert!(p.eof());
    }

    #[test]
    fn eof_is_false_when_input_remains() {
        let p = Parser::new("a".to_string());
        assert!(!p.eof());
    }
}
