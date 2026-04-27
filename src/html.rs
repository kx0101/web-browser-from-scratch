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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::NodeType;

    #[test]
    fn parse_single_text_node() {
        let node = HtmlParser::parse("hello world".to_string()).unwrap();
        assert!(matches!(node.node_type, NodeType::Text(ref s) if s == "hello world"));
    }

    #[test]
    fn parse_simple_element() {
        let node = HtmlParser::parse("<p></p>".to_string()).unwrap();
        match &node.node_type {
            NodeType::Element(data) => assert_eq!(data.tag_name, "p"),
            _ => panic!("expected element"),
        }
    }

    #[test]
    fn parse_element_with_text_child() {
        let node = HtmlParser::parse("<p>hello</p>".to_string()).unwrap();
        match &node.node_type {
            NodeType::Element(data) => {
                assert_eq!(data.tag_name, "p");
                assert_eq!(node.children.len(), 1);
                assert!(matches!(&node.children[0].node_type, NodeType::Text(s) if s == "hello"));
            }
            _ => panic!("expected element"),
        }
    }

    #[test]
    fn parse_element_with_attributes() {
        let node =
            HtmlParser::parse(r#"<div id="main" class="container"></div>"#.to_string()).unwrap();
        match &node.node_type {
            NodeType::Element(data) => {
                assert_eq!(data.tag_name, "div");
                assert_eq!(data.attrs.get("id").unwrap(), "main");
                assert_eq!(data.attrs.get("class").unwrap(), "container");
            }
            _ => panic!("expected element"),
        }
    }

    #[test]
    fn parse_nested_elements() {
        let html = "<div><p><span>hi</span></p></div>".to_string();
        let node = HtmlParser::parse(html).unwrap();

        match &node.node_type {
            NodeType::Element(data) => assert_eq!(data.tag_name, "div"),
            _ => panic!("expected div"),
        }

        let p = &node.children[0];
        match &p.node_type {
            NodeType::Element(data) => assert_eq!(data.tag_name, "p"),
            _ => panic!("expected p"),
        }

        let span = &p.children[0];
        match &span.node_type {
            NodeType::Element(data) => assert_eq!(data.tag_name, "span"),
            _ => panic!("expected span"),
        }

        assert!(matches!(&span.children[0].node_type, NodeType::Text(s) if s == "hi"));
    }

    #[test]
    fn parse_sibling_elements() {
        let html = "<div><p>one</p><p>two</p></div>".to_string();
        let node = HtmlParser::parse(html).unwrap();
        assert_eq!(node.children.len(), 2);
    }

    #[test]
    fn parse_single_quoted_attributes() {
        let node = HtmlParser::parse("<div id='main'></div>".to_string()).unwrap();
        match &node.node_type {
            NodeType::Element(data) => assert_eq!(data.attrs.get("id").unwrap(), "main"),
            _ => panic!("expected element"),
        }
    }

    #[test]
    fn parse_wraps_multiple_roots_in_html_element() {
        let node = HtmlParser::parse("<p>a</p><p>b</p>".to_string()).unwrap();
        match &node.node_type {
            NodeType::Element(data) => {
                assert_eq!(data.tag_name, "html");
                assert_eq!(node.children.len(), 2);
            }
            _ => panic!("expected html wrapper"),
        }
    }

    #[test]
    fn parse_unclosed_tag_returns_error() {
        assert!(HtmlParser::parse("<div>".to_string()).is_err());
    }
}
