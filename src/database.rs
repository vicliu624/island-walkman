//! Declarative OMGAUDIO serialization from observed on-device binary layouts.
//! All lengths and counts are derived from section/row data. See docs/PROTOCOL.md.
use crate::{metadata::Track, protocol::fixed_field, sony_artwork};
use anyhow::{Result, ensure};
use std::{collections::BTreeMap, fs, path::Path};

struct Section {
    tag: [u8; 4],
    count: usize,
    width: usize,
    flags: [u32; 2],
    payload: Vec<u8>,
    minimum_size: usize,
}
fn number16(bytes: &mut Vec<u8>, value: usize) {
    bytes.extend((value as u16).to_be_bytes());
}
fn number32(bytes: &mut Vec<u8>, value: usize) {
    bytes.extend((value as u32).to_be_bytes());
}
impl Section {
    fn rows(tag: &str, width: usize, rows: Vec<Vec<u8>>) -> Self {
        assert!(rows.iter().all(|row| row.len() == width));
        Self {
            tag: tag.as_bytes().try_into().unwrap(),
            count: rows.len(),
            width,
            flags: [0; 2],
            payload: rows.concat(),
            minimum_size: 0,
        }
    }
    fn encode(self) -> Vec<u8> {
        let mut bytes = self.tag.to_vec();
        number16(&mut bytes, self.count);
        number16(&mut bytes, self.width);
        for flag in self.flags {
            bytes.extend(flag.to_be_bytes());
        }
        bytes.extend(self.payload);
        bytes.resize(bytes.len().max(self.minimum_size), 0);
        bytes
    }
}
fn document(signature: &str, sections: Vec<Section>) -> Vec<u8> {
    let encoded: Vec<_> = sections.into_iter().map(Section::encode).collect();
    let mut output = vec![0; 16];
    output[..4].copy_from_slice(signature.as_bytes());
    output[4..6].copy_from_slice(&[1, 1]);
    output[8] = encoded.len() as u8;
    let mut offset = 16 + 16 * encoded.len();
    for section in &encoded {
        output.extend(&section[..4]);
        number32(&mut output, offset);
        number32(&mut output, section.len());
        number32(&mut output, 0);
        offset += section.len();
    }
    for section in encoded {
        output.extend(section);
    }
    output
}
fn metadata_row(duration: u32, fields: &[(&str, String)]) -> Vec<u8> {
    let mut row = vec![0; 8];
    row.extend(duration.to_be_bytes());
    number16(&mut row, fields.len());
    number16(&mut row, 128);
    for (tag, value) in fields {
        row.extend(fixed_field(tag, value));
    }
    row
}
fn group_file(rows: Vec<Vec<u8>>, width: usize) -> Vec<u8> {
    document("GPIF", vec![Section::rows("GPFB", width, rows)])
}
#[derive(Default)]
struct Tree {
    groups: Vec<(usize, u8, usize)>,
    tracks: Vec<u16>,
}
impl Tree {
    fn encode(self) -> Result<Vec<u8>> {
        ensure!(
            self.groups.len() <= 2048,
            "Too many device groups (maximum 2048)"
        );
        let rows = self
            .groups
            .iter()
            .map(|(id, level, start)| {
                let mut row = vec![];
                number16(&mut row, *id);
                row.extend([*level, 0]);
                number16(&mut row, *start);
                number16(&mut row, 0);
                row
            })
            .collect();
        let mut groups = Section::rows("GPLB", 8, rows);
        groups.flags[0] = groups.count as u32;
        if !self.groups.is_empty() {
            groups.minimum_size = 0x4010;
        }
        let rows = self
            .tracks
            .iter()
            .map(|id| id.to_be_bytes().to_vec())
            .collect();
        let mut tracks = Section::rows("TPLB", 2, rows);
        tracks.flags[0] = tracks.count as u32;
        // The observed TPLB class always has a final alignment block.
        tracks.minimum_size = 16 + tracks.count * 2 + (16 - tracks.count * 2 % 16);
        if self.groups.is_empty() && tracks.count == 0 {
            tracks.minimum_size = 16;
        }
        Ok(document("TREE", vec![groups, tracks]))
    }
}
fn duration(tracks: &[&Track]) -> u32 {
    tracks
        .iter()
        .map(|track| track.duration_ms as u64)
        .sum::<u64>()
        .min(u32::MAX as u64) as u32
}
fn images(
    root: &Path,
    tracks: &[&Track],
    namespace: u16,
    object: usize,
    enabled: bool,
) -> Result<(String, String)> {
    if enabled {
        if let Some(path) = tracks.iter().find_map(|track| track.cover.as_deref()) {
            return sony_artwork::prepare_images(
                root,
                Path::new(path),
                namespace,
                object.try_into()?,
            );
        }
    }
    Ok((String::new(), String::new()))
}
fn hierarchy_row(title: &str, duration: u32, pictures: (String, String), enabled: bool) -> Vec<u8> {
    let mut fields = vec![("TIT2", title.into())];
    if enabled {
        fields.extend([("PICP", pictures.0), ("PIC0", pictures.1)]);
    }
    fields.push(("XSOT", title.into()));
    metadata_row(duration, &fields)
}
fn catalog_tree_descriptors() -> Vec<u8> {
    let mut descriptors = vec![
        (1, 1, 1, "", ""),
        (2, 3, 1, "TPE1", ""),
        (3, 3, 1, "TALB", ""),
        (4, 3, 1, "TCON", ""),
        (0x22, 2, 0, "", ""),
        (0x2d, 3, 2, "TPE1", "TALB"),
    ];
    descriptors.extend(
        (5..45)
            .filter(|id| *id != 0x22)
            .map(|id| (id, 0, 0, "", "")),
    );
    let rows = descriptors
        .into_iter()
        .map(|(id, kind, keys, first, second)| {
            let mut row = vec![0; 80];
            row[0..2].copy_from_slice(&(id as u16).to_be_bytes());
            row[2..4].copy_from_slice(&(kind as u16).to_be_bytes());
            row[16..18].copy_from_slice(&(keys as u16).to_be_bytes());
            row[20..20 + first.len()].copy_from_slice(first.as_bytes());
            row[24..24 + second.len()].copy_from_slice(second.as_bytes());
            if id == 0x2d {
                row[48..60].copy_from_slice(b"TRNOTTCCTTCC");
            }
            row
        })
        .collect();
    let mut system = Section::rows("SYSB", 0x50, vec![vec![0; 0x50]]);
    // Match the observed A918 system profile. The flag word is retained as
    // opaque profile data; its individual bits have not been established.
    system.flags = [0xd1010100, 0];
    let mut trees = Section::rows("GTLB", 0x50, rows);
    trees.flags = [6, 0x04000000];
    document("GTLT", vec![system, trees])
}
pub fn write_database(root: &Path, tracks: &[Track], artwork: bool) -> Result<()> {
    let mut indexed = BTreeMap::new();
    for track in tracks {
        let id = track
            .device_id
            .ok_or_else(|| anyhow::anyhow!("Missing device track ID"))?;
        ensure!(
            id > 0 && id <= 60000,
            "Device track ID outside supported range"
        );
        ensure!(
            indexed.insert(id, track).is_none(),
            "Duplicate device track ID"
        );
    }
    let max = indexed.keys().next_back().copied().unwrap_or(0) as usize;
    let mut contents = vec![vec![0; 0x290]; max];
    for (&id, track) in &indexed {
        let row = &mut contents[id as usize - 1];
        row[2..4].copy_from_slice(&[255, 255]);
        row[4..8].copy_from_slice(&track.properties);
        row[8..12].copy_from_slice(&track.duration_ms.to_be_bytes());
        row[12..16].copy_from_slice(&[0, 5, 0, 128]);
        for (slot, (tag, text)) in [
            ("TIT2", track.title.as_str()),
            ("TPE1", &track.artist),
            ("TALB", &track.album),
            ("TCON", &track.genre),
            ("TSOP", ""),
        ]
        .into_iter()
        .enumerate()
        {
            row[16 + slot * 128..16 + (slot + 1) * 128].copy_from_slice(&fixed_field(tag, text));
        }
    }
    fs::write(
        root.join("04CNTINF.DAT"),
        document("CNIF", vec![Section::rows("CNFB", 0x290, contents)]),
    )?;
    fs::write(
        root.join("05CIDLST.DAT"),
        document(
            "CIDL",
            vec![Section::rows("CILB", 0x30, vec![vec![0; 0x30]; max])],
        ),
    )?;
    let all: Vec<_> = tracks.iter().collect();
    let mut tree_titles = vec![vec![0; 0x90]; 45];
    for index in [0, 1, 2, 3, 33, 44] {
        let name = if index == 44 {
            if artwork {
                "STD_TPE1_THUMBNAIL"
            } else {
                "STD_TPE1"
            }
        } else {
            ""
        };
        tree_titles[index] = metadata_row(
            if index == 33 { 0 } else { duration(&all) },
            &[("TIT2", name.into())],
        );
    }
    fs::write(
        root.join("02TREINF.DAT"),
        document("GTIF", vec![Section::rows("GTFB", 0x90, tree_titles)]),
    )?;
    fs::write(root.join("00GTRLST.DAT"), catalog_tree_descriptors())?;
    // Build four flat views. Album groups use a compound owner/title key.
    for view in 0..4 {
        let mut buckets: BTreeMap<(String, String), Vec<&Track>> = BTreeMap::new();
        for track in tracks {
            let key = match view {
                0 => (track.album_artist_or_artist().into(), track.album.clone()),
                1 => (track.artist.clone(), String::new()),
                2 => (track.album.clone(), String::new()),
                _ => (track.genre.clone(), String::new()),
            };
            buckets.entry(key).or_default().push(track);
        }
        let mut tree = Tree::default();
        let mut records = vec![];
        for ((first, second), mut members) in buckets {
            members.sort_by_key(|track| {
                (
                    &track.artist,
                    &track.album,
                    track.disc,
                    track.track,
                    &track.title,
                )
            });
            let object = records.len() + 1;
            tree.groups.push((object, 1, tree.tracks.len() + 1));
            tree.tracks
                .extend(members.iter().map(|track| track.device_id.unwrap()));
            let fields = if view == 0 {
                let (primary, extra) = images(root, &members, 1, object, artwork)?;
                vec![
                    ("TIT2", second),
                    ("TPE1", first),
                    ("TCON", members[0].genre.clone()),
                    ("TSOP", String::new()),
                    ("PICP", primary),
                    ("PIC0", extra),
                ]
            } else {
                vec![("TIT2", first)]
            };
            records.push(metadata_row(duration(&members), &fields));
        }
        fs::write(
            root.join(format!("01TREE{:02X}.DAT", view + 1)),
            tree.encode()?,
        )?;
        fs::write(
            root.join(format!("03GINF{:02X}.DAT", view + 1)),
            group_file(records, if view == 0 { 0x310 } else { 0x90 }),
        )?;
    }
    fs::write(root.join("01TREE22.DAT"), Tree::default().encode()?)?;
    fs::write(root.join("03GINF22.DAT"), group_file(vec![], 0x310))?;
    let mut owners: BTreeMap<&str, BTreeMap<&str, Vec<&Track>>> = BTreeMap::new();
    for track in tracks {
        owners
            .entry(&track.artist)
            .or_default()
            .entry(&track.album)
            .or_default()
            .push(track);
    }
    let mut hierarchy = Tree::default();
    hierarchy.groups.push((1, 1, 0));
    let empty = || (String::new(), String::new());
    let mut records = vec![hierarchy_row("", 0, empty(), artwork)];
    for (artist, albums) in owners {
        hierarchy.groups.push((records.len() + 1, 1, 0));
        records.push(hierarchy_row(artist, 0, empty(), artwork));
        for (album, mut members) in albums {
            members.sort_by_key(|track| (track.disc, track.track, &track.title));
            let object = records.len() + 1;
            hierarchy
                .groups
                .push((object, 2, hierarchy.tracks.len() + 1));
            hierarchy
                .tracks
                .extend(members.iter().map(|track| track.device_id.unwrap()));
            records.push(hierarchy_row(
                album,
                duration(&members),
                images(root, &members, 0x2d, object, artwork)?,
                artwork,
            ));
        }
    }
    fs::write(root.join("01TREE2D.DAT"), hierarchy.encode()?)?;
    fs::write(
        root.join("03GINF2D.DAT"),
        group_file(records, if artwork { 0x210 } else { 0x110 }),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn global_system_header_matches_the_accepted_a918_profile() {
        // First 64 bytes of the accepted, firmware-normalized global table.
        // Only protocol header bytes are used; no Sony executable is included.
        let observed = "47544c540101000002000000000000005359534200000030000000600000000047544c420000009000000e20000000005359534200010050d101010000000000";
        let expected: Vec<u8> = observed
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        let actual = catalog_tree_descriptors();
        assert_eq!(&actual[..expected.len()], expected);
        assert_eq!(actual.len(), 3760);
    }
}
