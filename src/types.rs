#![allow(dead_code)]

use std::collections::{BTreeMap, HashMap};

use crate::error::{CziError, Result};

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Dimension {
    Z,
    C,
    T,
    R,
    S,
    I,
    H,
    V,
    B,
    X,
    Y,
    M,
}

impl Dimension {
    pub const FRAME_ORDER: [Dimension; 9] = [
        Dimension::S,
        Dimension::T,
        Dimension::C,
        Dimension::Z,
        Dimension::R,
        Dimension::I,
        Dimension::H,
        Dimension::V,
        Dimension::B,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Z => "Z",
            Self::C => "C",
            Self::T => "T",
            Self::R => "R",
            Self::S => "S",
            Self::I => "I",
            Self::H => "H",
            Self::V => "V",
            Self::B => "B",
            Self::X => "X",
            Self::Y => "Y",
            Self::M => "M",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code.trim().to_ascii_uppercase().as_str() {
            "Z" => Some(Self::Z),
            "C" => Some(Self::C),
            "T" => Some(Self::T),
            "R" => Some(Self::R),
            "S" => Some(Self::S),
            "I" => Some(Self::I),
            "H" => Some(Self::H),
            "V" => Some(Self::V),
            "B" => Some(Self::B),
            "X" => Some(Self::X),
            "Y" => Some(Self::Y),
            "M" => Some(Self::M),
            _ => None,
        }
    }

    pub fn is_frame_dimension(self) -> bool {
        matches!(
            self,
            Self::Z | Self::C | Self::T | Self::R | Self::S | Self::I | Self::H | Self::V | Self::B
        )
    }
}

impl std::fmt::Display for Dimension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Interval {
    pub start: i32,
    pub size: usize,
}

impl Interval {
    pub fn end_exclusive(self) -> i32 {
        self.start + self.size as i32
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DimBounds {
    intervals: BTreeMap<Dimension, Interval>,
}

impl DimBounds {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, dimension: Dimension) -> Option<Interval> {
        self.intervals.get(&dimension).copied()
    }

    pub fn set(&mut self, dimension: Dimension, interval: Interval) {
        self.intervals.insert(dimension, interval);
    }

    pub fn update_value(&mut self, dimension: Dimension, value: i32) {
        match self.intervals.get_mut(&dimension) {
            Some(existing) => {
                let current_end = existing.end_exclusive();
                if value < existing.start {
                    existing.size = (current_end - value) as usize;
                    existing.start = value;
                } else if value >= current_end {
                    existing.size = (value - existing.start + 1) as usize;
                }
            }
            None => {
                self.set(
                    dimension,
                    Interval {
                        start: value,
                        size: 1,
                    },
                );
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (Dimension, Interval)> + '_ {
        self.intervals
            .iter()
            .map(|(dimension, interval)| (*dimension, *interval))
    }

    pub fn to_size_map(&self) -> HashMap<String, usize> {
        self.iter()
            .map(|(dimension, interval)| (dimension.as_str().to_owned(), interval.size))
            .collect()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Coordinate {
    values: BTreeMap<Dimension, i32>,
}

impl Coordinate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, dimension: Dimension, value: i32) -> Self {
        self.set(dimension, value);
        self
    }

    pub fn set(&mut self, dimension: Dimension, value: i32) {
        self.values.insert(dimension, value);
    }

    pub fn get(&self, dimension: Dimension) -> Option<i32> {
        self.values.get(&dimension).copied()
    }

    pub fn contains(&self, dimension: Dimension) -> bool {
        self.values.contains_key(&dimension)
    }

    pub fn iter(&self) -> impl Iterator<Item = (Dimension, i32)> + '_ {
        self.values
            .iter()
            .map(|(dimension, value)| (*dimension, *value))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlaneIndex {
    values: BTreeMap<Dimension, usize>,
}

impl PlaneIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, dimension: Dimension, value: usize) -> Self {
        self.set(dimension, value);
        self
    }

    pub fn set(&mut self, dimension: Dimension, value: usize) {
        self.values.insert(dimension, value);
    }

    pub fn get(&self, dimension: Dimension) -> Option<usize> {
        self.values.get(&dimension).copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = (Dimension, usize)> + '_ {
        self.values
            .iter()
            .map(|(dimension, value)| (*dimension, *value))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct IntRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl IntRect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub fn is_valid(self) -> bool {
        self.w >= 0 && self.h >= 0
    }

    pub fn is_non_empty(self) -> bool {
        self.w > 0 && self.h > 0
    }

    pub fn right(self) -> i32 {
        self.x + self.w
    }

    pub fn bottom(self) -> i32 {
        self.y + self.h
    }

    pub fn intersect(self, other: Self) -> Option<Self> {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = self.right().min(other.right());
        let y2 = self.bottom().min(other.bottom());
        if x2 <= x1 || y2 <= y1 {
            return None;
        }
        Some(Self::new(x1, y1, x2 - x1, y2 - y1))
    }

    pub(crate) fn union_with(&mut self, other: Self) {
        if !self.is_valid() {
            *self = other;
            return;
        }

        let x1 = self.x.min(other.x);
        let y1 = self.y.min(other.y);
        let x2 = self.right().max(other.right());
        let y2 = self.bottom().max(other.bottom());
        *self = Self::new(x1, y1, x2 - x1, y2 - y1);
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct IntSize {
    pub w: u32,
    pub h: u32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PixelType {
    Gray8,
    Gray16,
    Gray32Float,
    Bgr24,
    Bgr48,
    Bgr96Float,
    Bgra32,
    Gray64ComplexFloat,
    Bgr192ComplexFloat,
    Gray32,
    Gray64Float,
}

impl PixelType {
    pub fn from_raw(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Gray8),
            1 => Some(Self::Gray16),
            2 => Some(Self::Gray32Float),
            3 => Some(Self::Bgr24),
            4 => Some(Self::Bgr48),
            8 => Some(Self::Bgr96Float),
            9 => Some(Self::Bgra32),
            10 => Some(Self::Gray64ComplexFloat),
            11 => Some(Self::Bgr192ComplexFloat),
            12 => Some(Self::Gray32),
            13 => Some(Self::Gray64Float),
            _ => None,
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim() {
            "Gray8" => Some(Self::Gray8),
            "Gray16" => Some(Self::Gray16),
            "Gray32Float" => Some(Self::Gray32Float),
            "Bgr24" => Some(Self::Bgr24),
            "Bgr48" => Some(Self::Bgr48),
            "Bgr96Float" => Some(Self::Bgr96Float),
            "Bgra32" => Some(Self::Bgra32),
            "Gray64ComplexFloat" => Some(Self::Gray64ComplexFloat),
            "Bgr192ComplexFloat" => Some(Self::Bgr192ComplexFloat),
            "Gray32" => Some(Self::Gray32),
            "Gray64Float" => Some(Self::Gray64Float),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Gray8 => "Gray8",
            Self::Gray16 => "Gray16",
            Self::Gray32Float => "Gray32Float",
            Self::Bgr24 => "Bgr24",
            Self::Bgr48 => "Bgr48",
            Self::Bgr96Float => "Bgr96Float",
            Self::Bgra32 => "Bgra32",
            Self::Gray64ComplexFloat => "Gray64ComplexFloat",
            Self::Bgr192ComplexFloat => "Bgr192ComplexFloat",
            Self::Gray32 => "Gray32",
            Self::Gray64Float => "Gray64Float",
        }
    }

    pub fn bytes_per_pixel(self) -> usize {
        match self {
            Self::Gray8 => 1,
            Self::Gray16 => 2,
            Self::Gray32Float => 4,
            Self::Bgr24 => 3,
            Self::Bgr48 => 6,
            Self::Bgr96Float => 12,
            Self::Bgra32 => 4,
            Self::Gray64ComplexFloat => 16,
            Self::Bgr192ComplexFloat => 24,
            Self::Gray32 => 4,
            Self::Gray64Float => 8,
        }
    }
}

impl std::fmt::Display for PixelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CompressionMode {
    UnCompressed,
    Jpg,
    JpgXr,
    Zstd0,
    Zstd1,
}

impl CompressionMode {
    pub fn from_raw(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::UnCompressed),
            1 => Some(Self::Jpg),
            4 => Some(Self::JpgXr),
            5 => Some(Self::Zstd0),
            6 => Some(Self::Zstd1),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnCompressed => "UnCompressed",
            Self::Jpg => "Jpg",
            Self::JpgXr => "JpgXr",
            Self::Zstd0 => "Zstd0",
            Self::Zstd1 => "Zstd1",
        }
    }
}

impl std::fmt::Display for CompressionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SubBlockPyramidType {
    None,
    SingleSubBlock,
    MultiSubBlock,
}

impl SubBlockPyramidType {
    pub fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::SingleSubBlock),
            2 => Some(Self::MultiSubBlock),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileHeaderInfo {
    pub major: i32,
    pub minor: i32,
    pub primary_file_guid: String,
    pub file_guid: String,
    pub file_part: i32,
    pub subblock_directory_position: u64,
    pub metadata_position: u64,
    pub attachment_directory_position: u64,
    pub update_pending: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectorySubBlockInfo {
    pub index: usize,
    pub file_position: u64,
    pub file_part: i32,
    pub pixel_type: PixelType,
    pub compression: CompressionMode,
    pub coordinate: Coordinate,
    pub rect: IntRect,
    pub stored_size: IntSize,
    pub m_index: Option<i32>,
    pub pyramid_type: Option<SubBlockPyramidType>,
}

impl DirectorySubBlockInfo {
    pub fn is_layer0(&self) -> bool {
        self.rect.w >= 0
            && self.rect.h >= 0
            && self.stored_size.w == self.rect.w as u32
            && self.stored_size.h == self.rect.h as u32
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttachmentInfo {
    pub index: usize,
    pub file_position: u64,
    pub file_part: i32,
    pub content_guid: String,
    pub content_file_type: String,
    pub name: String,
    pub data_size: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttachmentBlob {
    pub info: AttachmentInfo,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawSubBlock {
    pub info: DirectorySubBlockInfo,
    pub metadata: Vec<u8>,
    pub data: Vec<u8>,
    pub attachment: Vec<u8>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BoundingBoxes {
    pub all: IntRect,
    pub layer0: IntRect,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubBlockStatistics {
    pub subblock_count: usize,
    pub dim_bounds: DimBounds,
    pub bounding_box: Option<IntRect>,
    pub bounding_box_layer0: Option<IntRect>,
    pub scene_bounding_boxes: BTreeMap<i32, BoundingBoxes>,
    pub min_m_index: Option<i32>,
    pub max_m_index: Option<i32>,
}

impl Default for SubBlockStatistics {
    fn default() -> Self {
        Self {
            subblock_count: 0,
            dim_bounds: DimBounds::new(),
            bounding_box: None,
            bounding_box_layer0: None,
            scene_bounding_boxes: BTreeMap::new(),
            min_m_index: None,
            max_m_index: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScalingInfo {
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub z: Option<f64>,
    pub unit: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ChannelInfo {
    pub index: usize,
    pub id: Option<String>,
    pub name: Option<String>,
    pub pixel_type: Option<PixelType>,
    pub color: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocumentInfo {
    pub name: Option<String>,
    pub title: Option<String>,
    pub comment: Option<String>,
    pub author: Option<String>,
    pub user_name: Option<String>,
    pub creation_date: Option<String>,
    pub description: Option<String>,
    pub application_name: Option<String>,
    pub application_version: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImageInfo {
    pub pixel_type: Option<PixelType>,
    pub sizes: BTreeMap<Dimension, usize>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MetadataSummary {
    pub document: DocumentInfo,
    pub image: ImageInfo,
    pub scaling: ScalingInfo,
    pub channels: Vec<ChannelInfo>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SummaryChannel {
    pub index: usize,
    pub name: Option<String>,
    pub color: Option<String>,
    pub pixel_type: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SummaryScaling {
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub z: Option<f64>,
    pub unit: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DatasetSummary {
    pub version_major: u32,
    pub version_minor: u32,
    pub sizes: BTreeMap<String, usize>,
    pub logical_frame_count: usize,
    pub channels: Vec<SummaryChannel>,
    pub pixel_type: Option<String>,
    pub scaling: Option<SummaryScaling>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bitmap {
    pub pixel_type: PixelType,
    pub width: u32,
    pub height: u32,
    pub stride: usize,
    pub data: Vec<u8>,
}

impl Bitmap {
    pub fn new(pixel_type: PixelType, width: u32, height: u32, data: Vec<u8>) -> Result<Self> {
        let stride = width as usize * pixel_type.bytes_per_pixel();
        let expected = stride
            .checked_mul(height as usize)
            .ok_or_else(|| CziError::internal_overflow("bitmap size"))?;
        if data.len() != expected {
            return Err(CziError::file_invalid_format(format!(
                "bitmap byte count mismatch: expected {expected}, got {}",
                data.len()
            )));
        }

        Ok(Self {
            pixel_type,
            width,
            height,
            stride,
            data,
        })
    }

    pub fn zeros(pixel_type: PixelType, width: u32, height: u32) -> Result<Self> {
        let stride = width as usize * pixel_type.bytes_per_pixel();
        let len = stride
            .checked_mul(height as usize)
            .ok_or_else(|| CziError::internal_overflow("bitmap allocation"))?;
        Ok(Self {
            pixel_type,
            width,
            height,
            stride,
            data: vec![0; len],
        })
    }

    pub fn bytes_per_pixel(&self) -> usize {
        self.pixel_type.bytes_per_pixel()
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.data
    }

    pub fn to_u16_vec(&self) -> Result<Vec<u16>> {
        if self.bytes_per_pixel() % 2 != 0 {
            return Err(CziError::unsupported_pixel_type(self.pixel_type.as_str()));
        }

        let mut values = Vec::with_capacity(self.data.len() / 2);
        for chunk in self.data.chunks_exact(2) {
            values.push(u16::from_le_bytes([chunk[0], chunk[1]]));
        }
        Ok(values)
    }

    pub fn into_gray_u16(self) -> Result<Vec<u16>> {
        let expected_len = self.width as usize * self.height as usize;
        match self.pixel_type {
            PixelType::Gray8 => Ok(self.data.into_iter().map(u16::from).collect()),
            PixelType::Bgr24 | PixelType::Bgra32 => {
                let channels = self.pixel_type.bytes_per_pixel();
                let mut collapsed = Vec::with_capacity(expected_len);
                for chunk in self.data.chunks_exact(channels) {
                    let sum: u32 = chunk.iter().map(|value| u32::from(*value)).sum();
                    collapsed.push((sum / channels as u32) as u16);
                }
                Ok(collapsed)
            }
            PixelType::Gray16 => self.to_u16_vec(),
            PixelType::Bgr48 => {
                let values = self.to_u16_vec()?;
                let channels = 3usize;
                let mut collapsed = Vec::with_capacity(expected_len);
                for chunk in values.chunks_exact(channels) {
                    let sum: u32 = chunk.iter().map(|value| u32::from(*value)).sum();
                    collapsed.push((sum / channels as u32) as u16);
                }
                Ok(collapsed)
            }
            _ => Err(CziError::unsupported_pixel_type(self.pixel_type.as_str())),
        }
    }

    pub fn to_f32_vec(&self) -> Result<Vec<f32>> {
        if self.bytes_per_pixel() % 4 != 0 {
            return Err(CziError::unsupported_pixel_type(self.pixel_type.as_str()));
        }

        let mut values = Vec::with_capacity(self.data.len() / 4);
        for chunk in self.data.chunks_exact(4) {
            values.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
        }
        Ok(values)
    }
}
