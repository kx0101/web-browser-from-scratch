use crate::css::{Unit, Value};
use crate::style::StyledNode;

#[derive(Clone, Copy, Default, Debug)]
pub struct Dimensions {
    pub content: Rect,

    pub padding: EdgeSizes,
    pub border: EdgeSizes,
    pub margin: EdgeSizes,
}

impl Dimensions {
    fn padding_box(&self) -> Rect {
        self.content.expanded_by(self.padding)
    }

    fn border_box(&self) -> Rect {
        self.padding_box().expanded_by(self.border)
    }

    fn margin_box(&self) -> Rect {
        self.border_box().expanded_by(self.margin)
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    fn expanded_by(&self, edge: EdgeSizes) -> Rect {
        Rect {
            x: self.x - edge.left,
            y: self.y - edge.top,
            width: self.width + edge.left + edge.right,
            height: self.height + edge.top + edge.bottom,
        }
    }
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

    pub fn get_style_node(&self) -> &'a StyledNode<'a> {
        match &self.box_type {
            BoxType::BlockNode(node) | BoxType::InlineNode(node) => node,
            BoxType::AnonymousBlock => panic!("Anonymous block does not have a style node"),
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

    pub fn layout(&mut self, containing_block: Dimensions) {
        match &self.box_type {
            BoxType::BlockNode(_) => self.layout_block(containing_block),
            BoxType::InlineNode(_) => unimplemented!(),
            BoxType::AnonymousBlock => self.layout_block(containing_block),
        }
    }

    // note: a block's width depends on its parent
    // while its height depends on its children
    //
    // so we have to traverse the tree twice: once top-down to calculate widths,
    // then bottom-up to calculate heights
    // which means parent's height is calculated after its children
    fn layout_block(&mut self, containing_block: Dimensions) {
        self.calculate_block_width(containing_block);

        self.calculate_block_position(containing_block);

        self.layout_block_children();

        self.calculate_block_height();
    }

    fn calculate_block_width(&mut self, containing_block: Dimensions) {
        let style = self.get_style_node();

        let auto = Value::Keyword("auto".to_string());
        let mut width = style.lookup("width", "width", &auto);

        let zero = Value::Length(0.0, Unit::Px);

        let mut margin_left = style.lookup("margin-left", "margin", &zero);
        let mut margin_right = style.lookup("margin-right", "margin", &zero);

        let border_left = style.lookup("border-left-width", "border-width", &zero);
        let border_right = style.lookup("border-right-width", "border-width", &zero);

        let padding_left = style.lookup("padding-left", "padding", &zero);
        let padding_right = style.lookup("padding-right", "padding", &zero);

        let total: f32 = [
            &margin_left,
            &margin_right,
            &border_left,
            &border_right,
            &padding_left,
            &padding_right,
            &width,
        ]
        .iter()
        .map(|v| v.to_px())
        .sum();

        if width != auto && total > containing_block.content.width {
            if margin_left == auto {
                margin_left = Value::Length(0.0, Unit::Px);
            }

            if margin_right == auto {
                margin_right = Value::Length(0.0, Unit::Px);
            }
        }

        let underflow = containing_block.content.width - total;

        match (width == auto, margin_left == auto, margin_right == auto) {
            (false, false, false) => {
                margin_right = Value::Length(margin_right.to_px() + underflow, Unit::Px)
            }
            (false, false, true) => margin_right = Value::Length(underflow, Unit::Px),
            (false, true, false) => margin_left = Value::Length(underflow, Unit::Px),
            (false, true, true) => {
                margin_left = Value::Length(underflow / 2.0, Unit::Px);
                margin_right = Value::Length(underflow / 2.0, Unit::Px);
            }
            (true, _, _) => {
                if margin_left == auto {
                    margin_left = Value::Length(0.0, Unit::Px);
                }

                if margin_right == auto {
                    margin_right = Value::Length(0.0, Unit::Px);
                }

                if underflow >= 0.0 {
                    width = Value::Length(underflow, Unit::Px);
                } else {
                    width = Value::Length(0.0, Unit::Px);
                    margin_right = Value::Length(margin_right.to_px() + underflow, Unit::Px);
                }
            }
        }

        let d = &mut self.dimensions;

        d.content.width = width.to_px();

        d.padding.left = padding_left.to_px();
        d.padding.right = padding_right.to_px();

        d.border.left = border_left.to_px();
        d.border.right = border_right.to_px();

        d.margin.left = margin_left.to_px();
        d.margin.right = margin_right.to_px();
    }

    fn calculate_block_position(&mut self, containing_block: Dimensions) {
        let style = self.get_style_node();
        let zero = Value::Length(0.0, Unit::Px);

        let margin_top = style.lookup("margin-top", "margin", &zero).to_px();
        let margin_bottom = style.lookup("margin-bottom", "margin", &zero).to_px();
        let border_top = style
            .lookup("border-top-width", "border-width", &zero)
            .to_px();
        let border_bottom = style
            .lookup("border-bottom-width", "border-width", &zero)
            .to_px();
        let padding_top = style.lookup("padding-top", "padding", &zero).to_px();
        let padding_bottom = style.lookup("padding-bottom", "padding", &zero).to_px();

        let d = &mut self.dimensions;

        d.margin.top = margin_top;
        d.margin.bottom = margin_bottom;

        d.border.top = border_top;
        d.border.bottom = border_bottom;

        d.padding.top = padding_top;
        d.padding.bottom = padding_bottom;

        d.content.x = containing_block.content.x + d.margin.left + d.border.left + d.padding.left;
        d.content.y = containing_block.content.y
            + containing_block.content.height
            + d.margin.top
            + d.border.top
            + d.padding.top;
    }

    fn calculate_block_height(&mut self) {
        if let Some(Value::Length(h, Unit::Px)) = self.get_style_node().value("height") {
            self.dimensions.content.height = h;
        }
    }

    fn layout_block_children(&mut self) {
        for child in &mut self.children {
            child.layout(self.dimensions);
            self.dimensions.content.height += child.dimensions.margin_box().height;
        }
    }
}

pub enum BoxType<'a> {
    BlockNode(&'a StyledNode<'a>),
    InlineNode(&'a StyledNode<'a>),
    AnonymousBlock,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

    fn with_laid_out_tree(
        html: &str,
        css: &str,
        containing_block: Dimensions,
        assert_layout: impl FnOnce(&LayoutBox),
    ) {
        let dom = HtmlParser::parse(html.to_string()).unwrap();
        let stylesheet = css::parse(css.to_string()).unwrap();
        let styled = style_tree(&dom, &stylesheet);
        let mut layout = build_layout_tree(&styled);

        layout.layout(containing_block);
        assert_layout(&layout);
    }

    fn containing_block(width: f32) -> Dimensions {
        Dimensions {
            content: Rect {
                width,
                ..Default::default()
            },
            ..Default::default()
        }
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

    #[test]
    fn gets_style_node_for_block_and_inline_boxes() {
        with_layout_tree(
            "<div><span></span></div>",
            "div { display: block; } span { display: inline; }",
            |layout| {
                assert_eq!(layout.get_style_node().display(), Display::Block);

                let anonymous = &layout.children[0];
                assert_eq!(
                    anonymous.children[0].get_style_node().display(),
                    Display::Inline
                );
            },
        );
    }

    #[test]
    #[should_panic(expected = "Anonymous block does not have a style node")]
    fn anonymous_blocks_do_not_have_style_nodes() {
        with_layout_tree(
            "<div><span></span></div>",
            "div { display: block; } span { display: inline; }",
            |layout| {
                layout.children[0].get_style_node();
            },
        );
    }

    #[test]
    fn block_layout_uses_css_box_edges() {
        with_laid_out_tree(
            "<div></div>",
            "
                div {
                    display: block;
                    width: 100px;
                    height: 31px;
                    margin-left: 10px;
                    margin-right: 20px;
                    margin-top: 11px;
                    margin-bottom: 13px;
                    padding-left: 5px;
                    padding-right: 7px;
                    padding-top: 17px;
                    padding-bottom: 19px;
                    border-left-width: 2px;
                    border-right-width: 3px;
                    border-top-width: 23px;
                    border-bottom-width: 29px;
                }
            ",
            containing_block(147.0),
            |layout| {
                let d = layout.dimensions;

                assert_eq!(d.content.x, 17.0);
                assert_eq!(d.content.y, 51.0);
                assert_eq!(d.content.width, 100.0);
                assert_eq!(d.content.height, 31.0);

                assert_eq!(d.margin.left, 10.0);
                assert_eq!(d.margin.right, 20.0);
                assert_eq!(d.margin.top, 11.0);
                assert_eq!(d.margin.bottom, 13.0);

                assert_eq!(d.padding.left, 5.0);
                assert_eq!(d.padding.right, 7.0);
                assert_eq!(d.padding.top, 17.0);
                assert_eq!(d.padding.bottom, 19.0);

                assert_eq!(d.border.left, 2.0);
                assert_eq!(d.border.right, 3.0);
                assert_eq!(d.border.top, 23.0);
                assert_eq!(d.border.bottom, 29.0);
            },
        );
    }

    #[test]
    fn auto_width_fills_remaining_containing_block_width() {
        with_laid_out_tree(
            "<div></div>",
            "
                div {
                    display: block;
                    margin-left: 10px;
                    margin-right: 20px;
                    padding-left: 5px;
                    padding-right: 5px;
                    border-left-width: 2px;
                    border-right-width: 3px;
                }
            ",
            containing_block(200.0),
            |layout| {
                assert_eq!(layout.dimensions.content.width, 155.0);
                assert_eq!(layout.dimensions.margin.left, 10.0);
                assert_eq!(layout.dimensions.margin.right, 20.0);
            },
        );
    }

    #[test]
    fn auto_horizontal_margins_split_underflow() {
        with_laid_out_tree(
            "<div></div>",
            "
                div {
                    display: block;
                    width: 100px;
                    margin-left: auto;
                    margin-right: auto;
                }
            ",
            containing_block(200.0),
            |layout| {
                assert_eq!(layout.dimensions.content.width, 100.0);
                assert_eq!(layout.dimensions.margin.left, 50.0);
                assert_eq!(layout.dimensions.margin.right, 50.0);
            },
        );
    }

    #[test]
    fn block_children_stack_vertically_and_expand_parent_height() {
        with_laid_out_tree(
            "<div><p></p><section></section></div>",
            "
                div, p, section { display: block; }
                p {
                    height: 10px;
                    margin-top: 1px;
                    margin-bottom: 2px;
                }
                section {
                    height: 20px;
                    margin-top: 3px;
                    margin-bottom: 4px;
                }
            ",
            containing_block(100.0),
            |layout| {
                let first_child = &layout.children[0];
                let second_child = &layout.children[1];

                assert_eq!(first_child.dimensions.content.y, 1.0);
                assert_eq!(first_child.dimensions.content.height, 10.0);

                assert_eq!(second_child.dimensions.content.y, 16.0);
                assert_eq!(second_child.dimensions.content.height, 20.0);

                assert_eq!(layout.dimensions.content.height, 40.0);
            },
        );
    }
}
