use std::cmp::max;
use std::io::{Cursor, Read, Seek, SeekFrom};

use crate::error::{CziError, Result};
use crate::types::{
    AttachmentBlob, AttachmentInfo, Bitmap, BoundingBoxes, CompressionMode, Coordinate, Dimension,
    DirectorySubBlockInfo, FileHeaderInfo, IntRect, IntSize, PixelType, RawSubBlock,
    SubBlockPyramidType, SubBlockStatistics,
};

const SEGMENT_HEADER_SIZE: usize = 32;
const FILE_HEADER_DATA_SIZE: usize = 512;
const SUBBLOCK_DIRECTORY_DATA_SIZE: usize = 128;
const METADATA_DATA_SIZE: usize = 256;
const ATTACHMENT_DATA_SIZE: usize = 256;
const SUBBLOCK_MIN_DATA_SIZE: usize = 256;
const DIMENSION_ENTRY_DV_SIZE: usize = 20;
const SUBBLOCK_DIRECTORY_ENTRY_DV_FIXED_SIZE: usize = 32;
const SUBBLOCK_SEGMENT_FIXED_SIZE: usize = 16;

const FILE_MAGIC: &[u8; 16] = b"ZISRAWFILE\0\0\0\0\0\0";
const SUBBLOCK_DIRECTORY_MAGIC: &[u8; 16] = b"ZISRAWDIRECTORY\0";
const SUBBLOCK_MAGIC: &[u8; 16] = b"ZISRAWSUBBLOCK\0\0";
const METADATA_MAGIC: &[u8; 16] = b"ZISRAWMETADATA\0\0";
const ATTACHMENT_DIRECTORY_MAGIC: &[u8; 16] = b"ZISRAWATTDIR\0\0\0\0";
const ATTACHMENT_MAGIC: &[u8; 16] = b"ZISRAWATTACH\0\0\0\0";

#[derive(Clone, Debug)]
pub(crate) struct ParsedCziFile {
    pub header: FileHeaderInfo,
    pub subblocks: Vec<DirectorySubBlockInfo>,
    pub attachments: Vec<AttachmentInfo>,
    pub statistics: SubBlockStatistics,
}

#[derive(Copy, Clone, Debug)]
struct SegmentHeader {
    magic: [u8; 16],
    allocated_size: u64,
    used_size: u64,
}

impl SegmentHeader {
    fn read<R: Read + Seek>(reader: &mut R, offset: u64) -> Result<Self> {
        let bytes = read_exact_at(reader, offset, SEGMENT_HEADER_SIZE)?;
        let mut magic = [0u8; 16];
        magic.copy_from_slice(&bytes[..16]);
        Ok(Self {
            magic,
            allocated_size: le_u64(&bytes, 16)?,
            used_size: le_u64(&bytes, 24)?,
        })
    }

    fn effective_size(self) -> u64 {
        if self.used_size != 0 {
            self.used_size
        } else {
            self.allocated_size
        }
    }
}

pub(crate) fn parse_file<R: Read + Seek>(reader: &mut R) -> Result<ParsedCziFile> {
    let header = parse_file_header(reader)?;
    let (subblocks, statistics) =
        parse_subblock_directory(reader, header.subblock_directory_position)?;
    let attachments = if header.attachment_directory_position == 0 {
        Vec::new()
    } else {
        parse_attachment_directory(reader, header.attachment_directory_position)?
    };

    Ok(ParsedCziFile {
        header,
        subblocks,
        attachments,
        statistics,
    })
}

pub(crate) fn read_metadata_xml<R: Read + Seek>(reader: &mut R, offset: u64) -> Result<String> {
    if offset == 0 {
        return Ok(String::new());
    }

    let header = SegmentHeader::read(reader, offset)?;
    ensure_magic(offset, &header.magic, METADATA_MAGIC)?;

    let fixed = read_exact_at(
        reader,
        checked_add(offset, SEGMENT_HEADER_SIZE as u64)?,
        METADATA_DATA_SIZE,
    )?;
    let xml_size = le_u32(&fixed, 0)? as usize;
    if xml_size == 0 {
        return Ok(String::new());
    }

    let xml_bytes = read_exact_at(
        reader,
        checked_add(offset, (SEGMENT_HEADER_SIZE + METADATA_DATA_SIZE) as u64)?,
        xml_size,
    )?;
    String::from_utf8(xml_bytes).map_err(|err| CziError::file_invalid_utf8(err.to_string()))
}

#[allow(dead_code)]
pub(crate) fn read_attachment_blob<R: Read + Seek>(
    reader: &mut R,
    info: &AttachmentInfo,
) -> Result<AttachmentBlob> {
    let header = SegmentHeader::read(reader, info.file_position)?;
    ensure_magic(info.file_position, &header.magic, ATTACHMENT_MAGIC)?;

    let fixed = read_exact_at(
        reader,
        checked_add(info.file_position, SEGMENT_HEADER_SIZE as u64)?,
        ATTACHMENT_DATA_SIZE,
    )?;
    let data_size = le_u64(&fixed, 0)? as usize;
    let data = if data_size == 0 {
        Vec::new()
    } else {
        read_exact_at(
            reader,
            checked_add(
                info.file_position,
                (SEGMENT_HEADER_SIZE + ATTACHMENT_DATA_SIZE) as u64,
            )?,
            data_size,
        )?
    };

    Ok(AttachmentBlob {
        info: info.clone(),
        data,
    })
}

pub(crate) fn read_raw_subblock<R: Read + Seek>(
    reader: &mut R,
    info: &DirectorySubBlockInfo,
) -> Result<RawSubBlock> {
    let header = SegmentHeader::read(reader, info.file_position)?;
    ensure_magic(info.file_position, &header.magic, SUBBLOCK_MAGIC)?;

    let prefix = read_exact_at(
        reader,
        checked_add(info.file_position, SEGMENT_HEADER_SIZE as u64)?,
        SUBBLOCK_MIN_DATA_SIZE,
    )?;

    let metadata_size = le_u32(&prefix, 0)? as usize;
    let attachment_size = le_u32(&prefix, 4)? as usize;
    let data_size = le_u64(&prefix, 8)? as usize;
    let schema = &prefix[16..18];

    let dynamic_header_size = match schema {
        b"DV" => {
            let dimension_count = le_u32(&prefix, 44)? as usize;
            if dimension_count > 1024 {
                return Err(CziError::file_invalid_format(format!(
                    "subblock at {} declares unreasonable dimension count {}",
                    info.file_position, dimension_count
                )));
            }
            SUBBLOCK_SEGMENT_FIXED_SIZE
                .checked_add(SUBBLOCK_DIRECTORY_ENTRY_DV_FIXED_SIZE)
                .and_then(|value| value.checked_add(dimension_count * DIMENSION_ENTRY_DV_SIZE))
                .ok_or_else(|| CziError::internal_overflow("subblock header size"))?
        }
        b"DE" => return Err(CziError::unsupported_subblock_schema("DE")),
        _ => {
            return Err(CziError::file_invalid_format(format!(
                "invalid subblock schema at {}",
                info.file_position
            )))
        }
    };

    let effective_header_size = max(dynamic_header_size, SUBBLOCK_MIN_DATA_SIZE);
    let metadata_offset = checked_add(
        info.file_position,
        (SEGMENT_HEADER_SIZE + effective_header_size) as u64,
    )?;
    let data_offset = checked_add(metadata_offset, metadata_size as u64)?;
    let attachment_offset = checked_add(data_offset, data_size as u64)?;

    let metadata = if metadata_size == 0 {
        Vec::new()
    } else {
        read_exact_at(reader, metadata_offset, metadata_size)?
    };
    let data = if data_size == 0 {
        Vec::new()
    } else {
        read_exact_at(reader, data_offset, data_size)?
    };
    let attachment = if attachment_size == 0 {
        Vec::new()
    } else {
        read_exact_at(reader, attachment_offset, attachment_size)?
    };

    let declared_size = header.effective_size() as usize;
    let actual_end = effective_header_size
        .checked_add(metadata_size)
        .and_then(|value| value.checked_add(data_size))
        .and_then(|value| value.checked_add(attachment_size))
        .ok_or_else(|| CziError::internal_overflow("subblock total size"))?;
    if declared_size < actual_end {
        return Err(CziError::file_invalid_format(format!(
            "subblock at {} overruns declared segment size",
            info.file_position
        )));
    }

    Ok(RawSubBlock {
        info: info.clone(),
        metadata,
        data,
        attachment,
    })
}

pub(crate) fn decode_subblock_bitmap(raw: &RawSubBlock) -> Result<Bitmap> {
    decode_bitmap(&raw.info, &raw.data)
}

pub(crate) fn decode_bitmap(info: &DirectorySubBlockInfo, encoded: &[u8]) -> Result<Bitmap> {
    let expected_len = expected_bitmap_len(info)?;

    let decoded = match info.compression {
        CompressionMode::UnCompressed => normalize_plain(encoded.to_vec(), expected_len),
        CompressionMode::Zstd0 => {
            let decoded = zstd::stream::decode_all(Cursor::new(encoded))
                .map_err(|err| CziError::file_decompression(err.to_string()))?;
            normalize_plain(decoded, expected_len)
        }
        CompressionMode::Zstd1 => decode_zstd1(info, encoded, expected_len)?,
        CompressionMode::Jpg | CompressionMode::JpgXr => {
            return Err(CziError::unsupported_compression(info.compression.as_str()))
        }
    };

    Bitmap::new(
        info.pixel_type,
        info.stored_size.w,
        info.stored_size.h,
        decoded,
    )
}

fn parse_file_header<R: Read + Seek>(reader: &mut R) -> Result<FileHeaderInfo> {
    let header = SegmentHeader::read(reader, 0)?;
    ensure_magic(0, &header.magic, FILE_MAGIC)?;

    let data = read_exact_at(reader, SEGMENT_HEADER_SIZE as u64, FILE_HEADER_DATA_SIZE)?;
    Ok(FileHeaderInfo {
        major: le_i32(&data, 0)?,
        minor: le_i32(&data, 4)?,
        primary_file_guid: parse_guid(&data[16..32])?,
        file_guid: parse_guid(&data[32..48])?,
        file_part: le_i32(&data, 48)?,
        subblock_directory_position: le_u64(&data, 52)?,
        metadata_position: le_u64(&data, 60)?,
        update_pending: le_i32(&data, 68)? != 0,
        attachment_directory_position: le_u64(&data, 72)?,
    })
}

fn parse_subblock_directory<R: Read + Seek>(
    reader: &mut R,
    offset: u64,
) -> Result<(Vec<DirectorySubBlockInfo>, SubBlockStatistics)> {
    let header = SegmentHeader::read(reader, offset)?;
    ensure_magic(offset, &header.magic, SUBBLOCK_DIRECTORY_MAGIC)?;

    let fixed = read_exact_at(
        reader,
        checked_add(offset, SEGMENT_HEADER_SIZE as u64)?,
        SUBBLOCK_DIRECTORY_DATA_SIZE,
    )?;
    let entry_count = le_i32(&fixed, 0)?;
    if entry_count < 0 {
        return Err(CziError::file_invalid_format(format!(
            "negative subblock directory entry count at {}",
            offset
        )));
    }

    let payload_size = header
        .effective_size()
        .checked_sub(SUBBLOCK_DIRECTORY_DATA_SIZE as u64)
        .ok_or_else(|| CziError::file_invalid_format("subblock directory used size is too small"))?
        as usize;
    let payload = read_exact_at(
        reader,
        checked_add(
            offset,
            (SEGMENT_HEADER_SIZE + SUBBLOCK_DIRECTORY_DATA_SIZE) as u64,
        )?,
        payload_size,
    )?;

    let mut cursor = 0usize;
    let mut subblocks = Vec::with_capacity(entry_count as usize);
    let mut statistics = SubBlockStatistics::default();

    for index in 0..entry_count as usize {
        if cursor + 2 > payload.len() {
            return Err(CziError::file_invalid_format(
                "subblock directory payload ended before schema bytes",
            ));
        }

        let schema = &payload[cursor..cursor + 2];
        if schema == b"DE" {
            return Err(CziError::unsupported_directory_schema("DE"));
        }
        if schema != b"DV" {
            return Err(CziError::file_invalid_format(format!(
                "unsupported subblock directory schema '{}' at entry {}",
                display_magic(schema),
                index
            )));
        }
        if cursor + SUBBLOCK_DIRECTORY_ENTRY_DV_FIXED_SIZE > payload.len() {
            return Err(CziError::file_invalid_format(
                "subblock directory payload ended inside DV header",
            ));
        }

        let pixel_type_raw = le_i32(&payload, cursor + 2)?;
        let pixel_type = PixelType::from_raw(pixel_type_raw)
            .ok_or_else(|| CziError::unsupported_pixel_type(pixel_type_raw.to_string()))?;
        let file_position = le_u64(&payload, cursor + 6)?;
        let file_part = le_i32(&payload, cursor + 14)?;
        let compression_raw = le_i32(&payload, cursor + 18)?;
        let compression = CompressionMode::from_raw(compression_raw)
            .ok_or_else(|| CziError::unsupported_compression(compression_raw.to_string()))?;
        let pyramid_type = SubBlockPyramidType::from_raw(payload[cursor + 22]);
        let dimension_count = le_i32(&payload, cursor + 28)?;
        if dimension_count < 0 || dimension_count > 1024 {
            return Err(CziError::file_invalid_format(format!(
                "invalid dimension count {} in subblock directory",
                dimension_count
            )));
        }
        let dimension_count = dimension_count as usize;
        let entry_size = SUBBLOCK_DIRECTORY_ENTRY_DV_FIXED_SIZE
            .checked_add(dimension_count * DIMENSION_ENTRY_DV_SIZE)
            .ok_or_else(|| CziError::internal_overflow("subblock directory entry size"))?;
        if cursor + entry_size > payload.len() {
            return Err(CziError::file_invalid_format(
                "subblock directory payload ended inside dimension entries",
            ));
        }

        let mut coordinate = Coordinate::new();
        let mut rect = IntRect::new(0, 0, -1, -1);
        let mut stored_size = IntSize { w: 0, h: 0 };
        let mut has_x = false;
        let mut has_y = false;
        let mut m_index = None;

        let mut dim_cursor = cursor + SUBBLOCK_DIRECTORY_ENTRY_DV_FIXED_SIZE;
        for _ in 0..dimension_count {
            let dim_code = parse_dim_code(&payload[dim_cursor..dim_cursor + 4]);
            let start = le_i32(&payload, dim_cursor + 4)?;
            let size = le_i32(&payload, dim_cursor + 8)?;
            let stored = le_i32(&payload, dim_cursor + 16)?;

            match Dimension::from_code(&dim_code) {
                Some(Dimension::X) => {
                    rect.x = start;
                    rect.w = size;
                    stored_size.w = stored.max(0) as u32;
                    has_x = true;
                }
                Some(Dimension::Y) => {
                    rect.y = start;
                    rect.h = size;
                    stored_size.h = stored.max(0) as u32;
                    has_y = true;
                }
                Some(Dimension::M) => {
                    m_index = Some(start);
                }
                Some(dimension) if dimension.is_frame_dimension() => {
                    coordinate.set(dimension, start);
                }
                _ => {}
            }

            dim_cursor += DIMENSION_ENTRY_DV_SIZE;
        }

        if !has_x || !has_y {
            return Err(CziError::file_invalid_format(format!(
                "subblock directory entry {} has no X/Y dimensions",
                index
            )));
        }

        let info = DirectorySubBlockInfo {
            index,
            file_position,
            file_part,
            pixel_type,
            compression,
            coordinate,
            rect,
            stored_size,
            m_index,
            pyramid_type,
        };
        update_statistics(&mut statistics, &info);
        subblocks.push(info);
        cursor += entry_size;
    }

    Ok((subblocks, statistics))
}

fn parse_attachment_directory<R: Read + Seek>(
    reader: &mut R,
    offset: u64,
) -> Result<Vec<AttachmentInfo>> {
    let header = SegmentHeader::read(reader, offset)?;
    ensure_magic(offset, &header.magic, ATTACHMENT_DIRECTORY_MAGIC)?;

    let fixed = read_exact_at(
        reader,
        checked_add(offset, SEGMENT_HEADER_SIZE as u64)?,
        ATTACHMENT_DATA_SIZE,
    )?;
    let entry_count = le_i32(&fixed, 0)?;
    if entry_count <= 0 {
        return Ok(Vec::new());
    }

    let entry_size = 128usize;
    let payload = read_exact_at(
        reader,
        checked_add(offset, (SEGMENT_HEADER_SIZE + ATTACHMENT_DATA_SIZE) as u64)?,
        entry_count as usize * entry_size,
    )?;

    let mut attachments = Vec::with_capacity(entry_count as usize);
    for index in 0..entry_count as usize {
        let start = index * entry_size;
        let end = start + entry_size;
        let entry = &payload[start..end];
        if &entry[..2] != b"A1" {
            continue;
        }

        let file_position = le_u64(entry, 12)?;
        let file_part = le_i32(entry, 20)?;
        let content_guid = parse_guid(&entry[24..40])?;
        let content_file_type = parse_fixed_string(&entry[40..48]);
        let name = parse_fixed_string(&entry[48..128]);
        let data_size = read_attachment_data_size(reader, file_position)?;

        attachments.push(AttachmentInfo {
            index,
            file_position,
            file_part,
            content_guid,
            content_file_type,
            name,
            data_size,
        });
    }

    Ok(attachments)
}

fn read_attachment_data_size<R: Read + Seek>(reader: &mut R, offset: u64) -> Result<u64> {
    let header = SegmentHeader::read(reader, offset)?;
    ensure_magic(offset, &header.magic, ATTACHMENT_MAGIC)?;
    let prefix = read_exact_at(reader, checked_add(offset, SEGMENT_HEADER_SIZE as u64)?, 16)?;
    le_u64(&prefix, 0)
}

fn update_statistics(statistics: &mut SubBlockStatistics, entry: &DirectorySubBlockInfo) {
    statistics.subblock_count += 1;

    statistics.bounding_box = Some(match statistics.bounding_box {
        Some(mut existing) => {
            existing.union_with(entry.rect);
            existing
        }
        None => entry.rect,
    });

    if entry.is_layer0() {
        statistics.bounding_box_layer0 = Some(match statistics.bounding_box_layer0 {
            Some(mut existing) => {
                existing.union_with(entry.rect);
                existing
            }
            None => entry.rect,
        });
    }

    for (dimension, value) in entry.coordinate.iter() {
        statistics.dim_bounds.update_value(dimension, value);
    }

    if let Some(m_index) = entry.m_index {
        statistics.min_m_index = Some(match statistics.min_m_index {
            Some(value) => value.min(m_index),
            None => m_index,
        });
        statistics.max_m_index = Some(match statistics.max_m_index {
            Some(value) => value.max(m_index),
            None => m_index,
        });
    }

    if let Some(scene_index) = entry.coordinate.get(Dimension::S) {
        let scene = statistics
            .scene_bounding_boxes
            .entry(scene_index)
            .or_insert(BoundingBoxes {
                all: entry.rect,
                layer0: IntRect::new(0, 0, -1, -1),
            });
        scene.all.union_with(entry.rect);
        if entry.is_layer0() {
            scene.layer0 = if scene.layer0.is_valid() {
                let mut value = scene.layer0;
                value.union_with(entry.rect);
                value
            } else {
                entry.rect
            };
        }
    }
}

fn decode_zstd1(
    info: &DirectorySubBlockInfo,
    encoded: &[u8],
    expected_len: usize,
) -> Result<Vec<u8>> {
    let (header_size, unpack_lo_hi) = parse_zstd1_header(encoded)?;
    let compressed = &encoded[header_size..];
    let decoded = zstd::stream::decode_all(Cursor::new(compressed))
        .map_err(|err| CziError::file_decompression(err.to_string()))?;

    if unpack_lo_hi {
        if !matches!(info.pixel_type, PixelType::Gray16 | PixelType::Bgr48) {
            return Err(CziError::unsupported_pixel_type(info.pixel_type.as_str()));
        }
        normalize_hilo(decoded, expected_len)
    } else {
        Ok(normalize_plain(decoded, expected_len))
    }
}

fn parse_zstd1_header(encoded: &[u8]) -> Result<(usize, bool)> {
    if encoded.is_empty() {
        return Err(CziError::file_decompression("zstd1 data is empty"));
    }

    match encoded[0] {
        1 => Ok((1, false)),
        3 => {
            if encoded.len() < 3 {
                return Err(CziError::file_decompression("zstd1 header is truncated"));
            }
            if encoded[1] != 1 {
                return Err(CziError::file_decompression("unsupported zstd1 chunk type"));
            }
            Ok((3, (encoded[2] & 1) == 1))
        }
        _ => Err(CziError::file_decompression("invalid zstd1 header")),
    }
}

fn normalize_plain(mut decoded: Vec<u8>, expected_len: usize) -> Vec<u8> {
    if decoded.len() < expected_len {
        decoded.resize(expected_len, 0);
        decoded
    } else {
        decoded.truncate(expected_len);
        decoded
    }
}

fn normalize_hilo(decoded: Vec<u8>, expected_len: usize) -> Result<Vec<u8>> {
    if decoded.len() % 2 != 0 {
        return Err(CziError::file_decompression(
            "hi/lo packed payload length must be even",
        ));
    }

    let mut unpacked = vec![0u8; decoded.len()];
    let half = decoded.len() / 2;
    for index in 0..half {
        unpacked[index * 2] = decoded[index];
        unpacked[index * 2 + 1] = decoded[index + half];
    }

    Ok(normalize_plain(unpacked, expected_len))
}

fn expected_bitmap_len(info: &DirectorySubBlockInfo) -> Result<usize> {
    (info.stored_size.w as usize)
        .checked_mul(info.stored_size.h as usize)
        .and_then(|value| value.checked_mul(info.pixel_type.bytes_per_pixel()))
        .ok_or_else(|| CziError::internal_overflow("bitmap byte count"))
}

fn read_exact_at<R: Read + Seek>(reader: &mut R, offset: u64, len: usize) -> Result<Vec<u8>> {
    reader.seek(SeekFrom::Start(offset))?;
    let mut buffer = vec![0u8; len];
    reader.read_exact(&mut buffer)?;
    Ok(buffer)
}

fn checked_add(lhs: u64, rhs: u64) -> Result<u64> {
    lhs.checked_add(rhs)
        .ok_or_else(|| CziError::internal_overflow("file offset add"))
}

fn ensure_magic(offset: u64, actual: &[u8], expected: &[u8; 16]) -> Result<()> {
    if actual == expected {
        return Ok(());
    }
    Err(CziError::file_invalid_magic(
        offset,
        display_magic(expected),
        display_magic(actual),
    ))
}

fn parse_guid(bytes: &[u8]) -> Result<String> {
    if bytes.len() != 16 {
        return Err(CziError::file_invalid_format("GUID must be 16 bytes"));
    }
    let data1 = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    let data2 = u16::from_le_bytes(bytes[4..6].try_into().unwrap());
    let data3 = u16::from_le_bytes(bytes[6..8].try_into().unwrap());
    Ok(format!(
        "{data1:08x}-{data2:04x}-{data3:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    ))
}

fn parse_dim_code(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_owned()
}

fn parse_fixed_string(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_owned()
}

fn display_magic(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

fn le_i32(bytes: &[u8], offset: usize) -> Result<i32> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| CziError::file_invalid_format("buffer too small for i32"))?;
    Ok(i32::from_le_bytes(slice.try_into().unwrap()))
}

fn le_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| CziError::file_invalid_format("buffer too small for u32"))?;
    Ok(u32::from_le_bytes(slice.try_into().unwrap()))
}

fn le_u64(bytes: &[u8], offset: usize) -> Result<u64> {
    let slice = bytes
        .get(offset..offset + 8)
        .ok_or_else(|| CziError::file_invalid_format("buffer too small for u64"))?;
    Ok(u64::from_le_bytes(slice.try_into().unwrap()))
}
