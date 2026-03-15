use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use crate::error::{CziError, Result};
use crate::metadata::parse_metadata_xml;
use crate::parse::{
    decode_subblock_bitmap, parse_file, read_attachment_blob, read_metadata_xml, read_raw_subblock,
};
use crate::types::{
    AttachmentBlob, AttachmentInfo, Bitmap, Coordinate, Dimension, DirectorySubBlockInfo,
    FileHeaderInfo, MetadataSummary, PlaneIndex, RawSubBlock, SubBlockStatistics,
};

pub struct CziFile {
    path: PathBuf,
    reader: BufReader<File>,
    header: FileHeaderInfo,
    subblocks: Vec<DirectorySubBlockInfo>,
    attachments: Vec<AttachmentInfo>,
    statistics: SubBlockStatistics,
    metadata_xml: Option<String>,
    metadata: Option<MetadataSummary>,
}

impl CziFile {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path)?;
        let mut reader = BufReader::new(file);
        let parsed = parse_file(&mut reader)?;

        Ok(Self {
            path,
            reader,
            header: parsed.header,
            subblocks: parsed.subblocks,
            attachments: parsed.attachments,
            statistics: parsed.statistics,
            metadata_xml: None,
            metadata: None,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn version(&self) -> (i32, i32) {
        (self.header.major, self.header.minor)
    }

    pub fn file_header(&self) -> &FileHeaderInfo {
        &self.header
    }

    pub fn statistics(&self) -> &SubBlockStatistics {
        &self.statistics
    }

    pub fn subblocks(&self) -> &[DirectorySubBlockInfo] {
        &self.subblocks
    }

    pub fn attachments(&self) -> &[AttachmentInfo] {
        &self.attachments
    }

    pub fn metadata_xml(&mut self) -> Result<&str> {
        if self.metadata_xml.is_none() {
            let xml = read_metadata_xml(&mut self.reader, self.header.metadata_position)?;
            self.metadata_xml = Some(xml);
        }

        Ok(self.metadata_xml.as_deref().unwrap_or_default())
    }

    pub fn metadata(&mut self) -> Result<&MetadataSummary> {
        if self.metadata.is_none() {
            if self.metadata_xml.is_none() {
                let xml = read_metadata_xml(&mut self.reader, self.header.metadata_position)?;
                self.metadata_xml = Some(xml);
            }
            let parsed = {
                let xml = self.metadata_xml.as_deref().unwrap_or_default();
                parse_metadata_xml(xml)?
            };
            self.metadata = Some(parsed);
        }

        Ok(self.metadata.as_ref().unwrap())
    }

    pub fn sizes(&self) -> Result<HashMap<String, usize>> {
        let mut sizes = HashMap::new();
        for dimension in Dimension::FRAME_ORDER {
            sizes.insert(
                dimension.as_str().to_owned(),
                self.statistics
                    .dim_bounds
                    .get(dimension)
                    .map(|interval| interval.size)
                    .unwrap_or(1),
            );
        }

        let rect = self
            .statistics
            .bounding_box_layer0
            .or(self.statistics.bounding_box);
        sizes.insert(
            Dimension::X.as_str().to_owned(),
            rect.map(|value| value.w.max(0) as usize).unwrap_or(0),
        );
        sizes.insert(
            Dimension::Y.as_str().to_owned(),
            rect.map(|value| value.h.max(0) as usize).unwrap_or(0),
        );

        Ok(sizes)
    }

    pub fn loop_indices(&self) -> Result<Vec<HashMap<String, usize>>> {
        let mut varying_dims = Vec::new();
        for dimension in Dimension::FRAME_ORDER {
            let size = self
                .statistics
                .dim_bounds
                .get(dimension)
                .map(|interval| interval.size)
                .unwrap_or(1);
            if size > 1 {
                varying_dims.push((dimension, size));
            }
        }

        if varying_dims.is_empty() {
            return Ok(vec![HashMap::new()]);
        }

        let total = varying_dims.iter().map(|(_, size)| *size).product();
        let mut out = Vec::with_capacity(total);
        let mut current = HashMap::new();
        build_loop_indices(&varying_dims, 0, &mut current, &mut out);
        Ok(out)
    }

    pub fn channel_pixel_types(&self) -> HashMap<usize, crate::types::PixelType> {
        let channel_start = self
            .statistics
            .dim_bounds
            .get(Dimension::C)
            .map(|interval| interval.start)
            .unwrap_or(0);

        let mut pixel_types = HashMap::new();
        for subblock in &self.subblocks {
            let actual_channel = subblock
                .coordinate
                .get(Dimension::C)
                .unwrap_or(channel_start);
            let relative_channel = actual_channel.saturating_sub(channel_start) as usize;
            pixel_types
                .entry(relative_channel)
                .or_insert(subblock.pixel_type);
        }
        pixel_types
    }

    pub fn read_frame(&mut self, index: usize) -> Result<Bitmap> {
        let indices = self.loop_indices()?;
        if index >= indices.len() {
            return Err(CziError::input_out_of_range(
                "frame index",
                index,
                indices.len(),
            ));
        }

        let mut plane = PlaneIndex::new();
        for (name, value) in &indices[index] {
            let dimension = Dimension::from_code(name).ok_or_else(|| {
                CziError::input_argument("frame index", format!("unknown dimension '{name}'"))
            })?;
            plane.set(dimension, *value);
        }
        self.read_plane(&plane)
    }

    pub fn read_frame_2d(&mut self, s: usize, t: usize, c: usize, z: usize) -> Result<Bitmap> {
        let plane = PlaneIndex::new()
            .with(Dimension::S, s)
            .with(Dimension::T, t)
            .with(Dimension::C, c)
            .with(Dimension::Z, z);
        self.read_plane(&plane)
    }

    pub fn read_plane(&mut self, index: &PlaneIndex) -> Result<Bitmap> {
        let actual = self.resolve_plane_index(index)?;
        let plane_rect = self
            .select_plane_rect(actual.get(Dimension::S))
            .ok_or_else(|| CziError::file_invalid_format("no plane bounding box available"))?;
        if plane_rect.w <= 0 || plane_rect.h <= 0 {
            return Err(CziError::file_invalid_format(
                "plane bounding box has non-positive size",
            ));
        }

        let mut matching: Vec<&DirectorySubBlockInfo> = self
            .subblocks
            .iter()
            .filter(|subblock| self.matches_plane(subblock, &actual))
            .collect();
        if matching.is_empty() {
            return Err(CziError::file_invalid_format(
                "no layer-0 subblocks matched the requested plane",
            ));
        }

        matching
            .sort_by_key(|subblock| (subblock.m_index.unwrap_or(i32::MIN), subblock.file_position));

        let pixel_type = matching[0].pixel_type;
        if matching
            .iter()
            .any(|subblock| subblock.pixel_type != pixel_type)
        {
            return Err(CziError::file_invalid_format(
                "requested plane contains mixed pixel types",
            ));
        }

        let mut bitmap = Bitmap::zeros(pixel_type, plane_rect.w as u32, plane_rect.h as u32)?;
        for subblock in matching {
            let raw = read_raw_subblock(&mut self.reader, subblock)?;
            let tile = decode_subblock_bitmap(&raw)?;
            blit_tile(
                &mut bitmap,
                &tile,
                subblock.rect.x - plane_rect.x,
                subblock.rect.y - plane_rect.y,
            )?;
        }

        Ok(bitmap)
    }

    pub fn read_subblock(&mut self, index: usize) -> Result<RawSubBlock> {
        let subblock = self.subblocks.get(index).ok_or_else(|| {
            CziError::input_out_of_range("subblock index", index, self.subblocks.len())
        })?;
        read_raw_subblock(&mut self.reader, subblock)
    }

    pub fn read_attachment(&mut self, index: usize) -> Result<AttachmentBlob> {
        let attachment = self.attachments.get(index).ok_or_else(|| {
            CziError::input_out_of_range("attachment index", index, self.attachments.len())
        })?;
        read_attachment_blob(&mut self.reader, attachment)
    }

    fn resolve_plane_index(&self, index: &PlaneIndex) -> Result<Coordinate> {
        let mut actual = Coordinate::new();

        for dimension in Dimension::FRAME_ORDER {
            let requested = index.get(dimension);
            match self.statistics.dim_bounds.get(dimension) {
                Some(interval) => {
                    let relative = match requested {
                        Some(value) => value,
                        None if interval.size <= 1 => 0,
                        None => return Err(CziError::input_missing_dim(dimension.as_str())),
                    };
                    if relative >= interval.size {
                        return Err(CziError::input_out_of_range(
                            format!("dimension {}", dimension.as_str()),
                            relative,
                            interval.size,
                        ));
                    }
                    actual.set(dimension, interval.start + relative as i32);
                }
                None => {
                    if requested.unwrap_or(0) != 0 {
                        return Err(CziError::input_argument(
                            dimension.as_str(),
                            "dimension is not present in this file",
                        ));
                    }
                }
            }
        }

        Ok(actual)
    }

    fn select_plane_rect(&self, scene: Option<i32>) -> Option<crate::types::IntRect> {
        if let Some(scene) = scene {
            if let Some(bounding_boxes) = self.statistics.scene_bounding_boxes.get(&scene) {
                if bounding_boxes.layer0.is_valid() {
                    return Some(bounding_boxes.layer0);
                }
                if bounding_boxes.all.is_valid() {
                    return Some(bounding_boxes.all);
                }
            }
        }

        self.statistics
            .bounding_box_layer0
            .or(self.statistics.bounding_box)
    }

    fn matches_plane(&self, subblock: &DirectorySubBlockInfo, actual: &Coordinate) -> bool {
        if !subblock.is_layer0() {
            return false;
        }

        for dimension in Dimension::FRAME_ORDER {
            let Some(requested_value) = actual.get(dimension) else {
                continue;
            };

            match subblock.coordinate.get(dimension) {
                Some(value) if value == requested_value => {}
                Some(_) => return false,
                None => {
                    if self
                        .statistics
                        .dim_bounds
                        .get(dimension)
                        .map(|interval| interval.size > 1)
                        .unwrap_or(false)
                    {
                        return false;
                    }
                }
            }
        }

        true
    }
}

fn build_loop_indices(
    dims: &[(Dimension, usize)],
    depth: usize,
    current: &mut HashMap<String, usize>,
    out: &mut Vec<HashMap<String, usize>>,
) {
    if depth == dims.len() {
        out.push(current.clone());
        return;
    }

    let (dimension, size) = dims[depth];
    for value in 0..size {
        current.insert(dimension.as_str().to_owned(), value);
        build_loop_indices(dims, depth + 1, current, out);
    }
    current.remove(dimension.as_str());
}

fn blit_tile(
    destination: &mut Bitmap,
    source: &Bitmap,
    offset_x: i32,
    offset_y: i32,
) -> Result<()> {
    if destination.pixel_type != source.pixel_type {
        return Err(CziError::file_invalid_format(
            "cannot compose tiles with different pixel types",
        ));
    }

    let source_rect = crate::types::IntRect::new(
        offset_x,
        offset_y,
        source.width as i32,
        source.height as i32,
    );
    let destination_rect =
        crate::types::IntRect::new(0, 0, destination.width as i32, destination.height as i32);
    let Some(intersection) = source_rect.intersect(destination_rect) else {
        return Ok(());
    };

    let bytes_per_pixel = destination.pixel_type.bytes_per_pixel();
    for row in 0..intersection.h as usize {
        let src_x = (intersection.x - offset_x) as usize;
        let src_y = (intersection.y - offset_y) as usize + row;
        let dst_x = intersection.x as usize;
        let dst_y = intersection.y as usize + row;
        let row_bytes = intersection.w as usize * bytes_per_pixel;

        let src_offset = src_y
            .checked_mul(source.stride)
            .and_then(|value| value.checked_add(src_x * bytes_per_pixel))
            .ok_or_else(|| CziError::internal_overflow("source tile offset"))?;
        let dst_offset = dst_y
            .checked_mul(destination.stride)
            .and_then(|value| value.checked_add(dst_x * bytes_per_pixel))
            .ok_or_else(|| CziError::internal_overflow("destination tile offset"))?;

        destination.data[dst_offset..dst_offset + row_bytes]
            .copy_from_slice(&source.data[src_offset..src_offset + row_bytes]);
    }

    Ok(())
}
