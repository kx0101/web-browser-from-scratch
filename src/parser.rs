use std::collections::HashMap;

use crate::dom;

#[derive(Debug)]
struct ParserError {
    message: String,
    pos: usize,
}

type ParserResult<T> = Result<T, ParserError>;

struct Parser {
    pos: usize,
    input: String,
}

impl Parser {
    fn parse(source: String) -> ParserResult<dom::Node> {
        let mut parser = Parser {
            pos: 0,
            input: source,
        };

        let mut nodes = parser.parse_nodes()?;
        if nodes.len() == 1 {
            return Ok(nodes.remove(0));
        }

        Ok(dom::elem("html".to_string(), HashMap::new(), nodes))
    }

    fn next(&self) -> ParserResult<char> {
        match self.input[self.pos..].chars().next() {
            Some(c) => Ok(c),
            None => Err(ParserError {
                message: "Unexpected end of input".to_string(),
                pos: self.pos,
            }),
        }
    }

    fn starts_with(&self, s: &str) -> bool {
        self.input[self.pos..].starts_with(s)
    }

    fn expect(&mut self, s: &str) -> ParserResult<()> {
        if !self.starts_with(s) {
            return Err(ParserError {
                message: format!(
                    "Expected: {:?} at byte {} but it was not found",
                    s, self.pos
                ),
                pos: self.pos,
            });
        }

        self.pos += s.len();

        Ok(())
    }

    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn consume(&mut self) -> ParserResult<char> {
        let c = self.next()?;
        self.pos += c.len_utf8();
        Ok(c)
    }

    fn consume_while(&mut self, predicate: impl Fn(char) -> bool) -> ParserResult<String> {
        let mut result = String::new();

        while !self.eof() && predicate(self.next()?) {
            result.push(self.consume()?);
        }

        Ok(result)
    }

    fn consume_whitespace(&mut self) -> ParserResult<String> {
        self.consume_while(char::is_whitespace)
    }

    fn parse_name(&mut self) -> ParserResult<String> {
        let name = self.consume_while(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9'))?;
        if name.is_empty() {
            return Err(ParserError {
                message: format!("Expected a name at byte {}", self.pos),
                pos: self.pos,
            });
        }

        Ok(name)
    }

    fn parse_node(&mut self) -> ParserResult<dom::Node> {
        if self.starts_with("<") {
            return self.parse_element();
        }

        self.parse_text()
    }

    fn parse_text(&mut self) -> ParserResult<dom::Node> {
        Ok(dom::text(self.consume_while(|c| c != '<')?))
    }

    fn parse_element(&mut self) -> ParserResult<dom::Node> {
        self.expect("<")?;
        let tag_name = self.parse_name()?;
        let attrs = self.parse_attributes()?;
        self.expect(">")?;

        let children = self.parse_nodes()?;

        self.expect("</")?;
        self.expect(&tag_name)?;
        self.expect(">")?;

        Ok(dom::elem(tag_name, attrs, children))
    }

    fn parse_attr(&mut self) -> ParserResult<(String, String)> {
        let name = self.parse_name()?;
        self.expect("=")?;

        let value = self.parse_attr_value()?;

        Ok((name, value))
    }

    fn parse_attr_value(&mut self) -> ParserResult<String> {
        let open_quote = self.consume()?;
        if open_quote != '"' && open_quote != '\'' {
            return Err(ParserError {
                message: format!(
                    "Expected an opening quote at byte {} but got {}",
                    self.pos, open_quote
                ),
                pos: self.pos,
            });
        }

        let value = self.consume_while(|c| c != open_quote)?;
        let close_quote = self.consume()?;
        if close_quote != open_quote {
            return Err(ParserError {
                message: format!(
                    "Expected a closing quote at byte {} but got {}",
                    self.pos, close_quote
                ),
                pos: self.pos,
            });
        }

        Ok(value)
    }

    fn parse_attributes(&mut self) -> ParserResult<dom::AttrMap> {
        let mut attributes = HashMap::new();

        loop {
            self.consume_whitespace()?;
            if self.eof() || self.next()? == '>' {
                break;
            }

            let (name, value) = self.parse_attr()?;
            attributes.insert(name, value);
        }

        Ok(attributes)
    }

    fn parse_nodes(&mut self) -> ParserResult<Vec<dom::Node>> {
        let mut nodes = Vec::new();

        loop {
            self.consume_whitespace()?;
            if self.eof() || self.starts_with("</") {
                break;
            }

            nodes.push(self.parse_node()?);
        }

        Ok(nodes)
    }
}
