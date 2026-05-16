//! 缩略图生成器
//!
//! 支持图片压缩和缩略图生成
//! 使用 image crate 进行图片处理

use image::{ImageBuffer, ImageOutputFormat, GenericImageView, imageops::FilterType};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use thiserror::Error;

/// 缩略图错误
#[derive(Error, Debug)]
pub enum ThumbnailError {
    #[error("图片解码失败: {0}")]
    Decode(#[from] image::ImageError),
    
    #[error("不支持的图片格式: {0}")]
    UnsupportedFormat(String),
    
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
}

/// 缩略图尺寸预设
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ThumbnailSize {
    /// 小缩略图 (100x100)
    Small,
    /// 中等缩略图 (300x300)
    Medium,
    /// 大缩略图 (600x600)
    Large,
    /// 自定义尺寸
    Custom { width: u32, height: u32 },
}

impl ThumbnailSize {
    /// 获取尺寸
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::Small => (100, 100),
            Self::Medium => (300, 300),
            Self::Large => (600, 600),
            Self::Custom { width, height } => (*width, *height),
        }
    }
}

impl Default for ThumbnailSize {
    fn default() -> Self {
        Self::Medium
    }
}

/// 图片格式
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Webp,
    Gif,
}

impl ImageFormat {
    /// 从 MIME 类型获取格式
    pub fn from_mime(mime: &str) -> Option<Self> {
        match mime {
            "image/jpeg" | "image/jpg" => Some(Self::Jpeg),
            "image/png" => Some(Self::Png),
            "image/webp" => Some(Self::Webp),
            "image/gif" => Some(Self::Gif),
            _ => None,
        }
    }
    
    /// 获取 MIME 类型
    pub fn mime(&self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::Webp => "image/webp",
            Self::Gif => "image/gif",
        }
    }
    
    /// 获取 image crate 格式
    pub fn to_image_format(&self) -> ImageOutputFormat {
        match self {
            Self::Jpeg => ImageOutputFormat::Jpeg(85),
            Self::Png => ImageOutputFormat::Png,
            Self::Webp => ImageOutputFormat::WebP,
            Self::Gif => ImageOutputFormat::Gif,
        }
    }
}

/// 缩略图生成器
pub struct ThumbnailGenerator {
    default_size: ThumbnailSize,
    default_format: ImageFormat,
    quality: u8,
}

impl Default for ThumbnailGenerator {
    fn default() -> Self {
        Self {
            default_size: ThumbnailSize::Medium,
            default_format: ImageFormat::Jpeg,
            quality: 85,
        }
    }
}

impl ThumbnailGenerator {
    /// 创建新的生成器
    pub fn new() -> Self {
        Self::default()
    }
    
    /// 设置默认尺寸
    pub fn with_size(mut self, size: ThumbnailSize) -> Self {
        self.default_size = size;
        self
    }
    
    /// 设置默认格式
    pub fn with_format(mut self, format: ImageFormat) -> Self {
        self.default_format = format;
        self
    }
    
    /// 设置质量 (1-100)
    pub fn with_quality(mut self, quality: u8) -> Self {
        self.quality = quality.min(100);
        self
    }
    
    /// 生成缩略图
    pub fn generate(
        &self,
        image_data: &[u8],
        size: Option<ThumbnailSize>,
        format: Option<ImageFormat>,
    ) -> Result<Vec<u8>, ThumbnailError> {
        let size = size.unwrap_or(self.default_size);
        let format = format.unwrap_or(self.default_format);
        
        // 解码图片
        let img = image::load_from_memory(image_data)?;
        let (orig_width, orig_height) = img.dimensions();
        
        // 计算缩放后的尺寸（保持宽高比）
        let (target_width, target_height) = size.dimensions();
        let (new_width, new_height) = self.calculate_dimensions(
            orig_width, 
            orig_height, 
            target_width, 
            target_height,
        );
        
        // 缩放图片
        let thumbnail = img.resize_exact(new_width, new_height, FilterType::Lanczos3);
        
        // 编码输出
        let mut buffer = Cursor::new(Vec::new());
        let output_format = match format {
            ImageFormat::Jpeg => ImageOutputFormat::Jpeg(self.quality),
            _ => format.to_image_format(),
        };
        
        thumbnail.write_to(&mut buffer, output_format)?;
        
        Ok(buffer.into_inner())
    }
    
    /// 生成带裁剪的缩略图（正方形）
    pub fn generate_square(
        &self,
        image_data: &[u8],
        size: Option<ThumbnailSize>,
        format: Option<ImageFormat>,
    ) -> Result<Vec<u8>, ThumbnailError> {
        let size = size.unwrap_or(self.default_size);
        let format = format.unwrap_or(self.default_format);
        let (target_size, _) = size.dimensions();
        
        // 解码图片
        let img = image::load_from_memory(image_data)?;
        let (orig_width, orig_height) = img.dimensions();
        
        // 计算裁剪区域（居中裁剪）
        let crop_size = orig_width.min(orig_height);
        let x = (orig_width - crop_size) / 2;
        let y = (orig_height - crop_size) / 2;
        
        // 裁剪
        let cropped = img.crop_imm(x, y, crop_size, crop_size);
        
        // 缩放到目标尺寸
        let thumbnail = cropped.resize_exact(target_size, target_size, FilterType::Lanczos3);
        
        // 编码输出
        let mut buffer = Cursor::new(Vec::new());
        let output_format = match format {
            ImageFormat::Jpeg => ImageOutputFormat::Jpeg(self.quality),
            _ => format.to_image_format(),
        };
        
        thumbnail.write_to(&mut buffer, output_format)?;
        
        Ok(buffer.into_inner())
    }
    
    /// 获取图片信息
    pub fn get_info(&self, image_data: &[u8]) -> Result<ImageInfo, ThumbnailError> {
        let img = image::load_from_memory(image_data)?;
        let (width, height) = img.dimensions();
        let color_type = img.color();
        
        Ok(ImageInfo {
            width,
            height,
            color_type: format!("{:?}", color_type),
            size_bytes: image_data.len() as u64,
        })
    }
    
    /// 计算缩放后的尺寸
    fn calculate_dimensions(
        &self,
        orig_width: u32,
        orig_height: u32,
        max_width: u32,
        max_height: u32,
    ) -> (u32, u32) {
        let width_ratio = max_width as f64 / orig_width as f64;
        let height_ratio = max_height as f64 / orig_height as f64;
        let ratio = width_ratio.min(height_ratio);
        
        // 如果原图比目标小，不放大
        if ratio > 1.0 {
            return (orig_width, orig_height);
        }
        
        (
            (orig_width as f64 * ratio) as u32,
            (orig_height as f64 * ratio) as u32,
        )
    }
}

/// 图片信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub color_type: String,
    pub size_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thumbnail_size_default() {
        let size = ThumbnailSize::default();
        assert_eq!(size.dimensions(), (300, 300));
    }

    #[test]
    fn test_thumbnail_size_custom() {
        let size = ThumbnailSize::Custom { width: 500, height: 500 };
        assert_eq!(size.dimensions(), (500, 500));
    }

    #[test]
    fn test_image_format_from_mime() {
        assert_eq!(ImageFormat::from_mime("image/jpeg"), Some(ImageFormat::Jpeg));
        assert_eq!(ImageFormat::from_mime("image/png"), Some(ImageFormat::Png));
        assert_eq!(ImageFormat::from_mime("image/gif"), Some(ImageFormat::Gif));
        assert_eq!(ImageFormat::from_mime("image/webp"), Some(ImageFormat::Webp));
        assert_eq!(ImageFormat::from_mime("image/bmp"), None);
    }

    #[test]
    fn test_generator_creation() {
        let gen = ThumbnailGenerator::new()
            .with_size(ThumbnailSize::Small)
            .with_format(ImageFormat::Png)
            .with_quality(90);
        
        assert_eq!(gen.default_size.dimensions(), (100, 100));
        assert_eq!(gen.default_format, ImageFormat::Png);
        assert_eq!(gen.quality, 90);
    }

    #[test]
    fn test_calculate_dimensions() {
        let gen = ThumbnailGenerator::new();
        
        // 横向图片
        let (w, h) = gen.calculate_dimensions(1920, 1080, 300, 300);
        assert!(w <= 300 && h <= 300);
        assert_eq!(w as f64 / h as f64, 1920.0 / 1080.0);
        
        // 纵向图片
        let (w, h) = gen.calculate_dimensions(1080, 1920, 300, 300);
        assert!(w <= 300 && h <= 300);
        
        // 小图不应该放大
        let (w, h) = gen.calculate_dimensions(100, 100, 300, 300);
        assert_eq!((w, h), (100, 100));
    }
}
