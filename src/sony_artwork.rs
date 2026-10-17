//! Reconstructed from Sony SonicStage 4.4 AuraPACAPI.dll using REA/Ghidra.
//! See docs/research/sony-inventory.md: FUN_6d2b4c20, FUN_6d2431a6,
//! FUN_6d25ffa6, FUN_6d26031c. Namespace/object business mapping remains unverified.
use anyhow::{Result, ensure};
use std::{
    fs::{self, File},
    path::Path,
};

fn binary_rows<'a>(bytes: &'a [u8], signature: &[u8; 4], tag: &[u8; 4]) -> Result<Vec<&'a [u8]>> {
    ensure!(
        bytes.len() >= 16 && &bytes[..4] == signature,
        "Invalid artwork index header"
    );
    let sections = bytes[8] as usize;
    ensure!(
        sections > 0 && 16 + sections * 16 <= bytes.len(),
        "Truncated artwork index directory"
    );
    let mut result = None;
    for directory in bytes[16..16 + sections * 16].chunks_exact(16) {
        if &directory[..4] != tag {
            continue;
        }
        ensure!(result.is_none(), "Duplicate artwork index section");
        let start = u32::from_be_bytes(directory[4..8].try_into().unwrap()) as usize;
        let length = u32::from_be_bytes(directory[8..12].try_into().unwrap()) as usize;
        ensure!(
            start >= 16 + sections * 16
                && length >= 16
                && start
                    .checked_add(length)
                    .is_some_and(|end| end <= bytes.len()),
            "Artwork index section out of bounds"
        );
        ensure!(
            &bytes[start..start + 4] == tag,
            "Artwork index section identifier mismatch"
        );
        let count = u16::from_be_bytes(bytes[start + 4..start + 6].try_into().unwrap()) as usize;
        let width = u16::from_be_bytes(bytes[start + 6..start + 8].try_into().unwrap()) as usize;
        ensure!(
            width > 0 && 16 + count * width <= length,
            "Invalid artwork index record size"
        );
        result = Some(
            bytes[start + 16..start + 16 + count * width]
                .chunks_exact(width)
                .collect(),
        );
    }
    result.ok_or_else(|| anyhow::anyhow!("Missing artwork index section"))
}
fn picture_file(root: &Path, canonical_root: &Path, descriptor: &str) -> Result<Option<String>> {
    let Some((_, relative)) = descriptor.split_once(';') else {
        return Ok(None);
    };
    let relative = relative.replace('\\', "/");
    ensure!(
        !relative.is_empty()
            && !relative.contains(':')
            && relative
                .split('/')
                .all(|part| !part.is_empty() && part != "." && part != ".."),
        "Invalid device artwork path"
    );
    let path = root.join(relative);
    if !path.is_file() {
        return Ok(None);
    }
    let resolved = path.canonicalize()?;
    ensure!(
        resolved.starts_with(canonical_root),
        "Device artwork path escapes the library"
    );
    Ok(Some(resolved.to_string_lossy().into_owned()))
}

/// Follow the firmware's selected thumbnail tree and stable numeric track IDs.
/// This avoids matching full OMA text against truncated GPIF display labels.
pub fn existing_track_covers(root: &Path) -> Result<std::collections::BTreeMap<u16, String>> {
    let mut result = std::collections::BTreeMap::new();
    let path = root.join("02TREINF.DAT");
    if !path.exists() {
        return Ok(result);
    }
    let info = fs::read(path)?;
    let mut tree_number = None;
    for (index, row) in binary_rows(&info, b"GTIF", b"GTFB")?
        .into_iter()
        .enumerate()
    {
        if row.len() < 16 {
            continue;
        }
        let fields = u16::from_be_bytes(row[12..14].try_into().unwrap()) as usize;
        let width = u16::from_be_bytes(row[14..16].try_into().unwrap()) as usize;
        if fields == 0 {
            continue;
        }
        ensure!(
            width == 128 && 16 + fields * width <= row.len(),
            "Invalid artwork index record size"
        );
        for field in row[16..16 + fields * width].chunks_exact(width) {
            if &field[..6] != b"TIT2\x00\x02" {
                continue;
            }
            let units: Vec<u16> = field[6..]
                .chunks_exact(2)
                .map(|b| u16::from_be_bytes([b[0], b[1]]))
                .take_while(|u| *u != 0)
                .collect();
            if String::from_utf16(&units)? == "STD_TPE1_THUMBNAIL" {
                ensure!(
                    tree_number.replace(index + 1).is_none(),
                    "Duplicate artwork tree marker"
                );
            }
        }
    }
    let Some(number) = tree_number else {
        return Ok(result);
    };
    let tree = fs::read(root.join(format!("01TREE{number:02X}.DAT")))?;
    let groups = binary_rows(&tree, b"TREE", b"GPLB")?;
    let tracks = binary_rows(&tree, b"TREE", b"TPLB")?;
    let metadata = fs::read(root.join(format!("03GINF{number:02X}.DAT")))?;
    let records = album_records(&metadata)?;
    let canonical_root = root.canonicalize()?;
    let mut ranges = vec![];
    for group in groups {
        ensure!(group.len() >= 8, "Invalid artwork index record size");
        let id = u16::from_be_bytes(group[..2].try_into().unwrap()) as usize;
        let start = u16::from_be_bytes(group[4..6].try_into().unwrap()) as usize;
        if start == 0 {
            continue;
        }
        ensure!(
            id > 0 && id <= records.len() && start <= tracks.len(),
            "Artwork track mapping out of bounds"
        );
        ensure!(
            ranges.last().is_none_or(|&(_, previous)| start > previous),
            "Ambiguous artwork track mapping"
        );
        ranges.push((id, start));
    }
    for (index, &(id, start)) in ranges.iter().enumerate() {
        let end = ranges
            .get(index + 1)
            .map(|&(_, next)| next - 1)
            .unwrap_or(tracks.len());
        let record = &records[id - 1];
        let mut picture = None;
        for tag in [b"PICP", b"PIC0"] {
            if !record.fields.contains_key(tag) {
                continue;
            }
            for descriptor in record
                .text(&metadata, tag)?
                .split('|')
                .filter(|value| !value.is_empty())
            {
                if let Some(file) = picture_file(root, &canonical_root, descriptor)? {
                    picture = Some(file);
                    break;
                }
            }
            if picture.is_some() {
                break;
            }
        }
        let Some(picture) = picture else {
            continue;
        };
        for track in &tracks[start - 1..end] {
            ensure!(track.len() >= 2, "Invalid artwork index record size");
            let id = u16::from_be_bytes(track[..2].try_into().unwrap());
            ensure!(id > 0, "Artwork track mapping out of bounds");
            if let Some(previous) = result.insert(id, picture.clone()) {
                ensure!(previous == picture, "Ambiguous artwork track mapping");
            }
        }
    }
    Ok(result)
}

/// Recover cover sources for existing songs before rebuilding indexes. Reject
/// absolute paths and traversal rather than trusting device metadata as a path.
pub fn existing_album_covers(
    root: &Path,
) -> Result<std::collections::BTreeMap<(String, String), String>> {
    let mut result = std::collections::BTreeMap::new();
    let path = root.join("03GINF01.DAT");
    if !path.exists() {
        return Ok(result);
    }
    let bytes = fs::read(path)?;
    let canonical_root = root.canonicalize()?;
    for record in album_records(&bytes)? {
        let name = record.text(&bytes, b"TIT2")?;
        let artist = record.text(&bytes, b"TPE1")?;
        if !record.fields.contains_key(b"PICP") {
            continue;
        }
        let descriptor = record.text(&bytes, b"PICP")?;
        let Some(path) = picture_file(root, &canonical_root, &descriptor)? else {
            continue;
        };
        // Legacy JSymphonic prepends artist/ to album group titles. Keep both
        // aliases so rebuilding a library does not discard its existing art.
        if let Some(album) = name.strip_prefix(&format!("{artist}/")) {
            result.insert((artist.clone(), album.into()), path.clone());
        }
        result.insert((artist, name), path);
    }
    Ok(result)
}

pub fn relative_path(namespace: u16, object: u16, index: u8) -> Result<String> {
    ensure!(object > 0, "Picture object ID must be in 1..65535");
    ensure!(
        namespace <= 255 && index <= 20,
        "Picture naming parameters are outside supported limits"
    );
    let owner = if namespace <= 1 {
        "1G".into()
    } else {
        format!(
            "{}{}",
            char::from_u32((namespace >> 4) as u32 + 0x47).unwrap(),
            char::from_u32((namespace & 15) as u32 + 0x47).unwrap()
        )
    };
    let suffix = if index == 0 {
        "P0".into()
    } else {
        format!("{}{}", (index - 1) / 2, ((index + 1) & 1) + 1)
    };
    Ok(format!(
        "20P{:02X}\\{owner}{object:04X}{suffix}.JPG",
        object & 31
    ))
}
pub fn descriptor(width: u32, height: u32, path: &str) -> Result<String> {
    ensure!(
        (1..=999).contains(&width) && (1..=999).contains(&height),
        "Image dimensions exceed Sony field limits"
    );
    ensure!(
        !path.contains(';')
            && !path.contains('|')
            && !path.contains("..")
            && !path.starts_with('\\'),
        "Invalid relative picture path"
    );
    Ok(format!("{width:03}{height:03};{path}"))
}
pub fn prepare_images(
    root: &Path,
    source: &Path,
    namespace: u16,
    object: u16,
) -> Result<(String, String)> {
    let image = image::open(source)?.to_rgb8();
    let mut descriptors = vec![];
    // These square sizes are our conservative choices, not a recovered Sony requirement.
    // Squares avoid claiming the still-unresolved order of the two size parameters.
    for (index, size) in [200, 96, 32].into_iter().enumerate() {
        let fitted = image::imageops::thumbnail(&image, size, size);
        let mut square = image::RgbImage::from_pixel(size, size, image::Rgb([255, 255, 255]));
        image::imageops::overlay(
            &mut square,
            &fitted,
            ((size - fitted.width()) / 2) as i64,
            ((size - fitted.height()) / 2) as i64,
        );
        let rel = relative_path(namespace, object, index as u8)?;
        let path = root.join(rel.replace('\\', "/"));
        fs::create_dir_all(path.parent().unwrap())?;
        image::codecs::jpeg::JpegEncoder::new_with_quality(File::create(path)?, 90)
            .encode_image(&square)?;
        descriptors.push(descriptor(size, size, &rel)?);
    }
    Ok((descriptors.remove(0), descriptors.join("|")))
}

/// Patch only a pre-existing album group's picture fields, leaving every other
/// database byte alone. Caller supplies an already copied host-side library.
pub fn patch_album_cover(root: &Path, source: &Path, album: &str, artist: &str) -> Result<usize> {
    let file = root.join("03GINF01.DAT");
    let mut bytes = fs::read(&file)?;
    let records = album_records(&bytes)?;
    let mut matches = Vec::new();
    for record in records {
        let name = record.text(&bytes, b"TIT2")?;
        let owner = record.text(&bytes, b"TPE1")?;
        if (name == album || name == format!("{artist}/{album}")) && owner == artist {
            matches.push(record);
        }
    }
    ensure!(
        matches.len() == 1,
        "Album match is not unique; artwork probe stopped"
    );
    let record = &matches[0];
    let primary_offset = record.field(b"PICP")?;
    let others_offset = record.field(b"PIC0")?;
    // The filename object value is deliberately allocated in a free namespace.
    // It is not asserted to be Sony property 0x50018 or a record identifier.
    let mut occupied = std::collections::HashSet::new();
    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = entry?;
        if entry.file_type().is_file() {
            occupied.insert(
                entry
                    .path()
                    .strip_prefix(root)?
                    .to_string_lossy()
                    .replace('\\', "/")
                    .to_uppercase(),
            );
        }
    }
    let object = (1..=u16::MAX)
        .find(|id| {
            (0..3).all(|index| {
                !occupied.contains(
                    &relative_path(1, *id, index)
                        .unwrap()
                        .replace('\\', "/")
                        .to_uppercase(),
                )
            })
        })
        .ok_or_else(|| anyhow::anyhow!("No artwork file IDs are available"))?;
    let (primary, others) = prepare_images(root, source, 1, object)?;
    bytes[primary_offset..primary_offset + 128]
        .copy_from_slice(&crate::protocol::fixed_field("PICP", &primary));
    bytes[others_offset..others_offset + 128]
        .copy_from_slice(&crate::protocol::fixed_field("PIC0", &others));
    fs::write(file, bytes)?;
    Ok(record.index + 1)
}

struct AlbumRecord {
    index: usize,
    fields: std::collections::BTreeMap<[u8; 4], usize>,
}
impl AlbumRecord {
    fn field(&self, tag: &[u8; 4]) -> Result<usize> {
        self.fields.get(tag).copied().ok_or_else(|| {
            anyhow::anyhow!(
                "Album record is missing field {}",
                String::from_utf8_lossy(tag)
            )
        })
    }
    fn text(&self, bytes: &[u8], tag: &[u8; 4]) -> Result<String> {
        let start = self.field(tag)?;
        ensure!(
            &bytes[start + 4..start + 6] == [0, 2],
            "Unsupported album text encoding"
        );
        let units: Vec<_> = bytes[start + 6..start + 128]
            .chunks_exact(2)
            .map(|x| u16::from_be_bytes([x[0], x[1]]))
            .take_while(|x| *x != 0)
            .collect();
        Ok(String::from_utf16(&units)?)
    }
}
fn album_records(bytes: &[u8]) -> Result<Vec<AlbumRecord>> {
    ensure!(
        bytes.len() >= 16 && &bytes[..4] == b"GPIF",
        "Unsupported album table identifier"
    );
    let classes = bytes[8] as usize;
    ensure!(
        classes > 0 && 16 + classes * 16 <= bytes.len(),
        "Album table directory is truncated"
    );
    let mut section = None;
    for i in 0..classes {
        let descriptor = &bytes[16 + i * 16..32 + i * 16];
        if &descriptor[..4] == b"GPFB" {
            ensure!(section.is_none(), "Album table has multiple GPFB sections");
            section = Some((
                u32::from_be_bytes(descriptor[4..8].try_into().unwrap()) as usize,
                u32::from_be_bytes(descriptor[8..12].try_into().unwrap()) as usize,
            ));
        }
    }
    let (start, length) =
        section.ok_or_else(|| anyhow::anyhow!("Album table has no GPFB section"))?;
    ensure!(
        start >= 16 + classes * 16
            && length >= 16
            && start
                .checked_add(length)
                .is_some_and(|end| end <= bytes.len()),
        "Album table section is out of bounds"
    );
    ensure!(
        &bytes[start..start + 4] == b"GPFB",
        "Album table section type mismatch"
    );
    let count = u16::from_be_bytes(bytes[start + 4..start + 6].try_into().unwrap()) as usize;
    let stride = u16::from_be_bytes(bytes[start + 6..start + 8].try_into().unwrap()) as usize;
    ensure!(
        stride >= 16 && (stride - 16) % 128 == 0 && 16 + count * stride <= length,
        "Invalid album table record size"
    );
    let mut result = Vec::with_capacity(count);
    for index in 0..count {
        let offset = start + 16 + index * stride;
        let fields =
            u16::from_be_bytes(bytes[offset + 12..offset + 14].try_into().unwrap()) as usize;
        let field_size =
            u16::from_be_bytes(bytes[offset + 14..offset + 16].try_into().unwrap()) as usize;
        ensure!(
            field_size == 128 && 16 + fields * field_size <= stride,
            "Invalid album field size"
        );
        let mut map = std::collections::BTreeMap::new();
        for i in 0..fields {
            let field = offset + 16 + i * 128;
            let tag = bytes[field..field + 4].try_into().unwrap();
            ensure!(
                map.insert(tag, field).is_none(),
                "Duplicate album record field"
            );
        }
        result.push(AlbumRecord { index, fields: map });
    }
    Ok(result)
}

/// Upgrade the legacy two-field hierarchical GPFB representation to the four
/// fields recovered in Sony's hierarchical-tree constructors. This is a local
/// format transform only; namespace selection still needs caller evidence.
pub fn expand_hierarchical_pictures(bytes: &[u8]) -> Result<Vec<u8>> {
    let records = album_records(bytes)?;
    ensure!(
        bytes[8] == 1,
        "Hierarchy conversion requires one GPFB section"
    );
    ensure!(
        records.iter().all(|r| r.fields.len() == 2
            && r.fields.contains_key(b"TIT2")
            && r.fields.contains_key(b"XSOT")),
        "Not a recognized two-field hierarchy"
    );
    let start = u32::from_be_bytes(bytes[20..24].try_into().unwrap()) as usize;
    let old_len = u32::from_be_bytes(bytes[24..28].try_into().unwrap()) as usize;
    let old_stride = u16::from_be_bytes(bytes[start + 6..start + 8].try_into().unwrap()) as usize;
    ensure!(
        old_stride == 16 + 2 * 128,
        "Two-field record has unknown reserved data; conversion refused"
    );
    let new_stride = 16 + 4 * 128;
    let new_len = 16 + records.len() * new_stride;
    let mut out = bytes[..start + 16].to_vec();
    out[24..28].copy_from_slice(&(new_len as u32).to_be_bytes());
    out[start + 6..start + 8].copy_from_slice(&(new_stride as u16).to_be_bytes());
    for record in &records {
        let offset = start + 16 + record.index * old_stride;
        let mut header = bytes[offset..offset + 16].to_vec();
        header[12..14].copy_from_slice(&4u16.to_be_bytes());
        out.extend(header);
        let title = record.field(b"TIT2")?;
        let sort = record.field(b"XSOT")?;
        out.extend(&bytes[title..title + 128]);
        out.extend(crate::protocol::fixed_field("PICP", ""));
        out.extend(crate::protocol::fixed_field("PIC0", ""));
        out.extend(&bytes[sort..sort + 128]);
    }
    out.extend(&bytes[start + old_len..]);
    let parsed = album_records(&out)?;
    ensure!(
        parsed.len() == records.len(),
        "Hierarchy conversion changed the record count"
    );
    Ok(out)
}

/// Candidate for the observed artist/album hierarchy; namespace is selected
/// from the current device, not claimed to be a Sony compile-time constant.
pub fn patch_hierarchical_album_cover(
    root: &Path,
    source: &Path,
    album: &str,
    namespace: u16,
) -> Result<usize> {
    let file = root.join(format!("03GINF{namespace:02X}.DAT"));
    let original = fs::read(&file)?;
    let mut bytes = expand_hierarchical_pictures(&original)?;
    let mut matches: Vec<_> = album_records(&bytes)?
        .into_iter()
        .filter(|r| r.text(&bytes, b"TIT2").is_ok_and(|title| title == album))
        .collect();
    ensure!(
        matches.len() == 1,
        "Cannot uniquely match the hierarchy album"
    );
    let record = matches.remove(0);
    let object: u16 = (record.index + 1).try_into()?;
    for index in 0..3 {
        ensure!(
            !root
                .join(relative_path(namespace, object, index)?.replace('\\', "/"))
                .exists(),
            "Candidate artwork already exists; overwrite refused"
        );
    }
    let (primary, extra) = prepare_images(root, source, namespace, object)?;
    let p = record.field(b"PICP")?;
    let e = record.field(b"PIC0")?;
    bytes[p..p + 128].copy_from_slice(&crate::protocol::fixed_field("PICP", &primary));
    bytes[e..e + 128].copy_from_slice(&crate::protocol::fixed_field("PIC0", &extra));
    let old_records = album_records(&original)?;
    let new_records = album_records(&bytes)?;
    for (old, new) in old_records.iter().zip(new_records.iter()) {
        for tag in [b"TIT2", b"XSOT"] {
            ensure!(
                old.text(&original, tag)? == new.text(&bytes, tag)?,
                "Hierarchy conversion changed existing text"
            );
        }
    }
    fs::write(file, bytes)?;
    Ok(record.index + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_artwork_tree_rejects_truncated_or_out_of_bounds_sections() {
        let temp = tempfile::tempdir().unwrap();
        crate::protocol::write_database(temp.path(), &[], true).unwrap();
        assert!(existing_track_covers(temp.path()).unwrap().is_empty());
        let path = temp.path().join("01TREE2D.DAT");
        let original = fs::read(&path).unwrap();
        let mut bad = original.clone();
        bad[20..24].copy_from_slice(&u32::MAX.to_be_bytes());
        fs::write(&path, bad).unwrap();
        assert!(
            existing_track_covers(temp.path())
                .unwrap_err()
                .to_string()
                .contains("out of bounds")
        );
        fs::write(&path, &original[..20]).unwrap();
        assert!(
            existing_track_covers(temp.path())
                .unwrap_err()
                .to_string()
                .contains("Truncated")
        );
    }
    #[test]
    fn sony_recovered_naming() {
        assert_eq!(relative_path(1, 1, 0).unwrap(), "20P01\\1G0001P0.JPG");
        assert_eq!(relative_path(1, 1, 1).unwrap(), "20P01\\1G000101.JPG");
        assert_eq!(relative_path(1, 1, 2).unwrap(), "20P01\\1G000102.JPG");
        assert_eq!(relative_path(1, 1, 3).unwrap(), "20P01\\1G000111.JPG");
        assert_eq!(relative_path(45, 0x1234, 0).unwrap(), "20P14\\IT1234P0.JPG");
    }
    #[test]
    fn sony_recovered_picture_descriptor() {
        assert_eq!(
            descriptor(200, 200, "20P01\\1G0001P0.JPG").unwrap(),
            "200200;20P01\\1G0001P0.JPG"
        );
        assert!(descriptor(1000, 200, "x.JPG").is_err());
        assert!(descriptor(200, 200, "../x.JPG").is_err());
        let pic0 = format!(
            "{}|{}",
            descriptor(96, 96, "20P01\\1G000101.JPG").unwrap(),
            descriptor(32, 32, "20P01\\1G000102.JPG").unwrap()
        );
        assert!(pic0.encode_utf16().count() <= 60);
    }

    fn relocated_album_fixture() -> Vec<u8> {
        // Deliberately differs from our writer: relocated section, reordered fields,
        // eight fields, an unused trailing slot and nonzero opaque header/padding.
        let stride = 16 + 9 * 128;
        let mut b = vec![0xa5; 64 + 16 + stride];
        b[..4].copy_from_slice(b"GPIF");
        b[8] = 1;
        b[16..20].copy_from_slice(b"GPFB");
        b[20..24].copy_from_slice(&64u32.to_be_bytes());
        b[24..28].copy_from_slice(&((16 + stride) as u32).to_be_bytes());
        b[64..68].copy_from_slice(b"GPFB");
        b[68..70].copy_from_slice(&1u16.to_be_bytes());
        b[70..72].copy_from_slice(&(stride as u16).to_be_bytes());
        b[92..94].copy_from_slice(&8u16.to_be_bytes());
        b[94..96].copy_from_slice(&128u16.to_be_bytes());
        for (i, (tag, text)) in [
            ("UNKN", "opaque"),
            ("PIC0", ""),
            ("TPE1", "Artist"),
            ("TSOP", ""),
            ("PICP", ""),
            ("TIT2", "Album"),
            ("TCON", "Jazz"),
            ("ABCD", "preserve"),
        ]
        .into_iter()
        .enumerate()
        {
            b[96 + i * 128..96 + (i + 1) * 128]
                .copy_from_slice(&crate::protocol::fixed_field(tag, text));
        }
        b
    }

    #[test]
    fn patch_parses_actual_layout_and_preserves_opaque_bytes_and_existing_images() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        let before = relocated_album_fixture();
        fs::write(root.join("03GINF01.DAT"), &before).unwrap();
        fs::create_dir(root.join("20P01")).unwrap();
        let existing = root.join("20P01/1G0001P0.JPG");
        fs::write(&existing, b"existing artwork must survive").unwrap();
        let source = root.join("source.png");
        image::RgbImage::from_pixel(2, 1, image::Rgb([200, 30, 40]))
            .save(&source)
            .unwrap();
        assert_eq!(
            patch_album_cover(root, &source, "Album", "Artist").unwrap(),
            1
        );
        let after = fs::read(root.join("03GINF01.DAT")).unwrap();
        let primary = 96 + 4 * 128;
        let extra = 96 + 128;
        for (i, (old, new)) in before.iter().zip(&after).enumerate() {
            if !(primary..primary + 128).contains(&i) && !(extra..extra + 128).contains(&i) {
                assert_eq!(old, new, "unexpected change at {i}");
            }
        }
        let record = album_records(&after).unwrap().remove(0);
        assert_eq!(
            record.text(&after, b"PICP").unwrap(),
            "200200;20P02\\1G0002P0.JPG"
        );
        assert!(record.text(&after, b"PIC0").unwrap().contains('|'));
        assert_eq!(
            fs::read(existing).unwrap(),
            b"existing artwork must survive"
        );
        assert!(root.join("20P02/1G000201.JPG").is_file());
    }

    #[test]
    fn malformed_album_bounds_and_duplicate_fields_are_rejected() {
        let mut b = relocated_album_fixture();
        b[68..70].copy_from_slice(&u16::MAX.to_be_bytes());
        assert!(album_records(&b).is_err());
        let mut b = relocated_album_fixture();
        b[96..100].copy_from_slice(b"PICP");
        assert!(album_records(&b).is_err());
        for length in [0, 15, 31, 63, 79, 100] {
            assert!(album_records(&relocated_album_fixture()[..length]).is_err());
        }
    }

    #[test]
    fn hierarchy_conversion_preserves_record_identity_and_text() {
        let stride = 16 + 256;
        let mut b = vec![0; 48 + stride];
        b[..4].copy_from_slice(b"GPIF");
        b[8] = 1;
        b[16..20].copy_from_slice(b"GPFB");
        b[20..24].copy_from_slice(&32u32.to_be_bytes());
        b[24..28].copy_from_slice(&((16 + stride) as u32).to_be_bytes());
        b[32..36].copy_from_slice(b"GPFB");
        b[36..38].copy_from_slice(&1u16.to_be_bytes());
        b[38..40].copy_from_slice(&(stride as u16).to_be_bytes());
        b[48..60].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
        b[60..62].copy_from_slice(&2u16.to_be_bytes());
        b[62..64].copy_from_slice(&128u16.to_be_bytes());
        b[64..192].copy_from_slice(&crate::protocol::fixed_field("TIT2", "专辑 🎵"));
        b[192..320].copy_from_slice(&crate::protocol::fixed_field("XSOT", "排序"));
        let out = expand_hierarchical_pictures(&b).unwrap();
        let r = album_records(&out).unwrap().remove(0);
        assert_eq!(&out[48..60], &b[48..60]);
        assert_eq!(r.text(&out, b"TIT2").unwrap(), "专辑 🎵");
        assert_eq!(r.text(&out, b"XSOT").unwrap(), "排序");
        assert_eq!(r.text(&out, b"PICP").unwrap(), "");
        assert_eq!(r.field(b"XSOT").unwrap(), 64 + 3 * 128);
        assert!(expand_hierarchical_pictures(&out).is_err());
    }
}
