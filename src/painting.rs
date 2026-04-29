use crate::{
    css::{Color, Value},
    layout::{BoxType, LayoutBox, Rect},
};

pub struct Canvas {
    pub pixels: Vec<Color>,
    pub width: usize,
    pub height: usize,
}

impl Canvas {
    fn new(width: usize, height: usize) -> Canvas {
        let white = Color {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        };

        Canvas {
            pixels: vec![white; width * height],
            width,
            height,
        }
    }

    fn paint_item(&mut self, item: &DisplayCommand) {
        match item {
            &DisplayCommand::SolidColor(color, rect) => {
                let x0 = rect.x.clamp(0.0, self.width as f32) as usize;
                let y0 = rect.y.clamp(0.0, self.height as f32) as usize;

                let x1 = (rect.x + rect.width).clamp(0.0, self.width as f32) as usize;
                let y1 = (rect.y + rect.height).clamp(0.0, self.height as f32) as usize;

                for y in y0..y1 {
                    for x in x0..x1 {
                        self.pixels[x + y * self.width] = color;
                    }
                }
            }
        }
    }
}

type DisplayList = Vec<DisplayCommand>;

enum DisplayCommand {
    SolidColor(Color, Rect),
}

pub fn paint(layout_root: &LayoutBox, bounds: Rect) -> Canvas {
    let display_list = build_display_list(layout_root);
    let mut canvas = Canvas::new(bounds.width as usize, bounds.height as usize);

    for item in display_list {
        canvas.paint_item(&item);
    }

    canvas
}

fn build_display_list(layout_root: &LayoutBox) -> DisplayList {
    let mut list = Vec::new();

    render_layout_box(&mut list, layout_root);

    list
}

fn render_layout_box(list: &mut DisplayList, layout_box: &LayoutBox) {
    render_background(list, layout_box);
    render_borders(list, layout_box);

    for child in &layout_box.children {
        render_layout_box(list, child);
    }
}

fn render_background(list: &mut DisplayList, layout_box: &LayoutBox) {
    if let Some(color) = get_color(layout_box, "background") {
        list.push(DisplayCommand::SolidColor(
            color,
            layout_box.dimensions.border_box(),
        ));
    }
}

fn get_color(layout_box: &LayoutBox, name: &str) -> Option<Color> {
    match layout_box.box_type {
        BoxType::BlockNode(style) | BoxType::InlineNode(style) => match style.value(name) {
            Some(Value::ColorValue(color)) => Some(color),
            _ => None,
        },
        BoxType::AnonymousBlock => None,
    }
}

fn render_borders(list: &mut DisplayList, layout_box: &LayoutBox) {
    let color = match get_color(layout_box, "border-color") {
        Some(color) => color,
        _ => return,
    };

    let d = &layout_box.dimensions;
    let border_box = d.border_box();

    list.push(DisplayCommand::SolidColor(
        color,
        Rect {
            x: border_box.x,
            y: border_box.y,
            width: d.border.left,
            height: border_box.height,
        },
    ));

    list.push(DisplayCommand::SolidColor(
        color,
        Rect {
            x: border_box.x + border_box.width - d.border.right,
            y: border_box.y,
            width: d.border.right,
            height: border_box.height,
        },
    ));

    list.push(DisplayCommand::SolidColor(
        color,
        Rect {
            x: border_box.x,
            y: border_box.y,
            width: border_box.width,
            height: d.border.top,
        },
    ));

    list.push(DisplayCommand::SolidColor(
        color,
        Rect {
            x: border_box.x,
            y: border_box.y + border_box.height - d.border.bottom,
            width: border_box.width,
            height: d.border.bottom,
        },
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        css,
        html::HtmlParser,
        layout::{self, Dimensions},
        style,
    };

    fn paint_document(html: &str, css: &str, bounds: Rect) -> Canvas {
        let dom = HtmlParser::parse(html.to_string()).unwrap();
        let stylesheet = css::parse(css.to_string()).unwrap();
        let styled = style::style_tree(&dom, &stylesheet);
        let mut layout_root = layout::build_layout_tree(&styled);

        let mut containing_block: Dimensions = Default::default();
        containing_block.content.width = bounds.width;
        layout_root.layout(containing_block);

        paint(&layout_root, bounds)
    }

    fn pixel(canvas: &Canvas, x: usize, y: usize) -> Color {
        canvas.pixels[x + y * canvas.width]
    }

    fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color { r, g, b, a: 255 }
    }

    #[test]
    fn paints_background_rectangle() {
        let canvas = paint_document(
            "<div></div>",
            "div { display: block; width: 2px; height: 2px; background: #ff0000; }",
            Rect {
                width: 4.0,
                height: 4.0,
                ..Default::default()
            },
        );

        assert_eq!(pixel(&canvas, 0, 0), rgb(255, 0, 0));
        assert_eq!(pixel(&canvas, 1, 1), rgb(255, 0, 0));
        assert_eq!(pixel(&canvas, 2, 2), rgb(255, 255, 255));
    }

    #[test]
    fn paints_borders_over_background() {
        let canvas = paint_document(
            "<div></div>",
            "div {
                display: block;
                width: 4px;
                height: 4px;
                background: #ffffff;
                border-width: 1px;
                border-color: #000000;
            }",
            Rect {
                width: 8.0,
                height: 8.0,
                ..Default::default()
            },
        );

        assert_eq!(pixel(&canvas, 0, 0), rgb(0, 0, 0));
        assert_eq!(pixel(&canvas, 5, 5), rgb(0, 0, 0));
        assert_eq!(pixel(&canvas, 1, 1), rgb(255, 255, 255));
    }

    #[test]
    fn clips_to_canvas_height_not_width() {
        let canvas = paint_document(
            "<div></div>",
            "div { display: block; width: 1px; height: 4px; background: #00ff00; }",
            Rect {
                width: 2.0,
                height: 4.0,
                ..Default::default()
            },
        );

        assert_eq!(pixel(&canvas, 0, 3), rgb(0, 255, 0));
    }

    #[test]
    fn paints_nested_rainbow_divs() {
        let canvas = paint_document(
            r#"
            <div class="a">
              <div class="b">
                <div class="c">
                  <div class="d">
                    <div class="e">
                      <div class="f">
                        <div class="g"></div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
            "#,
            r#"
            * { display: block; padding: 12px; }
            .a { background: #ff0000; }
            .b { background: #ffa500; }
            .c { background: #ffff00; }
            .d { background: #008000; }
            .e { background: #0000ff; }
            .f { background: #4b0082; }
            .g { background: #800080; }
            "#,
            Rect {
                width: 120.0,
                height: 120.0,
                ..Default::default()
            },
        );

        assert_eq!(pixel(&canvas, 0, 0), rgb(255, 0, 0));
        assert_eq!(pixel(&canvas, 12, 12), rgb(255, 165, 0));
        assert_eq!(pixel(&canvas, 24, 24), rgb(255, 255, 0));
        assert_eq!(pixel(&canvas, 36, 36), rgb(0, 128, 0));
        assert_eq!(pixel(&canvas, 48, 48), rgb(0, 0, 255));
        assert_eq!(pixel(&canvas, 60, 60), rgb(75, 0, 130));
        assert_eq!(pixel(&canvas, 72, 72), rgb(128, 0, 128));
    }
}
