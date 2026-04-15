use std::collections::{HashMap, HashSet};

pub struct Node {
    pub children: Vec<Node>,

    pub node_type: NodeType,
}

pub enum NodeType {
    Text(String),
    Element(ElementData),
}

pub struct ElementData {
    pub tag_name: String,
    pub attrs: AttrMap,
}

pub type AttrMap = HashMap<String, String>;

pub fn text(data: String) -> Node {
    Node {
        children: Vec::new(),
        node_type: NodeType::Text(data),
    }
}

pub fn elem(tag_name: String, attrs: AttrMap, children: Vec<Node>) -> Node {
    Node {
        children,
        node_type: NodeType::Element(ElementData { tag_name, attrs }),
    }
}

impl ElementData {
    pub fn id(&self) -> Option<&String> {
        self.attrs.get("id")
    }

    pub fn classes(&self) -> HashSet<&str> {
        match self.attrs.get("class") {
            Some(classlist) => classlist.split(' ').collect(),
            None => HashSet::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_node_has_no_children() {
        let node = text("hello".to_string());

        assert!(node.children.is_empty());
        assert!(matches!(node.node_type, NodeType::Text(ref s) if s == "hello"));
    }

    #[test]
    fn elem_node_stores_tag_and_attrs() {
        let mut attrs = HashMap::new();
        attrs.insert("id".to_string(), "main".to_string());
        let node = elem("div".to_string(), attrs, vec![]);

        match &node.node_type {
            NodeType::Element(data) => {
                assert_eq!(data.tag_name, "div");
                assert_eq!(data.attrs.get("id").unwrap(), "main");
            }
            _ => panic!("expected element"),
        }
    }

    #[test]
    fn elem_node_has_children() {
        let child = text("hi".to_string());
        let parent = elem("p".to_string(), HashMap::new(), vec![child]);

        assert_eq!(parent.children.len(), 1);
    }

    #[test]
    fn id_returns_id_attribute() {
        let mut attrs = HashMap::new();

        attrs.insert("id".to_string(), "main".to_string());

        let data = ElementData {
            tag_name: "div".to_string(),
            attrs,
        };

        assert_eq!(data.id(), Some(&"main".to_string()));
    }

    #[test]
    fn id_returns_none_when_missing() {
        let data = ElementData {
            tag_name: "div".to_string(),
            attrs: HashMap::new(),
        };

        assert_eq!(data.id(), None);
    }

    #[test]
    fn classes_parses_space_separated_list() {
        let mut attrs = HashMap::new();

        attrs.insert("class".to_string(), "foo bar baz".to_string());

        let data = ElementData {
            tag_name: "div".to_string(),
            attrs,
        };

        let classes = data.classes();
        assert!(classes.contains("foo"));
        assert!(classes.contains("bar"));
        assert!(classes.contains("baz"));
        assert_eq!(classes.len(), 3);
    }

    #[test]
    fn classes_returns_empty_when_missing() {
        let data = ElementData {
            tag_name: "div".to_string(),
            attrs: HashMap::new(),
        };

        assert!(data.classes().is_empty());
    }
}
