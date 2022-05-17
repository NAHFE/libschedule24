use svg::node::element::{Rectangle, Text as TextElement, Line};
use svg::node::Text as TextNode;

use std::str::FromStr;
use std::num::ParseIntError;
use std::fmt;

use crate::{Dimensions, data::*};

#[derive(Debug, PartialEq)]
struct Rgb {
    r: u8,
    g: u8,
    b: u8,
}

impl FromStr for Rgb {
    type Err = ParseIntError;

    // Parses a color hex code of the form '#rRgGbB..' into an
    // instance of 'Rgb'
    fn from_str(hex_code: &str) -> Result<Self, Self::Err> {
        // u8::from_str_radix(src: &str, radix: u32) converts a string
        // slice in a given base to u8
        let r: u8 = u8::from_str_radix(&hex_code[1..3], 16)?;
        let g: u8 = u8::from_str_radix(&hex_code[3..5], 16)?;
        let b: u8 = u8::from_str_radix(&hex_code[5..7], 16)?;

        Ok(Rgb { r, g, b })
    }
}

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "rgb({}, {}, {})", self.r, self.g, self.b)
    }
}

fn rect_style(rect: &Box) -> Result<String, ParseIntError> {
    // bg = fill, fg = stroke

    let mut cursor_pointer = false;
    let mut stroke_width = 1;
    match rect.type_field.as_str() {
        "Footer"|"ClockFrameStart"|"ClockFrameEnd" => {
            stroke_width = 0;
        },
        "Lesson" => {
            cursor_pointer = true;
        },
        _ => {},
    }

    let cursor_string = if cursor_pointer {
        " cursor: pointer;"
    } else {
        ""
    };

    let fg = Rgb::from_str(&rect.f_color)?;
    let bg = Rgb::from_str(&rect.b_color)?;
    Ok(format!("fill: {}; stroke: {}; stroke-width: {};{}", bg, fg, stroke_width, cursor_string))
}

fn text_style(txt: &Text) -> Result<String, ParseIntError> {
    let color = Rgb::from_str(&txt.f_color)?;
    Ok(format!("fill: {}; font-size: {}px; font-family: Open Sans; pointer-events: none;", color, txt.fontsize))
}

pub fn generate_svg(schema_data: &Schema, dimensions: Dimensions) -> Result<svg::Document, std::num::ParseIntError> {
    let mut doc = svg::Document::new()
        .set("width", dimensions.width)
        .set("height", dimensions.height)
        .set("shape-rendering", "crispEdges")
        .set("viewBox", (0, 0, dimensions.width, dimensions.height));

    for rect in &schema_data.box_list {
        let style = rect_style(rect)?;
        let mut elem = Rectangle::new()
            .set("x", rect.x)
            .set("y", rect.y)
            .set("width", rect.width)
            .set("height", rect.height)
            .set("box-id", rect.id)
            .set("shape-rendering", "crispEdges")
            .set("box-type", &rect.type_field[..])
            .set("style", &style[..]);
        if rect.type_field == "Lesson" {
            elem = elem
                .set("focusable", true)
                .set("tabindex", 0);
        }
        doc = doc.add(elem)
    }

    for txt in &schema_data.text_list {
        let style = text_style(txt)?;
        let text_node = TextNode::new(&txt.text[..]);
        let x_coord = match txt.type_field.as_str() {
            "ClockAxisBox"|"HeadingDay" => {
                let mut x = txt.x;
                for rect in &schema_data.box_list {
                    if rect.id == txt.parent_id {
                        // This is not perfect because it does not take letter spacing into account, but it is good enough.
                        x = rect.x + (rect.width/2) - (txt.text.len() as i64 * txt.fontsize as i64)/4;
                        break
                    }
                }

                x
            }
            _ => txt.x
        };
        if txt.italic || txt.bold {
            // These do not seem to be used at all.
            eprintln!("Unimplemented: italic|bold");
        }

        doc = doc.add(
            TextElement::new()
                .set("x", x_coord)
                .set("y", txt.y + txt.fontsize as i64)
                .set("text-id", txt.id)
                .set("style", &style[..])
                .add(text_node)
        )
    }

    for line in &schema_data.line_list {
        doc = doc.add(
            Line::new()
                .set("x1", line.p1x)
                .set("y1", line.p1y)
                .set("x2", line.p2x)
                .set("y2", line.p2y)
                .set("stroke", &line.color[..])
        )
    }

    Ok(doc)
}
