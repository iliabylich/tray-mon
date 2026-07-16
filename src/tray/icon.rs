use anyhow::{Context, Result, bail, ensure};
use icon::{IconFile, Icons};
use std::{os::unix::ffi::OsStrExt, path::Path};
use tempfile::NamedTempFile;

#[derive(Debug)]
pub struct TrayIcon {
    tempfile: NamedTempFile,
}

impl TrayIcon {
    const SIZE: u32 = 32;

    pub(crate) fn new_icon_name(name_or_path: &str, icons: &Icons) -> Result<Self> {
        if name_or_path.starts_with('/') {
            Self::from_filepath(name_or_path)
        } else {
            Self::from_icon_name(name_or_path, icons)
        }
    }

    fn from_filepath(path: &str) -> Result<Self> {
        let ext = Path::new(path).extension().context("no icon extension")?;

        match ext.as_bytes() {
            b"png" => Self::from_png(path),
            b"svg" => Self::from_svg(path),
            other => bail!(
                "unsupported icon extension: {:?}",
                String::from_utf8_lossy(other)
            ),
        }
    }

    fn from_png(path: impl AsRef<Path>) -> Result<Self> {
        let image = image::open(path)?;
        let resized = image.resize_exact(
            Self::SIZE,
            Self::SIZE,
            image::imageops::FilterType::Lanczos3,
        );
        let tempfile = png_tempfile()?;
        resized
            .save_with_format(tempfile.path(), image::ImageFormat::Png)
            .context("failed to re-scale PNG")?;
        Ok(Self { tempfile })
    }

    #[expect(clippy::cast_precision_loss)]
    fn from_svg(path: impl AsRef<Path>) -> Result<Self> {
        use resvg::{
            tiny_skia::Pixmap,
            usvg::{self, Transform},
        };

        let svg_data = std::fs::read(path)?;

        let options = usvg::Options::default();
        let tree = usvg::Tree::from_data(&svg_data, &options)?;

        let svg_size = tree.size();

        let scale_x = Self::SIZE as f32 / svg_size.width();
        let scale_y = Self::SIZE as f32 / svg_size.height();
        let scale = scale_x.min(scale_y);

        let rendered_width = svg_size.width() * scale;
        let rendered_height = svg_size.height() * scale;
        let offset_x = (Self::SIZE as f32 - rendered_width) / 2.0;
        let offset_y = (Self::SIZE as f32 - rendered_height) / 2.0;

        let transform =
            Transform::from_scale(scale, scale).post_translate(offset_x / scale, offset_y / scale);

        let mut pixmap =
            Pixmap::new(Self::SIZE, Self::SIZE).context("desired PNG has invalid dimensions")?;

        resvg::render(&tree, transform, &mut pixmap.as_mut());

        let tempfile = png_tempfile()?;
        pixmap
            .save_png(tempfile.path())
            .context("failed to convert SVG to PNG")?;
        Ok(Self { tempfile })
    }

    fn from_icon_name(name: &str, icons: &Icons) -> Result<Self> {
        let symbolic = format!("{name}-symbolic");

        let find = |name: &str, theme: &str| -> Option<IconFile> {
            icons.find_icon(name, Self::SIZE, 1, theme)
        };

        let Some(icon) = None
            .or_else(|| find(name, "hicolor"))
            .or_else(|| find(&symbolic, "hicolor"))
            .or_else(|| find(name, "Adwaita"))
            .or_else(|| find(&symbolic, "Adwaita"))
        else {
            bail!("failed to find icon {name}");
        };

        match icon.file_type() {
            icon::FileType::Png => Self::from_png(icon.path()),
            icon::FileType::Svg => Self::from_svg(icon.path()),
            icon::FileType::Xpm => bail!("XPM icons aren't supported"),
        }
    }

    pub(crate) fn new_icon_pixmap(width: u32, height: u32, bytes: Vec<u8>) -> Result<Self> {
        ensure!(width != 0, "RGBA source can't have width=0");
        ensure!(height != 0, "RGBA source can't have height=0");

        ensure!(
            width as usize * height as usize * 4 == bytes.len(),
            "RGBA buffer is malformed"
        );

        let source =
            image::RgbaImage::from_raw(width, height, bytes).context("RGBA buffer is malformed")?;

        let resized = image::imageops::resize(
            &source,
            Self::SIZE,
            Self::SIZE,
            image::imageops::FilterType::Lanczos3,
        );

        let tempfile = png_tempfile()?;
        resized
            .save_with_format(tempfile.path(), image::ImageFormat::Png)
            .context("failed to convert RGBA to PNG")?;
        Ok(Self { tempfile })
    }

    pub(crate) fn filepath(&self) -> &[u8] {
        self.tempfile.path().as_os_str().as_bytes()
    }
}

fn png_tempfile() -> Result<NamedTempFile> {
    tempfile::Builder::new()
        .suffix(".png")
        .tempfile()
        .context("failed to create tempfile")
}
