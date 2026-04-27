use crate::style::StyledNode;

#[derive(Clone, Copy, Default, Debug)]
pub struct Dimensions {
    pub content: Rect,

    pub padding: EdgeSizes,
    pub border: EdgeSizes,
    pub margin: EdgeSizes,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct EdgeSizes {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

pub struct LayoutBox<'a> {
    pub dimensions: Dimensions,
    pub box_type: BoxType<'a>,
    pub children: Vec<LayoutBox<'a>>,
}

impl<'a> LayoutBox<'a> {
    fn new(box_type: BoxType<'a>) -> LayoutBox<'a> {
        LayoutBox {
            box_type,
            dimensions: Default::default(),
            children: Vec::new(),
        }
    }

    fn get_inline_container(&mut self) -> &mut LayoutBox<'a> {
        match &self.box_type {
            BoxType::InlineNode(_) | BoxType::AnonymousBlock => self,
            BoxType::BlockNode(_) => {
                match self.children.last() {
                    Some(&LayoutBox {
                        box_type: BoxType::AnonymousBlock,
                        ..
                    }) => {}
                    _ => self.children.push(LayoutBox::new(BoxType::AnonymousBlock)),
                }

                self.children.last_mut().unwrap()
            }
        }
    }
}

pub enum BoxType<'a> {
    BlockNode(&'a StyledNode<'a>),
    InlineNode(&'a StyledNode<'a>),
    AnonymousBlock,
}

pub enum Display {
    Inline,
    Block,
    None,
}

pub fn build_layout_tree<'a>(style_node: &'a StyledNode<'a>) -> LayoutBox<'a> {
    let mut root = LayoutBox::new(match style_node.display() {
        Display::Block => BoxType::BlockNode(style_node),
        Display::Inline => BoxType::InlineNode(style_node),
        Display::None => panic!("root node has display: none"),
    });

    for child in &style_node.children {
        match child.display() {
            Display::Block => root.children.push(build_layout_tree(child)),
            Display::Inline => root
                .get_inline_container()
                .children
                .push(build_layout_tree(child)),
            Display::None => {}
        }
    }

    root
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{css, html::HtmlParser, style::style_tree};

    fn with_layout_tree(html: &str, css: &str, assert_layout: impl FnOnce(&LayoutBox)) {
        let dom = HtmlParser::parse(html.to_string()).unwrap();
        let stylesheet = css::parse(css.to_string()).unwrap();
        let styled = style_tree(&dom, &stylesheet);
        let layout = build_layout_tree(&styled);

        assert_layout(&layout);
    }

    #[test]
    fn builds_block_layout_tree() {
        with_layout_tree(
            "<div><p></p><section></section></div>",
            "div, p, section { display: block; }",
            |layout| {
                assert!(matches!(layout.box_type, BoxType::BlockNode(_)));
                assert_eq!(layout.children.len(), 2);
                assert!(matches!(layout.children[0].box_type, BoxType::BlockNode(_)));
                assert!(matches!(layout.children[1].box_type, BoxType::BlockNode(_)));
            },
        );
    }

    #[test]
    fn wraps_inline_children_in_anonymous_block() {
        with_layout_tree(
            "<div><span></span><em></em></div>",
            "div { display: block; } span, em { display: inline; }",
            |layout| {
                assert!(matches!(layout.box_type, BoxType::BlockNode(_)));
                assert_eq!(layout.children.len(), 1);

                let anonymous = &layout.children[0];
                assert!(matches!(anonymous.box_type, BoxType::AnonymousBlock));
                assert_eq!(anonymous.children.len(), 2);
                assert!(matches!(
                    anonymous.children[0].box_type,
                    BoxType::InlineNode(_)
                ));
                assert!(matches!(
                    anonymous.children[1].box_type,
                    BoxType::InlineNode(_)
                ));
            },
        );
    }

    #[test]
    fn separates_inline_runs_around_block_children() {
        with_layout_tree(
            "<div><span></span><p></p><em></em></div>",
            "div, p { display: block; } span, em { display: inline; }",
            |layout| {
                assert_eq!(layout.children.len(), 3);
                assert!(matches!(
                    layout.children[0].box_type,
                    BoxType::AnonymousBlock
                ));
                assert!(matches!(layout.children[1].box_type, BoxType::BlockNode(_)));
                assert!(matches!(
                    layout.children[2].box_type,
                    BoxType::AnonymousBlock
                ));
            },
        );
    }

    #[test]
    fn skips_display_none_children() {
        with_layout_tree(
            "<div><p></p><script></script><section></section></div>",
            "div, p, section { display: block; } script { display: none; }",
            |layout| {
                assert_eq!(layout.children.len(), 2);
                assert!(matches!(layout.children[0].box_type, BoxType::BlockNode(_)));
                assert!(matches!(layout.children[1].box_type, BoxType::BlockNode(_)));
            },
        );
    }

    #[test]
    #[should_panic(expected = "root node has display: none")]
    fn panics_when_root_is_display_none() {
        with_layout_tree("<div></div>", "div { display: none; }", |_| {});
    }
}
