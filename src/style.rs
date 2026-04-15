use std::collections::HashMap;

use crate::{
    css::{Rule, Selector, SimpleSelector, Specificity, Stylesheet, Value},
    dom::{ElementData, Node, NodeType},
};

type PropertyMap = HashMap<String, Value>;

type MatchedRule<'a> = (Specificity, &'a Rule);

struct StyledNode<'a> {
    node: &'a Node,
    specified_values: PropertyMap,
    children: Vec<StyledNode<'a>>,
}

pub fn style_tree<'a>(root: &'a Node, stylesheet: &'a Stylesheet) -> StyledNode<'a> {
    StyledNode {
        node: root,
        specified_values: match &root.node_type {
            NodeType::Element(elem) => specified_values(elem, stylesheet),
            NodeType::Text(_) => HashMap::new(),
        },
        children: root
            .children
            .iter()
            .map(|child| style_tree(child, stylesheet))
            .collect(),
    }
}

fn specified_values(elem: &ElementData, stylesheet: &Stylesheet) -> PropertyMap {
    let mut values = HashMap::new();
    let mut rules = matching_rules(elem, stylesheet);

    rules.sort_by_key(|(specificity, _rule)| *specificity);

    for (_specificity, rule) in rules {
        for declaration in &rule.declarations {
            values.insert(declaration.name.clone(), declaration.value.clone());
        }
    }

    values
}

fn matching_rules<'a>(elem: &ElementData, stylesheet: &'a Stylesheet) -> Vec<MatchedRule<'a>> {
    stylesheet
        .rules
        .iter()
        .filter_map(|rule| match_rule(elem, rule))
        .collect()
}

fn match_rule<'a>(elem: &ElementData, rule: &'a Rule) -> Option<MatchedRule<'a>> {
    rule.selectors
        .iter()
        .find(|selector| matches(elem, selector))
        .map(|selector| (selector.specificity(), rule))
}

fn matches(elem: &ElementData, selector: &Selector) -> bool {
    match selector {
        Selector::Simple(s) => match_simple_selector(elem, s),
    }
}

fn match_simple_selector(elem: &ElementData, selector: &SimpleSelector) -> bool {
    if selector.tag_name.iter().any(|name| *name != elem.tag_name) {
        return false;
    }

    if selector.id.iter().any(|id| Some(id) != elem.id()) {
        return false;
    }

    if selector
        .class
        .iter()
        .any(|class| !elem.classes().contains(class.as_str()))
    {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::{self, SimpleSelector, Value};
    use std::collections::HashMap;

    fn make_element(tag: &str, id: Option<&str>, classes: Option<&str>) -> Node {
        let mut attrs = HashMap::new();
        if let Some(id) = id {
            attrs.insert("id".to_string(), id.to_string());
        }
        if let Some(classes) = classes {
            attrs.insert("class".to_string(), classes.to_string());
        }

        crate::dom::elem(tag.to_string(), attrs, vec![])
    }

    fn simple_selector(tag: Option<&str>, id: Option<&str>, classes: Vec<&str>) -> SimpleSelector {
        SimpleSelector {
            tag_name: tag.map(|s| s.to_string()),
            id: id.map(|s| s.to_string()),
            class: classes.into_iter().map(|s| s.to_string()).collect(),
        }
    }

    fn elem_data(node: &Node) -> &ElementData {
        match &node.node_type {
            NodeType::Element(data) => data,
            _ => panic!("expected element"),
        }
    }

    #[test]
    fn match_tag_selector() {
        let node = make_element("div", None, None);
        let selector = simple_selector(Some("div"), None, vec![]);
        assert!(match_simple_selector(elem_data(&node), &selector));
    }

    #[test]
    fn no_match_wrong_tag() {
        let node = make_element("div", None, None);
        let selector = simple_selector(Some("p"), None, vec![]);
        assert!(!match_simple_selector(elem_data(&node), &selector));
    }

    #[test]
    fn match_id_selector() {
        let node = make_element("div", Some("main"), None);
        let selector = simple_selector(None, Some("main"), vec![]);
        assert!(match_simple_selector(elem_data(&node), &selector));
    }

    #[test]
    fn no_match_wrong_id() {
        let node = make_element("div", Some("main"), None);
        let selector = simple_selector(None, Some("sidebar"), vec![]);
        assert!(!match_simple_selector(elem_data(&node), &selector));
    }

    #[test]
    fn match_class_selector() {
        let node = make_element("div", None, Some("active visible"));
        let selector = simple_selector(None, None, vec!["active"]);
        assert!(match_simple_selector(elem_data(&node), &selector));
    }

    #[test]
    fn no_match_missing_class() {
        let node = make_element("div", None, Some("active"));
        let selector = simple_selector(None, None, vec!["hidden"]);
        assert!(!match_simple_selector(elem_data(&node), &selector));
    }

    #[test]
    fn universal_selector_matches_any_element() {
        let node = make_element("div", None, None);
        let selector = simple_selector(None, None, vec![]);
        assert!(match_simple_selector(elem_data(&node), &selector));
    }

    #[test]
    fn match_compound_selector() {
        let node = make_element("div", Some("main"), Some("active"));
        let selector = simple_selector(Some("div"), Some("main"), vec!["active"]);
        assert!(match_simple_selector(elem_data(&node), &selector));
    }

    #[test]
    fn higher_specificity_wins() {
        let css_source = r#"
            div { color: #ff0000; }
            #main { color: #00ff00; }
        "#;
        let stylesheet = css::parse(css_source.to_string()).unwrap();

        let node = make_element("div", Some("main"), None);
        let values = specified_values(elem_data(&node), &stylesheet);

        assert_eq!(
            values.get("color").unwrap(),
            &Value::ColorValue(css::Color { r: 0, g: 255, b: 0, a: 255 })
        );
    }

    #[test]
    fn text_node_gets_no_styles() {
        let stylesheet = css::parse("div { display: block; }".to_string()).unwrap();
        let text_node = crate::dom::text("hello".to_string());
        let styled = style_tree(&text_node, &stylesheet);

        assert!(styled.specified_values.is_empty());
    }

    #[test]
    fn style_tree_preserves_children() {
        let html = "<div><p>hi</p><p>there</p></div>";
        let dom = crate::html::HtmlParser::parse(html.to_string()).unwrap();
        let stylesheet = css::parse("".to_string()).unwrap();
        let styled = style_tree(&dom, &stylesheet);

        assert_eq!(styled.children.len(), 2);
    }

    #[test]
    fn unmatched_element_gets_empty_styles() {
        let stylesheet = css::parse("p { display: block; }".to_string()).unwrap();
        let node = make_element("div", None, None);
        let values = specified_values(elem_data(&node), &stylesheet);

        assert!(values.is_empty());
    }
}
