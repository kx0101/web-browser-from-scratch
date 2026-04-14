use crate::parser::{Parser, ParserResult};

#[derive(Debug)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

#[derive(Debug)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}

#[derive(Debug)]
pub enum Selector {
    Simple(SimpleSelector),
}

#[derive(Debug)]
pub struct SimpleSelector {
    pub tag_name: Option<String>,
    pub id: Option<String>,
    pub class: Vec<String>,
}

#[derive(Debug)]
pub struct Declaration {
    pub name: String,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Keyword(String),
    Length(f32, Unit),
    ColorValue(Color),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Unit {
    Px,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

pub type Specificity = (usize, usize, usize);

impl Selector {
    pub fn specificity(&self) -> Specificity {
        let Selector::Simple(ref simple) = *self;

        let a = simple.id.iter().count();
        let b = simple.class.len();
        let c = simple.tag_name.iter().count();

        (a, b, c)
    }
}

impl Value {
    pub fn to_px(&self) -> f32 {
        match *self {
            Value::Length(f, Unit::Px) => f,
            _ => 0.0,
        }
    }
}

pub fn parse(source: String) -> ParserResult<Stylesheet> {
    let mut css = CssParser {
        parser: Parser::new(source),
    };

    Ok(Stylesheet {
        rules: css.parse_rules()?,
    })
}

struct CssParser {
    parser: Parser,
}

impl CssParser {
    fn parse_rules(&mut self) -> ParserResult<Vec<Rule>> {
        let mut rules = Vec::new();

        loop {
            self.parser.consume_whitespace()?;
            if self.parser.eof() {
                break;
            }

            rules.push(self.parse_rule()?);
        }

        Ok(rules)
    }

    fn parse_rule(&mut self) -> ParserResult<Rule> {
        Ok(Rule {
            selectors: self.parse_selectors()?,
            declarations: self.parse_declarations()?,
        })
    }

    fn parse_selectors(&mut self) -> ParserResult<Vec<Selector>> {
        let mut selectors = Vec::new();

        loop {
            selectors.push(Selector::Simple(self.parse_simple_selector()?));
            self.parser.consume_whitespace()?;

            match self.parser.next()? {
                ',' => {
                    self.parser.consume()?;
                    self.parser.consume_whitespace()?;
                }
                '{' => break,
                c => {
                    return Err(self
                        .parser
                        .err(format!("Unexpected character {:?} in selector list", c)));
                }
            }
        }

        selectors.sort_by_key(|s| s.specificity());
        Ok(selectors)
    }

    fn parse_simple_selector(&mut self) -> ParserResult<SimpleSelector> {
        let mut selector = SimpleSelector {
            tag_name: None,
            id: None,
            class: Vec::new(),
        };

        while !self.parser.eof() {
            match self.parser.next()? {
                '#' => {
                    self.parser.consume()?;
                    selector.id = Some(self.parse_identifier()?);
                }
                '.' => {
                    self.parser.consume()?;
                    selector.class.push(self.parse_identifier()?);
                }
                '*' => {
                    self.parser.consume()?;
                }
                c if valid_identifier_char(c) => {
                    selector.tag_name = Some(self.parse_identifier()?);
                }
                _ => break,
            }
        }

        Ok(selector)
    }

    fn parse_declarations(&mut self) -> ParserResult<Vec<Declaration>> {
        self.parser.expect_char('{')?;
        let mut declarations = Vec::new();

        loop {
            self.parser.consume_whitespace()?;
            if self.parser.next()? == '}' {
                self.parser.consume()?;
                break;
            }

            declarations.push(self.parse_declaration()?);
        }

        Ok(declarations)
    }

    fn parse_declaration(&mut self) -> ParserResult<Declaration> {
        let name = self.parse_identifier()?;
        self.parser.consume_whitespace()?;
        self.parser.expect_char(':')?;
        self.parser.consume_whitespace()?;

        let value = self.parse_value()?;
        self.parser.consume_whitespace()?;
        self.parser.expect_char(';')?;

        Ok(Declaration { name, value })
    }

    fn parse_value(&mut self) -> ParserResult<Value> {
        match self.parser.next()? {
            '0'..='9' => self.parse_length(),
            '#' => self.parse_color(),
            _ => Ok(Value::Keyword(self.parse_identifier()?)),
        }
    }

    fn parse_length(&mut self) -> ParserResult<Value> {
        Ok(Value::Length(self.parse_float()?, self.parse_unit()?))
    }

    fn parse_float(&mut self) -> ParserResult<f32> {
        let s = self
            .parser
            .consume_while(|c| matches!(c, '0'..='9' | '.'))?;

        s.parse::<f32>()
            .map_err(|_| self.parser.err(format!("Invalid float: {:?}", s)))
    }

    fn parse_unit(&mut self) -> ParserResult<Unit> {
        let ident = self.parse_identifier()?.to_ascii_lowercase();

        match &*ident {
            "px" => Ok(Unit::Px),
            _ => Err(self.parser.err(format!("Unrecognized unit: {:?}", ident))),
        }
    }

    fn parse_color(&mut self) -> ParserResult<Value> {
        self.parser.expect_char('#')?;

        Ok(Value::ColorValue(Color {
            r: self.parse_hex_pair()?,
            g: self.parse_hex_pair()?,
            b: self.parse_hex_pair()?,
            a: 255,
        }))
    }

    fn parse_hex_pair(&mut self) -> ParserResult<u8> {
        if self.parser.pos + 2 > self.parser.input.len() {
            return Err(self.parser.err("Unexpected end of input in hex color"));
        }

        let s = &self.parser.input[self.parser.pos..self.parser.pos + 2];

        self.parser.pos += 2;

        u8::from_str_radix(s, 16).map_err(|_| self.parser.err(format!("Invalid hex pair: {:?}", s)))
    }

    fn parse_identifier(&mut self) -> ParserResult<String> {
        let ident = self.parser.consume_while(valid_identifier_char)?;
        if ident.is_empty() {
            return Err(self.parser.err("Expected identifier"));
        }

        Ok(ident)
    }
}

fn valid_identifier_char(c: char) -> bool {
    matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_')
}
