//! Pictures as files: PNG and JPEG XL.

use jxl_sys as jxl;
use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::ptr;

// jxl-sys names the C++ library for Linux and macOS only.
#[cfg(target_os = "freebsd")]
#[link(name = "c++")]
unsafe extern "C" {}

/// A picture, or one frame of a moving one.
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    /// 1 (gray) or 3 (red, green, blue).
    pub colors: usize,
    pub alpha: bool,
    /// Samples from 0 to 1: rows from the top, the channels of a pixel one after the other.
    pub data: Vec<f32>,
    /// How far every pixel is, from 0 (nearest) to 1: an extra channel of a JPEG XL file.
    pub depth: Option<Vec<f32>>,
}

impl Image {
    pub fn channels(&self) -> usize {
        self.colors + usize::from(self.alpha)
    }

    fn fits(&self, first: &Image) -> bool {
        self.width == first.width
            && self.height == first.height
            && self.colors == first.colors
            && self.alpha == first.alpha
            && self.depth.is_some() == first.depth.is_some()
    }

    fn sound(&self) -> bool {
        let pixels = self.width * self.height;
        matches!(self.colors, 1 | 3)
            && pixels > 0
            && self.data.len() == pixels * self.channels()
            && self.depth.as_ref().is_none_or(|d| d.len() == pixels)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Png,
    Jxl,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Encoding {
    pub format: Format,
    /// Bits of a sample: 8 or 16 in a PNG file, 1 to 16 in a JPEG XL file.
    pub bits: u32,
    /// PNG: the level of the compression, 0 to 9.
    pub level: u8,
    /// JPEG XL: the distance of a lossy picture from the true one; lossless without it.
    pub distance: Option<f32>,
    /// JPEG XL: the effort of the encoder, 1 to 10.
    pub effort: u8,
    /// Frames a second, as a fraction.
    pub rate: (u32, u32),
}

/// The frames of a picture, one after the other.
pub type Frames<'a> = &'a mut dyn FnMut(usize) -> Result<Image, String>;

/// A sample as a whole number from 0 to `max`.
pub fn step(sample: f32, max: u32) -> u32 {
    let scaled = f64::from(sample) * f64::from(max) + 0.5;
    if scaled >= f64::from(max) {
        max
    } else {
        scaled as u32
    }
}

/// The distance of JPEG XL for a quality from 0 to 100 (90 is the distance 1).
pub fn distance_of(quality: f32) -> f32 {
    unsafe { jxl::JxlEncoderDistanceFromQuality(quality) }
}

/// The bytes of the file with `count` frames.
pub fn encode(count: usize, frames: Frames, e: &Encoding) -> Result<Vec<u8>, String> {
    if count == 0 {
        return Err("a picture needs a frame".to_string());
    }
    match e.format {
        Format::Png => png_file(count, frames, e),
        Format::Jxl => jxl_file(count, frames, e, workers()),
    }
}

/// The samples of a picture that was read: whole numbers of `bits` bits, rows from the top,
/// the channels of a pixel one after the other.
pub struct Pixels {
    pub width: usize,
    pub height: usize,
    pub channels: usize,
    pub bits: u32,
    pub samples: Vec<u32>,
}

/// The first frame of a PNG or a JPEG XL file.
pub fn read(path: &str) -> Result<Pixels, String> {
    crate::sandbox::allowed(path)?;
    let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let jxl_box = [0, 0, 0, 0x0c, b'J', b'X', b'L', b' '];
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        png_pixels(&bytes).map_err(|e| format!("{path}: {e}"))
    } else if bytes.starts_with(&[0xff, 0x0a]) || bytes.starts_with(&jxl_box) {
        jxl_pixels(&bytes).map_err(|e| format!("{path}: {e}"))
    } else {
        Err(format!("{path} is neither a PNG nor a JPEG XL file"))
    }
}

fn png_pixels(bytes: &[u8]) -> Result<Pixels, String> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let size = reader
        .output_buffer_size()
        .ok_or("the PNG file is too large")?;
    let mut buffer = vec![0; size];
    let info = reader.next_frame(&mut buffer).map_err(|e| e.to_string())?;
    let data = &buffer[..info.buffer_size()];
    let channels = match info.color_type {
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::Indexed => {
            return Err("a PNG file with a table of colours that was not expanded".into());
        }
    };
    let (bits, samples) = match info.bit_depth {
        png::BitDepth::Sixteen => (
            16,
            data.as_chunks::<2>()
                .0
                .iter()
                .map(|p| u32::from(u16::from_be_bytes(*p)))
                .collect(),
        ),
        _ => (8, data.iter().map(|b| u32::from(*b)).collect()),
    };
    Ok(Pixels {
        width: info.width as usize,
        height: info.height as usize,
        channels,
        bits,
        samples,
    })
}

struct Decoder(*mut jxl::JxlDecoder);

impl Drop for Decoder {
    fn drop(&mut self) {
        unsafe { jxl::JxlDecoderDestroy(self.0) }
    }
}

fn jxl_pixels(bytes: &[u8]) -> Result<Pixels, String> {
    use jxl::JxlDecoderStatus as Status;
    let failed = || "the JPEG XL file cannot be read".to_string();
    unsafe {
        let decoder = Decoder(jxl::JxlDecoderCreate(ptr::null()));
        if decoder.0.is_null() {
            return Err(failed());
        }
        let wanted = Status::JXL_DEC_BASIC_INFO as i32 | Status::JXL_DEC_FULL_IMAGE as i32;
        if jxl::JxlDecoderSubscribeEvents(decoder.0, wanted) != Status::JXL_DEC_SUCCESS
            || jxl::JxlDecoderSetInput(decoder.0, bytes.as_ptr(), bytes.len())
                != Status::JXL_DEC_SUCCESS
        {
            return Err(failed());
        }
        jxl::JxlDecoderCloseInput(decoder.0);
        let mut shape: Option<(usize, usize, usize, u32)> = None;
        let mut buffer: Vec<u16> = Vec::new();
        let mut format = jxl::JxlPixelFormat {
            num_channels: 1,
            data_type: jxl::JxlDataType::JXL_TYPE_UINT16,
            endianness: jxl::JxlEndianness::JXL_NATIVE_ENDIAN,
            align: 0,
        };
        loop {
            match jxl::JxlDecoderProcessInput(decoder.0) {
                Status::JXL_DEC_BASIC_INFO => {
                    let mut info = MaybeUninit::<jxl::JxlBasicInfo>::zeroed();
                    if jxl::JxlDecoderGetBasicInfo(decoder.0, info.as_mut_ptr())
                        != Status::JXL_DEC_SUCCESS
                    {
                        return Err(failed());
                    }
                    let info = info.assume_init();
                    let channels = info.num_color_channels + u32::from(info.alpha_bits > 0);
                    format.num_channels = channels;
                    shape = Some((
                        info.xsize as usize,
                        info.ysize as usize,
                        channels as usize,
                        info.bits_per_sample.clamp(1, 16),
                    ));
                }
                Status::JXL_DEC_NEED_IMAGE_OUT_BUFFER => {
                    let mut size = 0usize;
                    if jxl::JxlDecoderImageOutBufferSize(decoder.0, &format, &mut size)
                        != Status::JXL_DEC_SUCCESS
                    {
                        return Err(failed());
                    }
                    buffer = vec![0u16; size.div_ceil(2)];
                    if jxl::JxlDecoderSetImageOutBuffer(
                        decoder.0,
                        &format,
                        buffer.as_mut_ptr().cast::<c_void>(),
                        size,
                    ) != Status::JXL_DEC_SUCCESS
                    {
                        return Err(failed());
                    }
                }
                Status::JXL_DEC_FULL_IMAGE | Status::JXL_DEC_SUCCESS => break,
                _ => return Err(failed()),
            }
        }
        let (width, height, channels, bits) = shape.ok_or_else(failed)?;
        if buffer.len() < width * height * channels {
            return Err(failed());
        }
        // (the decoder spreads every sample over 16 bits: back to the bits of the file)
        let top = u64::from((1u32 << bits) - 1);
        let samples = buffer[..width * height * channels]
            .iter()
            .map(|v| ((u64::from(*v) * top + 32767) / 65535) as u32)
            .collect();
        Ok(Pixels {
            width,
            height,
            channels,
            bits,
            samples,
        })
    }
}

/// Writes the file whole or not at all.
pub fn write(path: &str, bytes: &[u8]) -> Result<(), String> {
    crate::sandbox::allowed(path)?;
    crate::sandbox::spend(bytes.len())?;
    let target = std::path::Path::new(path);
    let name = target
        .file_name()
        .ok_or_else(|| format!("{path} is no name of a file"))?;
    let mut part = name.to_os_string();
    part.push(format!(".{}.part", std::process::id()));
    let part = target.with_file_name(part);
    std::fs::write(&part, bytes)
        .and_then(|()| std::fs::rename(&part, target))
        .map_err(|failure| {
            let _ = std::fs::remove_file(&part);
            format!("cannot write {path}: {failure}")
        })
}

fn next(k: usize, frames: Frames, first: Option<&Image>) -> Result<Image, String> {
    let image = frames(k)?;
    if !image.sound() || first.is_some_and(|f| !image.fits(f)) {
        return Err("the frames of a picture must have one size and the same channels".into());
    }
    Ok(image)
}

fn png_file(count: usize, frames: Frames, e: &Encoding) -> Result<Vec<u8>, String> {
    if !matches!(e.bits, 8 | 16) {
        return Err(format!(
            "a PNG file has 8 or 16 bits a sample, not {}",
            e.bits
        ));
    }
    let failed = |failure: png::EncodingError| format!("PNG: {failure}");
    let first = next(0, frames, None)?;
    if first.depth.is_some() {
        return Err("a PNG file has no channel for the depth: save_jxl has".to_string());
    }
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, first.width as u32, first.height as u32);
        encoder.set_color(match (first.colors, first.alpha) {
            (1, false) => png::ColorType::Grayscale,
            (1, true) => png::ColorType::GrayscaleAlpha,
            (_, false) => png::ColorType::Rgb,
            (_, true) => png::ColorType::Rgba,
        });
        encoder.set_depth(if e.bits == 16 {
            png::BitDepth::Sixteen
        } else {
            png::BitDepth::Eight
        });
        encoder.set_deflate_compression(match e.level {
            0 => png::DeflateCompression::NoCompression,
            level => png::DeflateCompression::Level(level.min(9)),
        });
        encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
        if count > 1 {
            let (frames_a_second, seconds) = (e.rate.0.min(65535), e.rate.1.min(65535));
            encoder.set_animated(count as u32, 0).map_err(failed)?;
            encoder
                .set_frame_delay(seconds as u16, frames_a_second as u16)
                .map_err(failed)?;
        }
        let mut writer = encoder.write_header().map_err(failed)?;
        let max = (1u32 << e.bits) - 1;
        let mut image = Some(first.clone());
        for k in 0..count {
            let frame = match image.take() {
                Some(frame) => frame,
                None => next(k, frames, Some(&first))?,
            };
            let mut bytes = Vec::with_capacity(frame.data.len() * (e.bits as usize / 8));
            for sample in &frame.data {
                let level = step(*sample, max);
                if e.bits == 16 {
                    bytes.extend_from_slice(&(level as u16).to_be_bytes());
                } else {
                    bytes.push(level as u8);
                }
            }
            writer.write_image_data(&bytes).map_err(failed)?;
        }
        writer.finish().map_err(failed)?;
    }
    Ok(out)
}

/// The threads of the JPEG XL encoder next to the one that calls it.
fn workers() -> usize {
    std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .clamp(1, 8)
        - 1
}

struct Jxl {
    encoder: *mut jxl::JxlEncoder,
    runner: *mut c_void,
}

impl Drop for Jxl {
    fn drop(&mut self) {
        unsafe {
            jxl::JxlEncoderDestroy(self.encoder);
            if !self.runner.is_null() {
                jxl::JxlThreadParallelRunnerDestroy(self.runner);
            }
        }
    }
}

impl Jxl {
    fn check(&self, status: jxl::JxlEncoderStatus, what: &str) -> Result<(), String> {
        if status == jxl::JxlEncoderStatus::JXL_ENC_SUCCESS {
            return Ok(());
        }
        let reason = unsafe { jxl::JxlEncoderGetError(self.encoder) };
        Err(format!("JPEG XL: {what} failed ({reason:?})"))
    }

    /// Takes what the encoder has ready.
    fn drain(&self, out: &mut Vec<u8>) -> Result<(), String> {
        let mut buffer = vec![0u8; 1 << 16];
        loop {
            let mut at = buffer.as_mut_ptr();
            let mut left = buffer.len();
            let status = unsafe { jxl::JxlEncoderProcessOutput(self.encoder, &mut at, &mut left) };
            out.extend_from_slice(&buffer[..buffer.len() - left]);
            if status != jxl::JxlEncoderStatus::JXL_ENC_NEED_MORE_OUTPUT {
                return self.check(status, "the encoding");
            }
        }
    }
}

/// Samples as the whole numbers of the file, in bytes or in pairs of bytes.
fn levels(samples: &[f32], bits: u32) -> Vec<u8> {
    let max = (1u32 << bits) - 1;
    let mut out = Vec::with_capacity(samples.len() * if bits > 8 { 2 } else { 1 });
    for sample in samples {
        let level = step(*sample, max);
        if bits > 8 {
            out.extend_from_slice(&(level as u16).to_ne_bytes());
        } else {
            out.push(level as u8);
        }
    }
    out
}

fn jxl_file(count: usize, frames: Frames, e: &Encoding, threads: usize) -> Result<Vec<u8>, String> {
    if !(1..=16).contains(&e.bits) {
        return Err(format!(
            "a JPEG XL file of this program has 1 to 16 bits a sample, not {}",
            e.bits
        ));
    }
    let first = next(0, frames, None)?;
    let (width, height) = (
        u32::try_from(first.width).map_err(|_| "the picture is too wide")?,
        u32::try_from(first.height).map_err(|_| "the picture is too high")?,
    );
    let lossless = e.distance.is_none();
    let extra = usize::from(first.alpha) + usize::from(first.depth.is_some());
    let data_type = if e.bits > 8 {
        jxl::JxlDataType::JXL_TYPE_UINT16
    } else {
        jxl::JxlDataType::JXL_TYPE_UINT8
    };
    let format = |channels: usize| jxl::JxlPixelFormat {
        num_channels: channels as u32,
        data_type,
        endianness: jxl::JxlEndianness::JXL_NATIVE_ENDIAN,
        align: 0,
    };
    let mut out = Vec::new();
    unsafe {
        let encoder = jxl::JxlEncoderCreate(ptr::null());
        if encoder.is_null() {
            return Err("JPEG XL: no encoder".to_string());
        }
        let mut j = Jxl {
            encoder,
            runner: ptr::null_mut(),
        };
        if threads > 0 {
            j.runner = jxl::JxlThreadParallelRunnerCreate(ptr::null(), threads);
            if j.runner.is_null() {
                return Err("JPEG XL: no threads".to_string());
            }
            j.check(
                jxl::JxlEncoderSetParallelRunner(
                    encoder,
                    Some(jxl::JxlThreadParallelRunner),
                    j.runner,
                ),
                "the threads",
            )?;
        }
        let mut info = MaybeUninit::uninit();
        jxl::JxlEncoderInitBasicInfo(info.as_mut_ptr());
        let mut info = info.assume_init();
        info.xsize = width;
        info.ysize = height;
        info.bits_per_sample = e.bits;
        info.exponent_bits_per_sample = 0;
        info.num_color_channels = first.colors as u32;
        info.num_extra_channels = extra as u32;
        info.alpha_bits = if first.alpha { e.bits } else { 0 };
        info.uses_original_profile = i32::from(lossless);
        if count > 1 {
            info.have_animation = 1;
            info.animation.tps_numerator = e.rate.0.max(1);
            info.animation.tps_denominator = e.rate.1.max(1);
            info.animation.num_loops = 0;
        }
        j.check(jxl::JxlEncoderSetBasicInfo(encoder, &info), "the layout")?;
        let mut color = MaybeUninit::uninit();
        jxl::JxlColorEncodingSetToSRGB(color.as_mut_ptr(), i32::from(first.colors == 1));
        j.check(
            jxl::JxlEncoderSetColorEncoding(encoder, color.as_ptr()),
            "the colours",
        )?;
        let kinds = first
            .alpha
            .then_some(jxl::JxlExtraChannelType::JXL_CHANNEL_ALPHA)
            .into_iter()
            .chain(
                first
                    .depth
                    .is_some()
                    .then_some(jxl::JxlExtraChannelType::JXL_CHANNEL_DEPTH),
            );
        for (index, kind) in kinds.enumerate() {
            let depth = kind == jxl::JxlExtraChannelType::JXL_CHANNEL_DEPTH;
            let mut channel = MaybeUninit::uninit();
            jxl::JxlEncoderInitExtraChannelInfo(kind, channel.as_mut_ptr());
            let mut channel = channel.assume_init();
            channel.bits_per_sample = e.bits;
            j.check(
                jxl::JxlEncoderSetExtraChannelInfo(encoder, index, &channel),
                "an extra channel",
            )?;
            if depth {
                let name = b"depth";
                j.check(
                    jxl::JxlEncoderSetExtraChannelName(
                        encoder,
                        index,
                        name.as_ptr().cast(),
                        name.len(),
                    ),
                    "the name of a channel",
                )?;
            }
        }
        let settings = jxl::JxlEncoderFrameSettingsCreate(encoder, ptr::null());
        if settings.is_null() {
            return Err("JPEG XL: no settings".to_string());
        }
        j.check(
            jxl::JxlEncoderFrameSettingsSetOption(
                settings,
                jxl::JxlEncoderFrameSettingId::JXL_ENC_FRAME_SETTING_EFFORT,
                i64::from(e.effort.clamp(1, 10)),
            ),
            "the effort",
        )?;
        j.check(
            jxl::JxlEncoderSetFrameLossless(settings, i32::from(lossless)),
            "the kind of compression",
        )?;
        if let Some(distance) = e.distance {
            j.check(
                jxl::JxlEncoderSetFrameDistance(settings, distance),
                "the distance",
            )?;
            // (transparency and depth stay exact in a lossy picture)
            for index in 0..extra {
                j.check(
                    jxl::JxlEncoderSetExtraChannelDistance(settings, index, 0.0),
                    "the distance of a channel",
                )?;
            }
        }
        let depth = jxl::JxlBitDepth {
            type_: jxl::JxlBitDepthType::JXL_BIT_DEPTH_FROM_CODESTREAM,
            ..Default::default()
        };
        j.check(
            jxl::JxlEncoderSetFrameBitDepth(settings, &depth),
            "the bits of a sample",
        )?;
        let mut image = Some(first.clone());
        for k in 0..count {
            let frame = match image.take() {
                Some(frame) => frame,
                None => next(k, frames, Some(&first))?,
            };
            if count > 1 {
                let mut header = MaybeUninit::uninit();
                jxl::JxlEncoderInitFrameHeader(header.as_mut_ptr());
                let mut header = header.assume_init();
                header.duration = 1;
                j.check(
                    jxl::JxlEncoderSetFrameHeader(settings, &header),
                    "the time of a frame",
                )?;
            }
            let pixels = levels(&frame.data, e.bits);
            j.check(
                jxl::JxlEncoderAddImageFrame(
                    settings,
                    &format(frame.channels()),
                    pixels.as_ptr().cast(),
                    pixels.len(),
                ),
                "a frame",
            )?;
            if let Some(far) = &frame.depth {
                let far = levels(far, e.bits);
                j.check(
                    jxl::JxlEncoderSetExtraChannelBuffer(
                        settings,
                        &format(1),
                        far.as_ptr().cast(),
                        far.len(),
                        u32::from(frame.alpha),
                    ),
                    "the depth",
                )?;
            }
            if k + 1 == count {
                jxl::JxlEncoderCloseInput(encoder);
            }
            j.drain(&mut out)?;
        }
    }
    Ok(out)
}

/// What the tests read files with.
#[cfg(test)]
pub(crate) mod testing {
    use super::*;

    pub(crate) fn whole(samples: &[f32], bits: u32) -> Vec<u32> {
        let max = (1u32 << bits) - 1;
        samples.iter().map(|s| step(*s, max)).collect()
    }

    /// The frames of a PNG file as whole numbers, with its shape.
    pub(crate) fn read_png(bytes: &[u8]) -> (png::OutputInfo, Vec<Vec<u32>>) {
        let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
            .read_info()
            .expect("a PNG file");
        let frames = reader
            .info()
            .animation_control()
            .map_or(1, |a| a.num_frames);
        let mut shape = None;
        let mut out = Vec::new();
        for _ in 0..frames {
            let mut buffer = vec![0; reader.output_buffer_size().expect("a size")];
            let info = reader.next_frame(&mut buffer).expect("a frame");
            let bytes = &buffer[..info.buffer_size()];
            out.push(match info.bit_depth {
                png::BitDepth::Sixteen => bytes
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|p| u32::from(u16::from_be_bytes(*p)))
                    .collect(),
                _ => bytes.iter().map(|b| u32::from(*b)).collect(),
            });
            shape = Some(info);
        }
        (shape.expect("a frame"), out)
    }

    /// The frames of a JPEG XL file: for each one its channels as numbers from 0 to 1, the
    /// extra ones last, and the names of the extra ones.
    pub(crate) fn read_jxl(bytes: &[u8]) -> (jxl_oxide::JxlImage, Vec<Vec<Vec<f32>>>, Vec<String>) {
        let image = jxl_oxide::JxlImage::builder()
            .pool(jxl_oxide::JxlThreadPool::none())
            .read(bytes)
            .expect("a JPEG XL file");
        let mut frames = Vec::new();
        let mut names = Vec::new();
        for k in 0..image.num_loaded_keyframes() {
            let render = image.render_frame(k).expect("a frame");
            names = render
                .extra_channels()
                .0
                .iter()
                .map(|c| {
                    if c.is_alpha() {
                        "alpha".to_string()
                    } else {
                        c.name().to_string()
                    }
                })
                .collect();
            frames.push(
                render
                    .image_planar()
                    .iter()
                    .map(|plane| plane.buf().to_vec())
                    .collect(),
            );
        }
        (image, frames, names)
    }

    /// The channels of a picture one by one, as a JPEG XL decoder gives them.
    pub(crate) fn planes(image: &Image) -> Vec<Vec<f32>> {
        let channels = image.channels();
        let mut out: Vec<Vec<f32>> = (0..channels)
            .map(|c| {
                image
                    .data
                    .iter()
                    .skip(c)
                    .step_by(channels)
                    .copied()
                    .collect()
            })
            .collect();
        out.extend(image.depth.clone());
        out
    }
}

#[cfg(test)]
mod tests {
    use super::testing::*;
    use super::*;

    /// A picture with slopes, edges and noise; `seed` makes frames differ.
    fn sample(width: usize, height: usize, colors: usize, alpha: bool, seed: u32) -> Image {
        let channels = colors + usize::from(alpha);
        let mut state = 0x2545_f491u32 ^ seed.wrapping_mul(0x9e37_79b9);
        let mut noise = || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            (state >> 8) as f32 / (1u32 << 24) as f32
        };
        let mut data = Vec::with_capacity(width * height * channels);
        for y in 0..height {
            for x in 0..width {
                for c in 0..channels {
                    let slope =
                        (x * (c + 1) + y * (3 - c.min(2))) as f32 / ((width + height) * 4) as f32;
                    let edge = if (x / 7 + y / 5 + c + seed as usize).is_multiple_of(3) {
                        0.25
                    } else {
                        0.0
                    };
                    data.push((slope + edge + noise() * 0.125).min(1.0));
                }
            }
        }
        Image {
            width,
            height,
            colors,
            alpha,
            data,
            depth: None,
        }
    }

    fn encoding(format: Format, bits: u32) -> Encoding {
        Encoding {
            format,
            bits,
            level: 9,
            distance: None,
            effort: 3,
            rate: (10, 1),
        }
    }

    fn one(image: &Image, e: &Encoding) -> Vec<u8> {
        encode(1, &mut |_| Ok(image.clone()), e).expect("a file")
    }

    #[test]
    fn samples_become_whole_numbers() {
        assert_eq!(step(0.0, 255), 0);
        assert_eq!(step(1.0, 255), 255);
        assert_eq!(step(0.5, 255), 128);
        assert_eq!(step(-3.0, 255), 0);
        assert_eq!(step(7.0, 65535), 65535);
        assert_eq!(step(f32::NAN, 255), 0);
        assert_eq!(step(1.0 / 65535.0, 65535), 1);
        assert_eq!(step(0.999_99, 1), 1);
        assert_eq!(step(0.49, 1), 0);
    }

    #[test]
    fn png_keeps_every_sample() {
        for colors in [1, 3] {
            for alpha in [false, true] {
                for bits in [8, 16] {
                    for level in [0, 1, 6, 9] {
                        let image = sample(37, 23, colors, alpha, 1);
                        let mut e = encoding(Format::Png, bits);
                        e.level = level;
                        let (info, frames) = read_png(&one(&image, &e));
                        assert_eq!((info.width, info.height), (37, 23));
                        assert_eq!(info.color_type.samples(), image.channels());
                        assert_eq!(info.bit_depth as u32, bits);
                        assert_eq!(frames, vec![whole(&image.data, bits)]);
                    }
                }
            }
        }
    }

    #[test]
    fn png_levels_trade_time_for_size() {
        // (steps and stripes: a picture without noise, as the drawings are)
        let mut image = sample(160, 120, 3, false, 2);
        for (k, v) in image.data.iter_mut().enumerate() {
            let (x, y, c) = (k / 3 % 160, k / 3 / 160, k % 3);
            *v = ((x / 3 + y / 5 + c) % 7) as f32 / 7.0 + (x * y % 11) as f32 / 110.0;
        }
        let size = |level: u8| {
            let mut e = encoding(Format::Png, 8);
            e.level = level;
            one(&image, &e).len()
        };
        let (none, light, most) = (size(0), size(1), size(9));
        assert!(none > 160 * 120 * 3, "{none}");
        assert!(light < none / 4 && most < light, "{none} {light} {most}");
    }

    #[test]
    fn png_holds_frames() {
        let frames: Vec<Image> = (0..4).map(|k| sample(20, 12, 3, true, k)).collect();
        let mut e = encoding(Format::Png, 16);
        e.rate = (25, 2);
        let bytes = encode(4, &mut |k| Ok(frames[k].clone()), &e).expect("a file");
        let mut reader = png::Decoder::new(std::io::Cursor::new(bytes.as_slice()))
            .read_info()
            .expect("a PNG file");
        let control = reader.info().animation_control().expect("frames");
        assert_eq!((control.num_frames, control.num_plays), (4, 0));
        let mut buffer = vec![0; reader.output_buffer_size().expect("a size")];
        reader.next_frame(&mut buffer).expect("a frame");
        let frame = reader.info().frame_control().expect("the time of a frame");
        assert_eq!((frame.delay_num, frame.delay_den), (2, 25));
        let (_, read) = read_png(&bytes);
        let want: Vec<Vec<u32>> = frames.iter().map(|f| whole(&f.data, 16)).collect();
        assert_eq!(read, want);
    }

    #[test]
    fn png_refuses_what_it_cannot_hold() {
        let mut image = sample(8, 8, 1, false, 0);
        let e = encoding(Format::Png, 12);
        assert!(
            encode(1, &mut |_| Ok(image.clone()), &e)
                .unwrap_err()
                .contains("8 or 16 bits")
        );
        image.depth = Some(vec![0.5; 64]);
        let e = encoding(Format::Png, 8);
        assert!(
            encode(1, &mut |_| Ok(image.clone()), &e)
                .unwrap_err()
                .contains("save_jxl")
        );
    }

    #[test]
    fn frames_must_agree() {
        for format in [Format::Png, Format::Jxl] {
            let e = encoding(format, 8);
            let failure = encode(2, &mut |k| Ok(sample(8 + k, 8, 3, false, 0)), &e).unwrap_err();
            assert!(failure.contains("one size"), "{failure}");
            assert!(encode(0, &mut |_| Err("never".into()), &e).is_err());
            let failure = encode(
                3,
                &mut |k| {
                    if k == 1 {
                        Err("frame 1 failed".to_string())
                    } else {
                        Ok(sample(8, 8, 3, false, 0))
                    }
                },
                &e,
            )
            .unwrap_err();
            assert_eq!(failure, "frame 1 failed");
        }
    }

    #[test]
    fn jxl_lossless_keeps_every_sample() {
        for colors in [1, 3] {
            for alpha in [false, true] {
                for bits in [1, 2, 5, 8, 10, 12, 16] {
                    let image = sample(41, 29, colors, alpha, 3);
                    let bytes = one(&image, &encoding(Format::Jxl, bits));
                    let (decoded, frames, names) = read_jxl(&bytes);
                    assert_eq!((decoded.width(), decoded.height()), (41, 29));
                    assert_eq!(decoded.pixel_format().is_grayscale(), colors == 1);
                    assert_eq!(decoded.pixel_format().has_alpha(), alpha);
                    assert_eq!(
                        decoded.image_header().metadata.bit_depth.bits_per_sample(),
                        bits
                    );
                    assert_eq!(names.len(), usize::from(alpha));
                    assert_eq!(frames.len(), 1);
                    let want: Vec<Vec<u32>> =
                        planes(&image).iter().map(|p| whole(p, bits)).collect();
                    let got: Vec<Vec<u32>> = frames[0].iter().map(|p| whole(p, bits)).collect();
                    assert_eq!(got, want, "{colors} colours, alpha {alpha}, {bits} bits");
                }
            }
        }
    }

    #[test]
    fn jxl_holds_depth_and_frames() {
        for alpha in [false, true] {
            let frames: Vec<Image> = (0..3)
                .map(|k| {
                    let mut image = sample(24, 18, 3, alpha, k);
                    image.depth = Some(sample(24, 18, 1, false, k + 10).data);
                    image
                })
                .collect();
            let mut e = encoding(Format::Jxl, 16);
            e.rate = (12, 1);
            let bytes = encode(3, &mut |k| Ok(frames[k].clone()), &e).expect("a file");
            let (decoded, read, names) = read_jxl(&bytes);
            let animation = decoded
                .image_header()
                .metadata
                .animation
                .as_ref()
                .expect("frames");
            assert_eq!(
                (animation.tps_numerator, animation.tps_denominator),
                (12, 1)
            );
            let want_names: Vec<&str> = if alpha {
                vec!["alpha", "depth"]
            } else {
                vec!["depth"]
            };
            assert_eq!(names, want_names);
            assert_eq!(read.len(), 3);
            for (frame, got) in frames.iter().zip(&read) {
                let want: Vec<Vec<u32>> = planes(frame).iter().map(|p| whole(p, 16)).collect();
                let got: Vec<Vec<u32>> = got.iter().map(|p| whole(p, 16)).collect();
                assert_eq!(got, want);
            }
        }
    }

    /// The mean square of the differences of two channels.
    fn error(a: &[f32], b: &[f32]) -> f64 {
        a.iter()
            .zip(b)
            .map(|(x, y)| f64::from(x - y).powi(2))
            .sum::<f64>()
            / a.len() as f64
    }

    #[test]
    fn jxl_lossy_is_smaller_and_near() {
        // a smooth picture, as a lossy file suits
        let (width, height) = (128, 96);
        let mut image = sample(width, height, 3, true, 0);
        for (k, v) in image.data.iter_mut().enumerate() {
            let (x, y, c) = (k / 4 % width, k / 4 / width, k % 4);
            let (fx, fy) = (x as f32 / width as f32, y as f32 / height as f32);
            *v = match c {
                0 => fx,
                1 => fy,
                2 => fx * fy,
                _ => {
                    if x < width / 2 {
                        1.0
                    } else {
                        0.25
                    }
                }
            };
        }
        image.depth = Some(sample(width, height, 1, false, 4).data);
        let exact = one(&image, &encoding(Format::Jxl, 16));
        let mut sizes = Vec::new();
        let mut errors = Vec::new();
        for quality in [95.0, 60.0, 20.0] {
            let mut e = encoding(Format::Jxl, 16);
            e.distance = Some(distance_of(quality));
            let bytes = one(&image, &e);
            let (_, frames, names) = read_jxl(&bytes);
            assert_eq!(names, ["alpha", "depth"]);
            let want = planes(&image);
            let colour: f64 = (0..3).map(|c| error(&want[c], &frames[0][c])).sum::<f64>() / 3.0;
            // transparency and depth are exact, the colours are near
            assert_eq!(whole(&frames[0][3], 16), whole(&want[3], 16));
            assert_eq!(whole(&frames[0][4], 16), whole(&want[4], 16));
            assert!(colour > 0.0, "quality {quality} changed nothing");
            sizes.push(bytes.len());
            errors.push(colour);
        }
        assert!(distance_of(90.0) == 1.0 && distance_of(100.0) == 0.0);
        assert!(sizes[0] > sizes[1] && sizes[1] > sizes[2], "{sizes:?}");
        assert!(errors[0] < errors[1] && errors[1] < errors[2], "{errors:?}");
        assert!(errors[0] < 1e-4 && errors[2] < 1e-2, "{errors:?}");
        assert!(exact.len() > sizes[2], "{} {sizes:?}", exact.len());
    }

    #[test]
    fn jxl_effort_and_threads() {
        let image = sample(96, 64, 3, false, 5);
        let mut e = encoding(Format::Jxl, 8);
        // the threads change nothing in the file
        for distance in [None, Some(1.0)] {
            e.distance = distance;
            let alone = jxl_file(1, &mut |_| Ok(image.clone()), &e, 0).expect("a file");
            let many = jxl_file(1, &mut |_| Ok(image.clone()), &e, 5).expect("a file");
            assert_eq!(alone, many);
        }
        // more effort, a smaller lossless file
        e.distance = None;
        e.effort = 1;
        let quick = one(&image, &e);
        e.effort = 9;
        let slow = one(&image, &e);
        assert!(slow.len() < quick.len(), "{} {}", slow.len(), quick.len());
        for bytes in [&quick, &slow] {
            let (_, frames, _) = read_jxl(bytes);
            let got: Vec<Vec<u32>> = frames[0].iter().map(|p| whole(p, 8)).collect();
            let want: Vec<Vec<u32>> = planes(&image).iter().map(|p| whole(p, 8)).collect();
            assert_eq!(got, want);
        }
        let failure = one_failure(&image, 17);
        assert!(failure.contains("1 to 16 bits"), "{failure}");
    }

    fn one_failure(image: &Image, bits: u32) -> String {
        encode(1, &mut |_| Ok(image.clone()), &encoding(Format::Jxl, bits)).unwrap_err()
    }

    #[test]
    fn files_are_written_whole() {
        let dir =
            std::env::temp_dir().join(format!("symbolic-math-imgfile-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a directory");
        let path = dir.join("picture.png");
        let path = path.to_str().expect("a name");
        write(path, b"first").expect("written");
        write(path, b"second").expect("written again");
        assert_eq!(std::fs::read(path).expect("the file"), b"second");
        let names: Vec<_> = std::fs::read_dir(&dir)
            .expect("the directory")
            .map(|e| e.expect("an entry").file_name())
            .collect();
        assert_eq!(names, ["picture.png"]);
        let missing = dir.join("none").join("picture.png");
        let failure = write(missing.to_str().expect("a name"), b"x").unwrap_err();
        assert!(failure.starts_with("cannot write "), "{failure}");
        assert!(write("/", b"x").is_err());
        std::fs::remove_dir_all(&dir).expect("removed");
    }
}
