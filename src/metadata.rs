use anyhow::{Context, Result, bail};
use id3::{Tag, TagLike};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::Path,
};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Track {
    pub key: String,
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub genre: String,
    pub year: i32,
    pub track: u32,
    pub track_total: u32,
    pub disc: u32,
    pub disc_total: u32,
    pub duration_ms: u32,
    pub bytes: u64,
    pub bitrate: u32,
    pub sample_rate: u32,
    pub channels: u8,
    pub frames: u32,
    pub audio_offset: u64,
    pub properties: [u8; 4],
    pub cover: Option<String>,
    pub tag_version: String,
    pub warnings: Vec<String>,
    pub device_id: Option<u16>,
}

pub fn hash_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = [0; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}

pub fn read_mp3(path: &Path, cache: &Path) -> Result<Track> {
    let path = path.canonicalize()?;
    let key = hash_file(&path)?;
    let mut t = Track {
        key: key.clone(),
        path: path.to_string_lossy().into(),
        title: path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into(),
        bytes: path.metadata()?.len(),
        ..Default::default()
    };
    let mut has_v2_title = false;
    let tag = match Tag::read_from_path(&path) {
        Ok(tag) => Some(tag),
        Err(e) => {
            if !matches!(e.kind, id3::ErrorKind::NoTag) {
                t.warnings.push(format!(
                    "Incomplete ID3 read: {e}; correct tags in the editor"
                ));
            }
            e.partial_tag
        }
    };
    match tag {
        Some(tag) => {
            t.tag_version = format!("{:?}", tag.version());
            let title = display_text(&tag, "TIT2");
            has_v2_title = !title.is_empty();
            if has_v2_title {
                t.title = title;
            }
            t.artist = display_text(&tag, "TPE1");
            t.album = display_text(&tag, "TALB");
            t.album_artist = display_text(&tag, "TPE2");
            t.genre = tag
                .genres_parsed()
                .iter()
                .map(|s| s.as_ref())
                .collect::<Vec<_>>()
                .join(" / ");
            t.year = tag
                .year()
                .filter(|y| *y > 0)
                .or_else(|| tag.date_recorded().map(|d| d.year).filter(|y| *y > 0))
                .unwrap_or(0);
            t.track = tag.track().unwrap_or(0);
            t.track_total = tag.total_tracks().unwrap_or(0);
            t.disc = tag.disc().unwrap_or(0);
            t.disc_total = tag.total_discs().unwrap_or(0);
            if let Some(pic) = tag
                .pictures()
                .find(|p| p.picture_type == id3::frame::PictureType::CoverFront)
                .or_else(|| tag.pictures().next())
            {
                match save_cover(&pic.data, cache, &key) {
                    Ok(p) => t.cover = Some(p.to_string_lossy().into()),
                    Err(e) => t.warnings.push(format!("Artwork read failed: {e}")),
                }
            }
        }
        None => {}
    }
    if let Ok(legacy) = id3::v1::Tag::read_from_path(&path) {
        // ID3v1 is ISO-8859-1. Do not guess GBK/Shift-JIS for legacy bytes.
        if [
            &legacy.title,
            &legacy.artist,
            &legacy.album,
            &legacy.comment,
        ]
        .iter()
        .any(|s| !s.is_ascii())
        {
            t.warnings.push("Non-ASCII ID3v1 text uses ISO-8859-1. Correct garbled text in the editor; GBK and Shift-JIS are not guessed automatically.".into());
        }
        if !has_v2_title && !legacy.title.trim().is_empty() {
            t.title = legacy.title.trim_end().into();
        }
        if t.artist.is_empty() {
            t.artist = legacy.artist.trim_end().into();
        }
        if t.album.is_empty() {
            t.album = legacy.album.trim_end().into();
        }
        if t.genre.is_empty() {
            t.genre = legacy.genre().unwrap_or("").trim_end().into();
        }
        if t.year <= 0 {
            t.year = legacy
                .year
                .trim()
                .parse::<i32>()
                .ok()
                .filter(|y| *y > 0)
                .unwrap_or(0);
        }
        if t.track == 0 {
            t.track = legacy.track.unwrap_or(0) as u32;
        }
        if t.tag_version.is_empty() {
            t.tag_version = if legacy.track.is_some() {
                "Id3v11"
            } else {
                "Id3v1"
            }
            .into();
        } else {
            t.tag_version.push_str(" + Id3v1 fallback");
        }
    }
    let mut f = File::open(&path)?;
    let mut h = [0; 10];
    f.read_exact(&mut h)?;
    let mut pos = if &h[..3] == b"ID3" {
        // ID3v2.4 section 3.1: size excludes both the header and optional footer.
        // https://id3.org/id3v2.4.0-structure
        10 + synchsafe(&h[6..10]) as u64 + if h[3] == 4 && h[5] & 0x10 != 0 { 10 } else { 0 }
    } else {
        0
    };
    let end = t.bytes;
    let mut first = true;
    let mut rate_sum = 0u64;
    let mut samples = 0u64;
    let mut first_rate = 0;
    let mut vbr = false;
    let mut search = 0;
    while pos + 4 <= end {
        f.seek(SeekFrom::Start(pos))?;
        let mut b = [0; 4];
        f.read_exact(&mut b)?;
        if let Some((length, rate, bitrate, spf, channels, prop)) = mp3_header(b) {
            if pos + length as u64 > end {
                break;
            }
            if first {
                t.audio_offset = pos;
                t.sample_rate = rate;
                t.channels = channels;
                t.properties = prop;
                first_rate = bitrate;
                first = false;
            }
            if rate != t.sample_rate {
                bail!("MP3 sample rate changes within the file; direct wrapping is unsupported")
            }
            vbr |= bitrate != first_rate;
            rate_sum += bitrate as u64;
            samples += spf as u64;
            t.frames += 1;
            pos += length as u64;
        } else if first && search < 1024 * 1024 {
            pos += 1;
            search += 1;
        } else {
            break;
        }
    }
    if first || t.frames < 2 {
        bail!("No valid MP3 audio frame: {}", path.display())
    }
    t.duration_ms = (samples * 1000 / t.sample_rate as u64)
        .try_into()
        .context("Audio is too long")?;
    t.bitrate = (rate_sum / t.frames as u64) as u32;
    t.properties[1] = if vbr { 0x90 } else { 0x80 };
    for (name, value) in [
        ("Title", &t.title),
        ("Artist", &t.artist),
        ("Album", &t.album),
        ("Genre", &t.genre),
    ] {
        if value.encode_utf16().count() > 60 {
            t.warnings.push(format!(
                "{name} exceeds 60 UTF-16 code units; full text is retained locally and in OMA tags"
            ));
        }
    }
    Ok(t)
}
fn display_text(tag: &Tag, id: &str) -> String {
    tag.text_values_for_frame_id(id)
        .unwrap_or_default()
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" / ")
}
pub fn synchsafe(b: &[u8]) -> u32 {
    b.iter().fold(0, |n, v| (n << 7) | (*v as u32 & 127))
}
pub fn mp3_header(b: [u8; 4]) -> Option<(u32, u32, u32, u32, u8, [u8; 4])> {
    if b[0] != 255 || b[1] & 0xe0 != 0xe0 {
        return None;
    }
    let version = (b[1] >> 3) & 3;
    let layer = (b[1] >> 1) & 3;
    let bi = b[2] >> 4;
    let si = (b[2] >> 2) & 3;
    if version == 1 || layer != 1 || bi == 0 || bi == 15 || si == 3 {
        return None;
    }
    let br = if version == 3 {
        [
            0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0,
        ][bi as usize]
    } else {
        [
            0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0,
        ][bi as usize]
    };
    let rate = [44100, 48000, 32000][si as usize]
        / if version == 3 {
            1
        } else if version == 2 {
            2
        } else {
            4
        };
    let len = if version == 3 { 144000 } else { 72000 } * br / rate + ((b[2] >> 1) & 1) as u32;
    let ch = if b[3] >> 6 == 3 { 1 } else { 2 };
    Some((
        len,
        rate,
        br,
        if version == 3 { 1152 } else { 576 },
        ch,
        [
            3,
            0x80,
            (version << 6) | (layer << 4) | bi,
            if ch == 1 { 0x30 } else { 0x10 },
        ],
    ))
}
pub fn save_cover(data: &[u8], cache: &Path, key: &str) -> Result<std::path::PathBuf> {
    anyhow::ensure!(
        key.len() == 64 && key.bytes().all(|c| c.is_ascii_hexdigit()),
        "Invalid artwork cache key"
    );
    let mut reader = image::ImageReader::new(std::io::Cursor::new(data)).with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(12000);
    limits.max_image_height = Some(12000);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let img = reader.decode()?.to_rgb8();
    fs::create_dir_all(cache)?;
    let target = cache.join(format!("{key}.jpg"));
    let thumb = image::imageops::thumbnail(&img, 600, 600);
    let mut encoded = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded, 90).encode_image(&thumb)?;
    // Encode before replacing the cache so decoder/encoder failures preserve it.
    fs::write(&target, encoded)?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn audio() -> Vec<u8> {
        (0..3)
            .flat_map(|_| {
                let mut f = vec![255, 251, 144, 0];
                f.extend([0; 413]);
                f
            })
            .collect()
    }
    #[test]
    fn legacy_fallback_latin1_and_v2_priority() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("legacy.mp3");
        let mut bytes = audio();
        let mut old = [0; 128];
        old[..3].copy_from_slice(b"TAG");
        old[3..7].copy_from_slice(b"caf\xe9");
        old[33..39].copy_from_slice(b"Artist");
        old[63..68].copy_from_slice(b"Album");
        old[93..97].copy_from_slice(b"1999");
        old[126] = 17;
        old[127] = 17;
        bytes.extend(old);
        fs::write(&p, &bytes).unwrap();
        let t = read_mp3(&p, d.path()).unwrap();
        assert_eq!(t.title, "café");
        assert_eq!(t.track, 17);
        assert_eq!(t.year, 1999);
        assert_eq!(t.genre, "Rock");
        assert!(t.warnings.iter().any(|w| w.contains("ISO-8859-1")));
        assert_eq!(fs::read(&p).unwrap(), bytes);
        let mut v2 = Tag::new();
        v2.set_title("Unicode 标题");
        v2.set_artist("New Artist");
        v2.set_year(2025);
        v2.write_to_path(&p, id3::Version::Id3v24).unwrap();
        let before = fs::read(&p).unwrap();
        let t = read_mp3(&p, d.path()).unwrap();
        assert_eq!(t.title, "Unicode 标题");
        assert_eq!(t.artist, "New Artist");
        assert_eq!(t.album, "Album");
        assert_eq!(t.year, 2025);
        assert_eq!(t.track, 17);
        assert_eq!(before, fs::read(&p).unwrap());
    }
    #[test]
    fn footer_does_not_skip_first_audio_frame() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("footer.mp3");
        let mut bytes = b"ID3\x04\0\x10\0\0\0\0".to_vec();
        bytes.extend(b"3DI\x04\0\x10\0\0\0\0");
        bytes.extend(audio());
        fs::write(&p, &bytes).unwrap();
        let t = read_mp3(&p, d.path()).unwrap();
        assert_eq!(t.audio_offset, 20);
        assert_eq!(t.frames, 3);
        assert_eq!(fs::read(&p).unwrap(), bytes);
    }
    #[test]
    fn multivalue_year_zero_and_front_picture_priority() {
        use id3::frame::{Picture, PictureType};
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("pictures.mp3");
        fs::write(&p, audio()).unwrap();
        let mut tag = Tag::new();
        let artist = "艺术家🎵".repeat(30);
        tag.set_text_values("TPE1", [artist.as_str(), "Second artist"]);
        tag.set_text_values("TCON", ["Rock", "Pop"]);
        tag.set_year(0);
        tag.set_date_recorded("2003-09-23".parse::<id3::Timestamp>().unwrap());
        for (kind, color) in [
            (PictureType::CoverBack, [0, 0, 255]),
            (PictureType::CoverFront, [255, 0, 0]),
        ] {
            let img = image::RgbImage::from_pixel(8, 8, image::Rgb(color));
            let mut data = std::io::Cursor::new(Vec::new());
            img.write_to(&mut data, image::ImageFormat::Png).unwrap();
            tag.add_frame(Picture {
                mime_type: "image/png".into(),
                picture_type: kind,
                description: format!("{kind:?}"),
                data: data.into_inner(),
            });
        }
        tag.write_to_path(&p, id3::Version::Id3v24).unwrap();
        let before = fs::read(&p).unwrap();
        let t = read_mp3(&p, d.path()).unwrap();
        assert_eq!(t.artist, format!("{artist} / Second artist"));
        assert_eq!(t.genre, "Rock / Pop");
        assert_eq!(t.year, 2003);
        let pixel = image::open(t.cover.unwrap())
            .unwrap()
            .to_rgb8()
            .get_pixel(0, 0)
            .0;
        assert!(pixel[0] > 240 && pixel[2] < 10);
        assert_eq!(fs::read(&p).unwrap(), before);
    }
    #[test]
    fn headers() {
        let x = mp3_header([255, 251, 144, 0]).unwrap();
        assert_eq!((x.0, x.1, x.2), (417, 44100, 128));
        assert!(mp3_header([0, 0, 0, 0]).is_none());
    }
    #[test]
    fn id3_versions_unicode_and_numbers() {
        use id3::Version;
        let d = tempfile::tempdir().unwrap();
        for v in [Version::Id3v22, Version::Id3v23, Version::Id3v24] {
            let p = d.path().join(format!("{v:?}.mp3"));
            let mut audio = vec![];
            for _ in 0..10 {
                audio.extend([255, 251, 144, 0]);
                audio.extend(vec![0; 413]);
            }
            fs::write(&p, audio).unwrap();
            let mut tag = Tag::new();
            tag.set_title("中文 🎵 café");
            tag.set_artist("甲 / 乙");
            tag.set_album("长专辑");
            tag.set_album_artist("群星");
            tag.set_track(123);
            tag.set_total_tracks(200);
            tag.set_disc(2);
            tag.set_total_discs(3);
            tag.set_year(2024);
            tag.write_to_path(&p, v).unwrap();
            let before = fs::read(&p).unwrap();
            let t = read_mp3(&p, d.path()).unwrap();
            assert_eq!(t.title, "中文 🎵 café");
            assert_eq!(t.track, 123);
            assert_eq!(t.track_total, 200);
            assert_eq!(t.disc, 2);
            assert_eq!(t.year, 2024);
            assert_eq!(
                before,
                fs::read(&p).unwrap(),
                "metadata import must never rewrite source tags"
            );
        }
    }
    #[test]
    fn recorded_date_and_cover_cache_paths() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("date.mp3");
        let mut audio = Vec::new();
        for _ in 0..3 {
            audio.extend([255, 251, 144, 0]);
            audio.extend([0; 413]);
        }
        fs::write(&p, audio).unwrap();
        let mut tag = Tag::new();
        tag.set_date_recorded("2003-09-23T12:34:56".parse::<id3::Timestamp>().unwrap());
        tag.write_to_path(&p, id3::Version::Id3v24).unwrap();
        assert_eq!(read_mp3(&p, d.path()).unwrap().year, 2003);
        let error = save_cover(&[], d.path(), "../outside").unwrap_err();
        assert!(error.to_string().contains("cache key"));
    }
}
