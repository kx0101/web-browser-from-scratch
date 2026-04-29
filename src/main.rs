use std::{error::Error, fs, path::PathBuf};

use clap::Parser;

pub mod css;
pub mod dom;
pub mod html;
pub mod layout;
pub mod painting;
pub mod parser;
pub mod style;

#[derive(Debug, Parser)]
#[command(
    name = "web-browser",
    version,
    about = "Render a tiny subset of HTML/CSS to a PNG image"
)]
struct Cli {
    #[arg(
        short = 'H',
        long,
        default_value = "examples/demo.html",
        value_name = "FILE"
    )]
    html: PathBuf,

    #[arg(short, long, default_value = "examples/demo.css", value_name = "FILE")]
    css: PathBuf,

    #[arg(short, long, default_value = "output.png", value_name = "FILE")]
    output: PathBuf,

    #[arg(long, default_value_t = 800)]
    width: u32,

    #[arg(long)]
    height: Option<u32>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    let html_source = fs::read_to_string(&cli.html)?;
    let css_source = fs::read_to_string(&cli.css)?;

    let root_node = html::HtmlParser::parse(html_source)?;
    let stylesheet = css::parse(css_source)?;
    let style_root = style::style_tree(&root_node, &stylesheet);

    let mut containing_block: layout::Dimensions = Default::default();
    containing_block.content.width = cli.width as f32;

    let mut layout_root = layout::build_layout_tree(&style_root);
    layout_root.layout(containing_block);

    let viewport_height = cli
        .height
        .unwrap_or_else(|| layout_root.dimensions.border_box().height.ceil().max(1.0) as u32);

    let viewport = layout::Rect {
        width: cli.width as f32,
        height: viewport_height as f32,
        ..Default::default()
    };
    let canvas = painting::paint(&layout_root, viewport);
    save_png(&canvas, &cli.output)?;

    println!(
        "Rendered {} + {} to {}",
        cli.html.display(),
        cli.css.display(),
        cli.output.display()
    );
    println!("Viewport: {}x{}", cli.width, viewport_height);
    println!("Layout tree:");
    print_layout_tree(&layout_root, 0);

    Ok(())
}

fn save_png(canvas: &painting::Canvas, output: &PathBuf) -> Result<(), image::ImageError> {
    let image = image::ImageBuffer::from_fn(canvas.width as u32, canvas.height as u32, |x, y| {
        let color = canvas.pixels[(x + y * canvas.width as u32) as usize];
        image::Rgba([color.r, color.g, color.b, color.a])
    });

    image.save(output)
}

fn print_layout_tree(layout_box: &layout::LayoutBox<'_>, depth: usize) {
    let indent = "  ".repeat(depth);
    println!(
        "{}{} content=({}) margin=({}) border=({}) padding=({})",
        indent,
        box_name(layout_box),
        format_rect(layout_box.dimensions.content),
        format_edges(layout_box.dimensions.margin),
        format_edges(layout_box.dimensions.border),
        format_edges(layout_box.dimensions.padding)
    );

    for child in &layout_box.children {
        print_layout_tree(child, depth + 1);
    }
}

fn box_name(layout_box: &layout::LayoutBox<'_>) -> String {
    match &layout_box.box_type {
        layout::BoxType::BlockNode(style_node) => {
            format!(
                "block {}{}",
                node_name(style_node),
                style_summary(style_node)
            )
        }
        layout::BoxType::InlineNode(style_node) => {
            format!(
                "inline {}{}",
                node_name(style_node),
                style_summary(style_node)
            )
        }
        layout::BoxType::AnonymousBlock => "anonymous block".to_string(),
    }
}

fn node_name(style_node: &style::StyledNode<'_>) -> String {
    match &style_node.node.node_type {
        dom::NodeType::Element(element) => {
            let id = element.id().map(|id| format!("#{id}")).unwrap_or_default();
            let classes = element
                .attrs
                .get("class")
                .map(|class| {
                    class
                        .split_whitespace()
                        .map(|name| format!(".{name}"))
                        .collect::<String>()
                })
                .unwrap_or_default();

            format!("<{}{}{}>", element.tag_name, id, classes)
        }
        dom::NodeType::Text(text) => format!("text({text:?})"),
    }
}

fn style_summary(style_node: &style::StyledNode<'_>) -> String {
    let mut values = Vec::new();

    if let Some(background) = color_property(style_node, "background") {
        values.push(format!("background={background}"));
    }

    if let Some(border_color) = color_property(style_node, "border-color") {
        values.push(format!("border-color={border_color}"));
    }

    if values.is_empty() {
        String::new()
    } else {
        format!(" {}", values.join(" "))
    }
}

fn color_property(style_node: &style::StyledNode<'_>, name: &str) -> Option<String> {
    match style_node.value(name) {
        Some(css::Value::ColorValue(color)) => {
            Some(format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b))
        }
        _ => None,
    }
}

fn format_rect(rect: layout::Rect) -> String {
    format!(
        "x={:.0}, y={:.0}, w={:.0}, h={:.0}",
        rect.x, rect.y, rect.width, rect.height
    )
}

fn format_edges(edges: layout::EdgeSizes) -> String {
    format!(
        "l={:.0}, r={:.0}, t={:.0}, b={:.0}",
        edges.left, edges.right, edges.top, edges.bottom
    )
}
