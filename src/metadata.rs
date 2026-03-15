use std::collections::BTreeMap;

use roxmltree::{Document, Node};

use crate::error::{CziError, Result};
use crate::types::{
    ChannelInfo, Dimension, DocumentInfo, ImageInfo, MetadataSummary, PixelType, ScalingInfo,
};

pub(crate) fn parse_metadata_xml(xml: &str) -> Result<MetadataSummary> {
    if xml.trim().is_empty() {
        return Ok(MetadataSummary::default());
    }

    let doc = Document::parse(xml).map_err(|err| CziError::file_metadata(err.to_string()))?;
    let root = doc.root_element();

    let information = find_path(root, &["Metadata", "Information"]);
    let image_node = information.and_then(|node| child(node, "Image"));
    let document_node = information.and_then(|node| child(node, "Document"));
    let application_node = information.and_then(|node| child(node, "Application"));
    let scaling_node = find_path(root, &["Metadata", "Scaling"]);

    let document = parse_document(document_node, application_node);
    let image = parse_image(image_node);
    let scaling = parse_scaling(scaling_node);
    let channels = parse_channels(image_node);

    Ok(MetadataSummary {
        document,
        image,
        scaling,
        channels,
    })
}

fn parse_document(
    document_node: Option<Node<'_, '_>>,
    application_node: Option<Node<'_, '_>>,
) -> DocumentInfo {
    let mut document = DocumentInfo::default();

    if let Some(node) = document_node {
        document.name = child_text(node, "Name");
        document.title = child_text(node, "Title");
        document.comment = child_text(node, "Comment");
        document.author = child_text(node, "Author");
        document.user_name = child_text(node, "UserName");
        document.creation_date = child_text(node, "CreationDate");
        document.description = child_text(node, "Description");
    }

    if let Some(node) = application_node {
        document.application_name = child_text(node, "Name");
        document.application_version = child_text(node, "Version");
    }

    document
}

fn parse_image(image_node: Option<Node<'_, '_>>) -> ImageInfo {
    let mut image = ImageInfo {
        pixel_type: None,
        sizes: BTreeMap::new(),
    };

    let Some(node) = image_node else {
        return image;
    };

    image.pixel_type = child_text(node, "PixelType").and_then(|value| PixelType::from_name(&value));

    for child in node.children().filter(|candidate| candidate.is_element()) {
        let tag = child.tag_name().name();
        if let Some(dimension_name) = tag.strip_prefix("Size") {
            if let Some(dimension) = Dimension::from_code(dimension_name) {
                if let Some(value) = child
                    .text()
                    .and_then(|text| text.trim().parse::<usize>().ok())
                {
                    image.sizes.insert(dimension, value);
                }
            }
        }
    }

    image
}

fn parse_channels(image_node: Option<Node<'_, '_>>) -> Vec<ChannelInfo> {
    let Some(node) = image_node else {
        return Vec::new();
    };

    let Some(channels_node) = find_path(node, &["Dimensions", "Channels"]) else {
        return Vec::new();
    };

    channels_node
        .children()
        .filter(|candidate| candidate.is_element() && candidate.tag_name().name() == "Channel")
        .enumerate()
        .map(|(index, channel)| ChannelInfo {
            index,
            id: channel.attribute("Id").map(|value| value.to_owned()),
            name: channel.attribute("Name").map(|value| value.to_owned()),
            pixel_type: child_text(channel, "PixelType")
                .and_then(|value| PixelType::from_name(&value)),
            color: child_text(channel, "Color"),
        })
        .collect()
}

fn parse_scaling(scaling_node: Option<Node<'_, '_>>) -> ScalingInfo {
    let Some(node) = scaling_node else {
        return ScalingInfo::default();
    };

    let items = child(node, "Items").unwrap_or(node);
    let mut scaling = ScalingInfo::default();

    for distance in items
        .children()
        .filter(|candidate| candidate.is_element() && candidate.tag_name().name() == "Distance")
    {
        let value = child_text(distance, "Value").and_then(|text| text.parse::<f64>().ok());
        let id = distance
            .attribute("Id")
            .map(|value| value.trim().to_ascii_uppercase());

        match id.as_deref() {
            Some("X") => scaling.x = value,
            Some("Y") => scaling.y = value,
            Some("Z") => scaling.z = value,
            _ => {}
        }

        if scaling.unit.is_none() {
            scaling.unit = child_text(distance, "DefaultUnitFormat")
                .or_else(|| child_text(distance, "Unit"))
                .or_else(|| child_text(distance, "UnitFormat"));
        }
    }

    scaling
}

fn find_path<'a, 'input>(start: Node<'a, 'input>, path: &[&str]) -> Option<Node<'a, 'input>> {
    let mut current = Some(start);
    for name in path {
        current = current.and_then(|node| child(node, name));
    }
    current
}

fn child<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Option<Node<'a, 'input>> {
    node.children()
        .find(|candidate| candidate.is_element() && candidate.tag_name().name() == name)
}

fn child_text(node: Node<'_, '_>, name: &str) -> Option<String> {
    child(node, name)
        .and_then(|candidate| candidate.text())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}
