use crate::BevyUiXmlError;
use roxmltree::Document;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct UiDocument {
    pub root: ElementNode,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ElementNode {
    pub tag: String,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub text: String,
    pub attributes: HashMap<String, String>,
    pub children: Vec<ElementNode>,
}

impl ElementNode {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attributes.get(name).map(String::as_str)
    }

    pub fn widget_type(&self) -> &str {
        if self.tag == "input" {
            if self
                .attr("type")
                .is_some_and(|value| value.eq_ignore_ascii_case("checkbox"))
            {
                return "checkbox";
            }
            if self
                .attr("type")
                .is_some_and(|value| value.eq_ignore_ascii_case("radio"))
            {
                return "radio";
            }
        }
        canonical_tag(&self.tag)
    }
}

fn canonical_tag(tag: &str) -> &str {
    match tag {
        "ui" | "panel" | "div" | "container" => "panel",
        "button" | "btn" => "button",
        "text" | "label" | "span" | "p" => "text",
        "image" | "img" => "image",
        "form" => "form",
        "input" => "input",
        "textarea" => "textarea",
        "select" => "select",
        "option" => "option",
        "checkbox" => "checkbox",
        "radio" => "radio",
        _ => tag,
    }
}

pub fn parse_layout(input: &str) -> Result<UiDocument, BevyUiXmlError> {
    let xml = Document::parse(input)?;
    let root = xml
        .root()
        .children()
        .find(|node| node.is_element())
        .ok_or(BevyUiXmlError::EmptyLayout)?;

    Ok(UiDocument {
        root: parse_element(root),
    })
}

fn parse_element(node: roxmltree::Node<'_, '_>) -> ElementNode {
    let mut attributes = HashMap::new();
    let mut id = None;
    let mut classes = Vec::new();

    for attr in node.attributes() {
        match attr.name() {
            "id" => id = Some(attr.value().to_string()),
            "class" => {
                classes = attr
                    .value()
                    .split_whitespace()
                    .map(ToOwned::to_owned)
                    .collect();
            }
            name => {
                attributes.insert(name.to_string(), attr.value().to_string());
            }
        }
    }

    let children = node
        .children()
        .filter(|child| child.is_element())
        .map(parse_element)
        .collect();

    let text = node
        .children()
        .filter(|child| child.is_text())
        .filter_map(|child| child.text())
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    ElementNode {
        tag: node.tag_name().name().to_ascii_lowercase(),
        id,
        classes,
        text,
        attributes,
        children,
    }
}
