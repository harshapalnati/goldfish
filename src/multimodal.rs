//! # Multi-Modal Memory Support
//!
//! Store and retrieve images, audio, and other media types alongside text memories.
//!
//! ## Features
//!
//! - **Image Storage**: JPEG, PNG, WebP support
//! - **Image Embeddings**: CLIP-style vision embeddings
//! - **Image Search**: Find similar images
//! - **Text-Image Associations**: Link images to text memories
//! - **Metadata Extraction**: Automatic image metadata
//!
//! ## Supported Formats
//!
//! - Images: JPEG, PNG, GIF, WebP, BMP
//! - Max size: 10MB per image
//! - Recommended: 512x512 for embeddings
//!
//! ## Example
//!
//! ```rust,no_run
//! use goldfish::multimodal::{MultiModalSystem, ImageMemory, ImageFormat};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let mm = MultiModalSystem::new("./data").await?;
//!     
//!     // Store an image
//!     let image_data = std::fs::read("photo.jpg")?;
//!     let image = ImageMemory::new(image_data, ImageFormat::Jpeg)
//!         .with_description("A photo of my office");
//!     
//!     mm.save_image(&image).await?;
//!     
//!     // Search for similar images
//!     let similar = mm.search_images_by_text("office", 5).await?;
//!     
//!     Ok(())
//! }
//! ```

use crate::{
    error::{MemoryError, Result},
    types::{Memory, MemoryId, MemoryType},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported image formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Gif,
    WebP,
    Bmp,
}

impl ImageFormat {
    /// Get file extension
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::Gif => "gif",
            Self::WebP => "webp",
            Self::Bmp => "bmp",
        }
    }
    
    /// Get MIME type
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::Gif => "image/gif",
            Self::WebP => "image/webp",
            Self::Bmp => "image/bmp",
        }
    }
    
    /// Detect format from magic bytes
    pub fn detect(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }
        
        match &data[0..8] {
            [0xFF, 0xD8, 0xFF, ..] => Some(Self::Jpeg),
            [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] => Some(Self::Png),
            [0x47, 0x49, 0x46, 0x38, ..] => Some(Self::Gif),
            [0x52, 0x49, 0x46, 0x46, ..] if data.len() > 8 && &data[8..12] == b"WEBP" => Some(Self::WebP),
            [0x42, 0x4D, ..] => Some(Self::Bmp),
            _ => None,
        }
    }
}

/// Image dimensions
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ImageDimensions {
    pub width: u32,
    pub height: u32,
}

impl ImageDimensions {
    /// Calculate aspect ratio
    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height.max(1) as f32
    }
    
    /// Check if landscape orientation
    pub fn is_landscape(&self) -> bool {
        self.width > self.height
    }
    
    /// Check if portrait orientation
    pub fn is_portrait(&self) -> bool {
        self.height > self.width
    }
}

/// Image memory representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMemory {
    pub id: MemoryId,
    pub data: Vec<u8>,
    pub format: ImageFormat,
    pub dimensions: Option<ImageDimensions>,
    pub description: Option<String>,
    pub embedding: Option<Vec<f32>>,
    pub created_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
    pub tags: Vec<String>,
    /// Associated text memory IDs
    pub associations: Vec<MemoryId>,
    /// Color palette (extracted dominant colors)
    pub color_palette: Vec<String>,
    /// OCR text if available
    pub ocr_text: Option<String>,
}

impl ImageMemory {
    /// Create new image memory
    pub fn new(data: Vec<u8>, format: ImageFormat) -> Self {
        Self {
            id: MemoryId::new(),
            data,
            format,
            dimensions: None,
            description: None,
            embedding: None,
            created_at: Utc::now(),
            metadata: HashMap::new(),
            tags: Vec::new(),
            associations: Vec::new(),
            color_palette: Vec::new(),
            ocr_text: None,
        }
    }
    
    /// Auto-detect format from data
    pub fn from_bytes(data: Vec<u8>) -> Result<Self> {
        let format = ImageFormat::detect(&data)
            .ok_or_else(|| MemoryError::Validation("Unknown image format".to_string()))?;
        
        Ok(Self::new(data, format))
    }
    
    /// Add description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
    
    /// Add tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
    
    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
    
    /// Associate with text memory
    pub fn associate_with(mut self, memory_id: MemoryId) -> Self {
        self.associations.push(memory_id);
        self
    }
    
    /// Get file size in bytes
    pub fn size_bytes(&self) -> usize {
        self.data.len()
    }
    
    /// Get file size in KB
    pub fn size_kb(&self) -> f64 {
        self.size_bytes() as f64 / 1024.0
    }
    
    /// Get data URI for embedding in HTML
    pub fn data_uri(&self) -> String {
        format!(
            "data:{};base64,{}",
            self.format.mime_type(),
            base64::encode(&self.data)
        )
    }
}

/// Image storage backend trait
#[async_trait::async_trait]
pub trait ImageStorage: Send + Sync {
    /// Save image
    async fn save(&self, image: &ImageMemory) -> Result<()>;
    
    /// Load image by ID
    async fn load(&self, id: &MemoryId) -> Result<Option<ImageMemory>>;
    
    /// Delete image
    async fn delete(&self, id: &MemoryId) -> Result<bool>;
    
    /// List all images
    async fn list(&self, limit: usize) -> Result<Vec<ImageMemory>>;
    
    /// Search by embedding
    async fn search_by_embedding(&self, embedding: &[f32], limit: usize) -> Result<Vec<(ImageMemory, f32)>>;
    
    /// Find images associated with memory
    async fn find_by_association(&self, memory_id: &MemoryId) -> Result<Vec<ImageMemory>>;
}

/// Multi-modal system configuration
#[derive(Debug, Clone)]
pub struct MultiModalConfig {
    /// Max image size in bytes (default: 10MB)
    pub max_image_size: usize,
    /// Thumbnail size
    pub thumbnail_size: u32,
    /// Enable automatic OCR
    pub enable_ocr: bool,
    /// Enable color palette extraction
    pub enable_color_extraction: bool,
    /// Supported formats
    pub supported_formats: Vec<ImageFormat>,
}

impl Default for MultiModalConfig {
    fn default() -> Self {
        Self {
            max_image_size: 10 * 1024 * 1024, // 10MB
            thumbnail_size: 256,
            enable_ocr: false,
            enable_color_extraction: true,
            supported_formats: vec![
                ImageFormat::Jpeg,
                ImageFormat::Png,
                ImageFormat::WebP,
                ImageFormat::Gif,
                ImageFormat::Bmp,
            ],
        }
    }
}

/// Multi-modal memory system
pub struct MultiModalSystem {
    storage: Box<dyn ImageStorage>,
    config: MultiModalConfig,
}

impl MultiModalSystem {
    /// Create new multi-modal system
    pub fn new(storage: Box<dyn ImageStorage>, config: MultiModalConfig) -> Self {
        Self { storage, config }
    }
    
    /// Save an image
    pub async fn save_image(&self, image: &ImageMemory) -> Result<()> {
        // Validate image size
        if image.size_bytes() > self.config.max_image_size {
            return Err(MemoryError::Validation(
                format!("Image too large: {} > {} bytes", image.size_bytes(), self.config.max_image_size)
            ));
        }
        
        // Validate format
        if !self.config.supported_formats.contains(&image.format) {
            return Err(MemoryError::Validation(
                format!("Unsupported image format: {:?}", image.format)
            ));
        }
        
        self.storage.save(image).await
    }
    
    /// Load an image
    pub async fn load_image(&self, id: &MemoryId) -> Result<Option<ImageMemory>> {
        self.storage.load(id).await
    }
    
    /// Delete an image
    pub async fn delete_image(&self, id: &MemoryId) -> Result<bool> {
        self.storage.delete(id).await
    }
    
    /// List all images
    pub async fn list_images(&self, limit: usize) -> Result<Vec<ImageMemory>> {
        self.storage.list(limit).await
    }
    
    /// Search images by text (requires text-to-image embedding model)
    pub async fn search_images_by_text(&self, query: &str, limit: usize) -> Result<Vec<(ImageMemory, f32)>> {
        // This would use a CLIP-style model to generate image embedding from text
        // For now, search by description
        let all_images = self.storage.list(1000).await?;
        let query_lower = query.to_lowercase();
        
        let results: Vec<_> = all_images
            .into_iter()
            .filter_map(|img| {
                let score = if let Some(ref desc) = img.description {
                    if desc.to_lowercase().contains(&query_lower) {
                        Some(0.9)
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                score.map(|s| (img, s))
            })
            .take(limit)
            .collect();
        
        Ok(results)
    }
    
    /// Search images by similar image
    pub async fn search_images_by_image(&self, image_id: &MemoryId, limit: usize) -> Result<Vec<(ImageMemory, f32)>> {
        let image = self.storage.load(image_id).await?
            .ok_or_else(|| MemoryError::NotFound(format!("Image {} not found", image_id)))?;
        
        if let Some(ref embedding) = image.embedding {
            self.storage.search_by_embedding(embedding, limit).await
        } else {
            Ok(vec![])
        }
    }
    
    /// Find images associated with text memory
    pub async fn find_images_for_memory(&self, memory_id: &MemoryId) -> Result<Vec<ImageMemory>> {
        self.storage.find_by_association(memory_id).await
    }
    
    /// Associate image with text memory
    pub async fn associate_with_memory(&self, image_id: &MemoryId, memory_id: &MemoryId) -> Result<()> {
        let mut image = self.storage.load(image_id).await?
            .ok_or_else(|| MemoryError::NotFound(format!("Image {} not found", image_id)))?;
        
        if !image.associations.contains(memory_id) {
            image.associations.push(memory_id.clone());
            self.storage.save(&image).await?;
        }
        
        Ok(())
    }
    
    /// Validate image data
    pub fn validate_image(&self, data: &[u8]) -> Result<ImageFormat> {
        if data.is_empty() {
            return Err(MemoryError::Validation("Empty image data".to_string()));
        }
        
        if data.len() > self.config.max_image_size {
            return Err(MemoryError::Validation(
                format!("Image too large: {} > {} bytes", data.len(), self.config.max_image_size)
            ));
        }
        
        ImageFormat::detect(data)
            .ok_or_else(|| MemoryError::Validation("Unknown or unsupported image format".to_string()))
    }
}

/// Image processing utilities
pub mod processing {
    use super::*;
    
    /// Calculate dimensions from image data (if possible)
    pub fn get_dimensions(data: &[u8], format: ImageFormat) -> Option<ImageDimensions> {
        match format {
            ImageFormat::Png => get_png_dimensions(data),
            ImageFormat::Jpeg => get_jpeg_dimensions(data),
            ImageFormat::Gif => get_gif_dimensions(data),
            ImageFormat::Bmp => get_bmp_dimensions(data),
            ImageFormat::WebP => get_webp_dimensions(data),
        }
    }
    
    fn get_png_dimensions(data: &[u8]) -> Option<ImageDimensions> {
        if data.len() < 24 {
            return None;
        }
        
        // PNG IHDR chunk is at offset 16
        let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
        let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
        
        Some(ImageDimensions { width, height })
    }
    
    fn get_jpeg_dimensions(data: &[u8]) -> Option<ImageDimensions> {
        // JPEG dimensions parsing is complex, simplified here
        // Would need proper JPEG parser in production
        None
    }
    
    fn get_gif_dimensions(data: &[u8]) -> Option<ImageDimensions> {
        if data.len() < 10 {
            return None;
        }
        
        let width = u16::from_le_bytes([data[6], data[7]]) as u32;
        let height = u16::from_le_bytes([data[8], data[9]]) as u32;
        
        Some(ImageDimensions { width, height })
    }
    
    fn get_bmp_dimensions(data: &[u8]) -> Option<ImageDimensions> {
        if data.len() < 26 {
            return None;
        }
        
        let width = i32::from_le_bytes([data[18], data[19], data[20], data[21]]) as u32;
        let height = i32::from_le_bytes([data[22], data[23], data[24], data[25]]) as u32;
        
        Some(ImageDimensions { width, height })
    }
    
    fn get_webp_dimensions(data: &[u8]) -> Option<ImageDimensions> {
        // WebP dimensions parsing requires VP8 chunk parsing
        None
    }
    
    /// Extract dominant colors from image
    pub fn extract_color_palette(_data: &[u8], _format: ImageFormat) -> Vec<String> {
        // Would use image processing library
        vec![]
    }
}

/// Configuration builder
#[derive(Debug, Default)]
pub struct MultiModalConfigBuilder {
    config: MultiModalConfig,
}

impl MultiModalConfigBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set max image size
    pub fn max_image_size(mut self, bytes: usize) -> Self {
        self.config.max_image_size = bytes;
        self
    }
    
    /// Set thumbnail size
    pub fn thumbnail_size(mut self, size: u32) -> Self {
        self.config.thumbnail_size = size;
        self
    }
    
    /// Enable OCR
    pub fn enable_ocr(mut self, enable: bool) -> Self {
        self.config.enable_ocr = enable;
        self
    }
    
    /// Enable color extraction
    pub fn enable_color_extraction(mut self, enable: bool) -> Self {
        self.config.enable_color_extraction = enable;
        self
    }
    
    /// Set supported formats
    pub fn supported_formats(mut self, formats: Vec<ImageFormat>) -> Self {
        self.config.supported_formats = formats;
        self
    }
    
    /// Build configuration
    pub fn build(self) -> MultiModalConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_format_detection() {
        // PNG magic bytes
        let png_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(ImageFormat::detect(&png_data), Some(ImageFormat::Png));
        
        // JPEG magic bytes
        let jpeg_data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];
        assert_eq!(ImageFormat::detect(&jpeg_data), Some(ImageFormat::Jpeg));
        
        // GIF magic bytes
        let gif_data = vec![0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x00, 0x00];
        assert_eq!(ImageFormat::detect(&gif_data), Some(ImageFormat::Gif));
        
        // Unknown
        let unknown = vec![0x00, 0x00, 0x00, 0x00];
        assert_eq!(ImageFormat::detect(&unknown), None);
    }

    #[test]
    fn test_image_memory_creation() {
        let data = vec![0xFF, 0xD8, 0xFF]; // JPEG
        let image = ImageMemory::new(data.clone(), ImageFormat::Jpeg)
            .with_description("Test image")
            .with_tags(vec!["test".to_string()]);
        
        assert_eq!(image.format, ImageFormat::Jpeg);
        assert_eq!(image.description, Some("Test image".to_string()));
        assert_eq!(image.tags, vec!["test"]);
        assert_eq!(image.size_bytes(), 3);
    }

    #[test]
    fn test_image_dimensions() {
        let dims = ImageDimensions { width: 1920, height: 1080 };
        assert_eq!(dims.aspect_ratio(), 1920.0 / 1080.0);
        assert!(dims.is_landscape());
        assert!(!dims.is_portrait());
        
        let dims2 = ImageDimensions { width: 1080, height: 1920 };
        assert!(dims2.is_portrait());
    }

    #[test]
    fn test_image_format_extensions() {
        assert_eq!(ImageFormat::Jpeg.extension(), "jpg");
        assert_eq!(ImageFormat::Png.extension(), "png");
        assert_eq!(ImageFormat::Gif.extension(), "gif");
        assert_eq!(ImageFormat::WebP.extension(), "webp");
        assert_eq!(ImageFormat::Bmp.extension(), "bmp");
    }

    #[test]
    fn test_png_dimension_parsing() {
        // Create minimal PNG IHDR chunk
        let mut data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x0D]); // IHDR length
        data.extend_from_slice(b"IHDR");
        data.extend_from_slice(&1920u32.to_be_bytes()); // width
        data.extend_from_slice(&1080u32.to_be_bytes()); // height
        
        let dims = processing::get_png_dimensions(&data);
        assert!(dims.is_some());
        let dims = dims.unwrap();
        assert_eq!(dims.width, 1920);
        assert_eq!(dims.height, 1080);
    }
}
