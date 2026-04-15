use std::collections::HashMap;

use crate::dom;
use crate::parser::{Parser, ParserResult};

pub struct HtmlParser {
    parser: Parser,
}

impl HtmlParser {
    pub fn parse(source: String) -> ParserResult<dom::Node> {
        let mut html = HtmlParser {
            parser: Parser::new(source),
        };

        let mut nodes = html.parse_nodes()?;
        if nodes.len() == 1 {
            return Ok(nodes.remove(0));
        }

        Ok(dom::elem("html".to_string(), HashMap::new(), nodes))
    }

    fn parse_name(&mut self) -> ParserResult<String> {
        let name = self
            .parser
            .consume_while(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9'))?;

        if name.is_empty() {
            return Err(self.parser.err("Expected tag or attribute name"));
        }

        Ok(name)
    }

    fn parse_node(&mut self) -> ParserResult<dom::Node> {
        if self.parser.starts_with("<") {
            return self.parse_element();
        }

        self.parse_text()
    }

    fn parse_text(&mut self) -> ParserResult<dom::Node> {
        Ok(dom::text(self.parser.consume_while(|c| c != '<')?))
    }

    fn parse_element(&mut self) -> ParserResult<dom::Node> {
        self.parser.expect("<")?;
        let tag_name = self.parse_name()?;
        let attrs = self.parse_attributes()?;
        self.parser.expect(">")?;

        let children = self.parse_nodes()?;

        self.parser.expect("</")?;
        self.parser.expect(&tag_name)?;
        self.parser.expect(">")?;

        Ok(dom::elem(tag_name, attrs, children))
    }

    fn parse_attr(&mut self) -> ParserResult<(String, String)> {
        let name = self.parse_name()?;
        self.parser.expect("=")?;

        let value = self.parse_attr_value()?;

        Ok((name, value))
    }

    fn parse_attr_value(&mut self) -> ParserResult<String> {
        let open_quote = self.parser.consume()?;
        if open_quote != '"' && open_quote != '\'' {
            return Err(self
                .parser
                .err(format!("Expected opening quote, got {:?}", open_quote)));
        }

        let value = self.parser.consume_while(|c| c != open_quote)?;
        let close_quote = self.parser.consume()?;
        if close_quote != open_quote {
            return Err(self.parser.err(format!(
                "Expected closing {:?}, got {:?}",
                open_quote, close_quote
            )));
        }

        Ok(value)
    }

    fn parse_attributes(&mut self) -> ParserResult<dom::AttrMap> {
        let mut attributes = HashMap::new();

        loop {
            self.parser.consume_whitespace()?;
            if self.parser.eof() {
                return Err(self
                    .parser
                    .err("Unexpected end of input while parsing attributes"));
            }
            if self.parser.next()? == '>' {
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
            self.parser.consume_whitespace()?;
            if self.parser.eof() || self.parser.starts_with("</") {
                break;
            }

            nodes.push(self.parse_node()?);
        }

        Ok(nodes)
    }
}
