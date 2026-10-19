use anyhow::{Context, Result, bail, ensure};
use image::{DynamicImage, ImageDecoder, ImageFormat, Limits, RgbaImage};
use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

pub const MAX_BYTES: u64 = 100 * 1024 * 1024;
const MAX_SIDE: u32 = 12_000;
const MAX_PIXELS: u64 = 24_000_000;

pub struct ImportedImage {
    pub pixels: RgbaImage,
    pub filename: String,
}

pub fn open(path: &Path) -> Result<ImportedImage> {
    let file = File::open(path)
        .context("Could not read this file. Check its location and permissions.")?;
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file(),
        "Choose a PNG or JPEG file, rather than a folder."
    );
    ensure!(
        metadata.len() <= MAX_BYTES,
        "This file exceeds 100 MiB. Choose a smaller image."
    );
    // Bound reads too, in case the file grows during import.
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= MAX_BYTES,
        "This file exceeds 100 MiB. Choose a smaller image."
    );
    let pixels = decode(&bytes).context("Could not import image")?;
    Ok(ImportedImage {
        pixels,
        filename: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
    })
}

fn decode(bytes: &[u8]) -> Result<RgbaImage> {
    let reader = BufReader::new(std::io::Cursor::new(bytes));
    match image::guess_format(bytes).ok() {
        Some(ImageFormat::Png) => {
            let decoder = image::codecs::png::PngDecoder::with_limits(reader, limits())
                .context("The PNG is damaged or exceeds supported limits. Try another file.")?;
            ensure!(
                !decoder.is_apng()?,
                "Animated PNG is not supported. Export a single frame as PNG."
            );
            decode_pixels(decoder)
        }
        Some(ImageFormat::Jpeg) => decode_pixels(
            image::codecs::jpeg::JpegDecoder::new(reader)
                .context("The JPEG is damaged. Try exporting it again.")?,
        ),
        _ => bail!("Unsupported or damaged image. Choose a static PNG or JPEG."),
    }
}

fn limits() -> Limits {
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    limits.max_alloc = Some(256 * 1024 * 1024);
    limits
}

fn decode_pixels(mut decoder: impl ImageDecoder) -> Result<RgbaImage> {
    let (width, height) = decoder.dimensions();
    ensure!(
        width > 0
            && height > 0
            && width <= MAX_SIDE
            && height <= MAX_SIDE
            && u64::from(width) * u64::from(height) <= MAX_PIXELS,
        "Image is too large. Use at most 24 megapixels and 12,000 pixels per side."
    );
    ensure!(
        decoder.color_type().bits_per_pixel() / u16::from(decoder.color_type().channel_count())
            <= 8,
        "Only 8-bit images are supported. Export an 8-bit PNG or JPEG."
    );
    decoder.set_limits(limits())?;
    // No color management in phase 1: never guess how a profile should render.
    ensure!(
        decoder.icc_profile()?.is_none(),
        "Embedded color profiles are not supported in phase 1. Export an sRGB PNG or JPEG without an embedded ICC profile."
    );
    let orientation = decoder.orientation()?;
    let mut image = DynamicImage::from_decoder(decoder)
        .context("The image data is damaged. Try exporting it again.")?;
    image.apply_orientation(orientation);
    Ok(image.into_rgba8())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageEncoder, Rgba};

    #[test]
    fn transparent_png_keeps_pixels() {
        let original = RgbaImage::from_fn(3, 2, |x, y| Rgba([x as u8 * 80, y as u8 * 90, 40, 127]));
        let mut bytes = Vec::new();
        image::codecs::png::PngEncoder::new(&mut bytes)
            .write_image(original.as_raw(), 3, 2, image::ExtendedColorType::Rgba8)
            .unwrap();
        assert_eq!(decode(&bytes).unwrap(), original);
    }

    #[test]
    fn corrupt_and_unsupported_files_fail() {
        assert!(decode(b"not an image").is_err());
        assert!(decode(b"\x89PNG\r\n\x1a\ntruncated").is_err());
        assert!(decode(b"GIF89a").is_err());
    }

    #[test]
    fn jpeg_exif_orientation_is_applied_once() {
        let original =
            image::RgbImage::from_fn(3, 2, |x, y| image::Rgb([x as u8 * 80, y as u8 * 90, 40]));
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 100)
            .encode_image(&original)
            .unwrap();
        let expected = image::imageops::rotate90(&decode(&jpeg).unwrap());
        // Little-endian TIFF IFD with EXIF orientation 6 (90 degrees clockwise).
        let exif = b"Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0";
        let mut oriented = jpeg[..2].to_vec();
        oriented.extend_from_slice(&[0xff, 0xe1]);
        oriented.extend_from_slice(&((exif.len() + 2) as u16).to_be_bytes());
        oriented.extend_from_slice(exif);
        oriented.extend_from_slice(&jpeg[2..]);
        assert_eq!(decode(&oriented).unwrap(), expected);
    }

    #[test]
    fn dimensions_and_precision_are_bounded() {
        for (width, height) in [(12_001, 1), (5000, 5000)] {
            let mut bytes = Vec::new();
            image::codecs::png::PngEncoder::new(&mut bytes)
                .write_image(
                    &vec![0; width as usize * height as usize],
                    width,
                    height,
                    image::ExtendedColorType::L8,
                )
                .unwrap();
            assert!(decode(&bytes).is_err());
        }
        let mut bytes = Vec::new();
        image::codecs::png::PngEncoder::new(&mut bytes)
            .write_image(&[0, 0], 1, 1, image::ExtendedColorType::L16)
            .unwrap();
        assert!(decode(&bytes).unwrap_err().to_string().contains("8-bit"));
    }

    #[test]
    fn animated_png_is_rejected() {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 1, 1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_animated(2, 0).unwrap();
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&[255, 0, 0, 255]).unwrap();
            writer.write_image_data(&[0, 255, 0, 255]).unwrap();
            writer.finish().unwrap();
        }
        assert!(decode(&bytes).unwrap_err().to_string().contains("Animated"));
    }

    #[test]
    fn embedded_profiles_are_rejected_explicitly() {
        let mut bytes = Vec::new();
        let mut encoder = image::codecs::png::PngEncoder::new(&mut bytes);
        encoder.set_icc_profile(vec![42; 128]).unwrap();
        encoder
            .write_image(&[10, 20, 30, 255], 1, 1, image::ExtendedColorType::Rgba8)
            .unwrap();
        assert!(
            decode(&bytes)
                .unwrap_err()
                .to_string()
                .contains("color profiles")
        );
    }

    #[test]
    fn import_handles_unicode_paths_without_modifying_source() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("写真.png");
        let mut bytes = Vec::new();
        image::codecs::png::PngEncoder::new(&mut bytes)
            .write_image(&[10, 20, 30, 40], 1, 1, image::ExtendedColorType::Rgba8)
            .unwrap();
        std::fs::write(&path, &bytes).unwrap();
        let imported = open(&path).unwrap();
        assert_eq!(imported.filename, "写真.png");
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        let oversized = directory.path().join("oversized.png");
        File::create(&oversized)
            .unwrap()
            .set_len(MAX_BYTES + 1)
            .unwrap();
        assert!(open(&oversized).is_err());
    }
}
