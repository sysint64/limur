// Based on Glyphon: https://github.com/grovesNL/glyphon
use std::{
    collections::HashSet,
    error::Error,
    fmt::{Display, Formatter},
    hash::BuildHasherDefault,
};

use limur::ColorRgba;
use lru::LruCache;
use rustc_hash::FxHasher;
use sumi::GraphicsContext;

use crate::vector_resources::{VectorData, to_color};

type Hasher = BuildHasherDefault<FxHasher>;

pub(crate) struct InnerAtlas {
    pub kind: Kind,
    pub texture: wgpu::Texture,
    pub texture_view: wgpu::TextureView,
    pub packer: etagere::BucketedAtlasAllocator,
    pub size: u32,
    pub glyph_cache: LruCache<cosmic_text::CacheKey, GlyphDetails, Hasher>,
    pub glyphs_in_use: HashSet<cosmic_text::CacheKey, Hasher>,
    pub max_texture_dimension_2d: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Mask,
    Color { srgb: bool },
}

pub(crate) struct GlyphDetails {
    width: u16,
    height: u16,
    gpu_cache: GpuCacheStatus,
    atlas_id: Option<etagere::AllocId>,
    top: i16,
    left: i16,
}

pub(crate) enum GpuCacheStatus {
    InAtlas {
        x: u16,
        y: u16,
        content_type: ContentType,
    },
    SkipRasterization,
}

/// The type of image data contained in a rasterized glyph
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ContentType {
    /// Each pixel contains 32 bits of rgba data
    Color,
    /// Each pixel contains a single 8 bit channel
    Mask,
}

impl InnerAtlas {
    const INITIAL_SIZE: u32 = 1024;

    fn new(context: &sumi::GraphicsContext, kind: Kind) -> Self {
        let max_texture_dimension_2d = context.device.limits().max_texture_dimension_2d;
        let size = Self::INITIAL_SIZE.min(max_texture_dimension_2d);

        let packer = etagere::BucketedAtlasAllocator::new(etagere::size2(size as i32, size as i32));

        // Create a texture to use for our atlas
        let texture = context.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("text atlas"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: kind.texture_format(),
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let glyph_cache = LruCache::unbounded_with_hasher(Hasher::default());
        let glyphs_in_use = HashSet::with_hasher(Hasher::default());

        Self {
            kind,
            texture,
            texture_view,
            packer,
            size,
            glyph_cache,
            glyphs_in_use,
            max_texture_dimension_2d,
        }
    }

    pub(crate) fn try_allocate(
        &mut self,
        width: usize,
        height: usize,
    ) -> Option<etagere::Allocation> {
        let size = etagere::size2(width as i32, height as i32);

        loop {
            let allocation = self.packer.allocate(size);

            if allocation.is_some() {
                return allocation;
            }

            // Try to free least recently used allocation
            let (mut key, mut value) = self.glyph_cache.peek_lru()?;

            // Find a glyph with an actual size
            while value.atlas_id.is_none() {
                // All sized glyphs are in use, cache is full
                if self.glyphs_in_use.contains(key) {
                    return None;
                }

                let _ = self.glyph_cache.pop_lru();

                (key, value) = self.glyph_cache.peek_lru()?;
            }

            // All sized glyphs are in use, cache is full
            if self.glyphs_in_use.contains(key) {
                return None;
            }

            let (_, value) = self.glyph_cache.pop_lru().unwrap();
            self.packer.deallocate(value.atlas_id.unwrap());
        }
    }

    pub fn num_channels(&self) -> usize {
        self.kind.num_channels()
    }

    pub(crate) fn grow(
        &mut self,
        context: &sumi::GraphicsContext,
        font_system: &mut cosmic_text::FontSystem,
        cache: &mut cosmic_text::SwashCache,
        scale_factor: f32,
    ) -> bool {
        if self.size >= self.max_texture_dimension_2d {
            return false;
        }

        // Grow each dimension by a factor of 2. The growth factor was chosen to match the growth
        // factor of `Vec`.`
        const GROWTH_FACTOR: u32 = 2;
        let new_size = (self.size * GROWTH_FACTOR).min(self.max_texture_dimension_2d);

        self.packer
            .grow(etagere::size2(new_size as i32, new_size as i32));

        // Create a texture to use for our atlas
        self.texture = context.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("glyphon atlas"),
            size: wgpu::Extent3d {
                width: new_size,
                height: new_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.kind.texture_format(),
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Re-upload glyphs
        for (&cache_key, glyph) in &self.glyph_cache {
            let (x, y) = match glyph.gpu_cache {
                GpuCacheStatus::InAtlas { x, y, .. } => (x, y),
                GpuCacheStatus::SkipRasterization => continue,
            };

            let image = cache.get_image_uncached(font_system, cache_key).unwrap();
            let width = image.placement.width as usize;
            let height = image.placement.height as usize;
            let image_data = image.data;

            context.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: x as u32,
                        y: y as u32,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &image_data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(width as u32 * self.kind.num_channels() as u32),
                    rows_per_image: None,
                },
                wgpu::Extent3d {
                    width: width as u32,
                    height: height as u32,
                    depth_or_array_layers: 1,
                },
            );
        }

        self.texture_view = self
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.size = new_size;

        true
    }

    fn trim(&mut self) {
        self.glyphs_in_use.clear();
    }
}

impl Kind {
    fn num_channels(self) -> usize {
        match self {
            Kind::Mask => 1,
            Kind::Color { .. } => 4,
        }
    }

    fn texture_format(self) -> wgpu::TextureFormat {
        match self {
            Kind::Mask => wgpu::TextureFormat::R8Unorm,
            Kind::Color { srgb } => {
                if srgb {
                    wgpu::TextureFormat::Rgba8UnormSrgb
                } else {
                    wgpu::TextureFormat::Rgba8Unorm
                }
            }
        }
    }

    fn as_content_type(&self) -> ContentType {
        match self {
            Self::Mask => ContentType::Mask,
            Self::Color { .. } => ContentType::Color,
        }
    }
}

/// The color mode of a [`TextAtlas`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    /// Accurate color management.
    ///
    /// This mode will use a proper sRGB texture for colored glyphs. This will
    /// produce physically accurate color blending when rendering.
    Accurate,

    /// Web color management.
    ///
    /// This mode reproduces the color management strategy used in the Web and
    /// implemented by browsers.
    ///
    /// This entails storing glyphs colored using the sRGB color space in a
    /// linear RGB texture. Blending will not be physically accurate, but will
    /// produce the same results as most UI toolkits.
    ///
    /// This mode should be used to render to a linear RGB texture containing
    /// sRGB colors.
    Web,
}

pub(crate) struct TextAtlasBindGroup {
    pub(crate) layout: wgpu::BindGroupLayout,
    pub(crate) bind_group: wgpu::BindGroup,
}

impl TextAtlasBindGroup {
    pub fn new(context: &sumi::GraphicsContext, resources: &TextResources) -> Self {
        let layout = context
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("glyphon atlas bind group layout"),
            });

        let bind_group = Self::create_bind_group(context, &layout, resources);

        Self { layout, bind_group }
    }

    fn create_bind_group(
        context: &sumi::GraphicsContext,
        layout: &wgpu::BindGroupLayout,
        resources: &TextResources,
    ) -> wgpu::BindGroup {
        context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            &resources.color_atlas.texture_view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(
                            &resources.mask_atlas.texture_view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&resources.sampler),
                    },
                ],
                label: Some("text atlas bind group"),
            })
    }

    pub(crate) fn rebuild(&mut self, context: &sumi::GraphicsContext, resources: &TextResources) {
        self.bind_group = Self::create_bind_group(context, &self.layout, resources);
    }
}

// An atlas containing a cache of rasterized glyphs that can be rendered.
pub struct TextResources {
    pub(crate) color_atlas: InnerAtlas,
    pub(crate) mask_atlas: InnerAtlas,
    pub(crate) format: wgpu::TextureFormat,
    pub(crate) color_mode: ColorMode,
    pub(crate) sampler: wgpu::Sampler,
}

impl TextResources {
    /// Creates a new [`TextAtlas`].
    pub fn new(
        context: &sumi::GraphicsContext,
        // cache: &Cache,
        format: wgpu::TextureFormat,
    ) -> Self {
        Self::with_color_mode(context, /*cache,*/ format, ColorMode::Accurate)
    }

    /// Creates a new [`TextAtlas`] with the given [`ColorMode`].
    pub fn with_color_mode(
        context: &sumi::GraphicsContext,
        // cache: &Cache,
        format: wgpu::TextureFormat,
        color_mode: ColorMode,
    ) -> Self {
        let color_atlas = InnerAtlas::new(
            context,
            Kind::Color {
                srgb: match color_mode {
                    ColorMode::Accurate => true,
                    ColorMode::Web => false,
                },
            },
        );
        let mask_atlas = InnerAtlas::new(context, Kind::Mask);

        let sampler = context.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("text atlas sampler"),
            min_filter: wgpu::FilterMode::Nearest,
            mag_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            lod_min_clamp: 0f32,
            lod_max_clamp: 0f32,
            ..Default::default()
        });

        Self {
            color_atlas,
            mask_atlas,
            format,
            sampler,
            color_mode,
        }
    }

    pub fn trim(&mut self) {
        self.mask_atlas.trim();
        self.color_atlas.trim();
    }

    pub(crate) fn grow(
        &mut self,
        context: &GraphicsContext,
        font_system: &mut cosmic_text::FontSystem,
        cache: &mut cosmic_text::SwashCache,
        content_type: ContentType,
        scale_factor: f32,
    ) -> bool {
        let did_grow = match content_type {
            ContentType::Mask => self
                .mask_atlas
                .grow(context, font_system, cache, scale_factor),
            ContentType::Color => self
                .color_atlas
                .grow(context, font_system, cache, scale_factor),
        };

        if did_grow {
            self.rebind(context.device);
        }

        did_grow
    }

    pub(crate) fn inner_for_content_mut(&mut self, content_type: ContentType) -> &mut InnerAtlas {
        match content_type {
            ContentType::Color => &mut self.color_atlas,
            ContentType::Mask => &mut self.mask_atlas,
        }
    }

    fn rebind(&mut self, device: &wgpu::Device) {
        // self.bind_group = self.cache.create_atlas_bind_group(
        //     device,
        //     &self.color_atlas.texture_view,
        //     &self.mask_atlas.texture_view,
        // );
    }
}

pub(crate) struct GetGlyphImageResult {
    pub(crate) content_type: ContentType,
    pub(crate) top: i16,
    pub(crate) left: i16,
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) data: Vec<u8>,
}

pub(crate) struct GlyphMetadata {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) line_y: f32,
    pub(crate) scale_factor: f32,
    pub(crate) color: ColorRgba,
    pub(crate) metadata: usize,
    pub(crate) cache_key: cosmic_text::CacheKey,
}

#[derive(Clone, Copy)]
pub(crate) struct Bounds {
    pub(crate) min: i32,
    pub(crate) max: i32,
}

#[derive(Clone, Copy)]
pub(crate) struct GlyphBounds {
    pub(crate) x: Bounds,
    pub(crate) y: Bounds,
}

pub(crate) struct GlyphSystem<'a> {
    pub(crate) resources: &'a mut TextResources,
    pub(crate) cache: &'a mut cosmic_text::SwashCache,
    pub(crate) font_system: &'a mut cosmic_text::FontSystem,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct GlyphToRender {
    pub(crate) pos: [i32; 2],
    pub(crate) dim: [u16; 2],
    pub(crate) uv: [u16; 2],
    pub(crate) color: [f32; 4],
    pub(crate) content_type_with_srgb: [u16; 2],
}

/// An error that occurred while preparing text for rendering.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrepareError {
    AtlasFull,
}

impl Display for PrepareError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "Prepare error: glyph texture atlas is full")
    }
}

impl Error for PrepareError {}

/// An error that occurred while rendering text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderError {
    RemovedFromAtlas,
    ScreenResolutionChanged,
}

impl Display for RenderError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            RenderError::RemovedFromAtlas => {
                write!(
                    f,
                    "Render error: glyph no longer exists within the texture atlas"
                )
            }
            RenderError::ScreenResolutionChanged => write!(
                f,
                "Render error: screen resolution changed since last `prepare` call"
            ),
        }
    }
}

impl Error for RenderError {}

#[repr(u16)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum TextColorConversion {
    None = 0,
    ConvertToLinear = 1,
}

pub(crate) fn prepare_glyph(
    context: &sumi::GraphicsContext,
    system: &mut GlyphSystem,
    metadata: GlyphMetadata,
    bounds: GlyphBounds,
    get_glyph_image: impl FnOnce(&mut GlyphSystem) -> Option<GetGlyphImageResult>,
) -> Result<Option<VectorData>, PrepareError> {
    let details = if let Some(details) = system
        .resources
        .mask_atlas
        .glyph_cache
        .get(&metadata.cache_key)
    {
        system
            .resources
            .mask_atlas
            .glyphs_in_use
            .insert(metadata.cache_key);
        details
    } else if let Some(details) = system
        .resources
        .color_atlas
        .glyph_cache
        .get(&metadata.cache_key)
    {
        system
            .resources
            .color_atlas
            .glyphs_in_use
            .insert(metadata.cache_key);
        details
    } else {
        let Some(image) = get_glyph_image(system) else {
            return Ok(None);
        };

        let should_rasterize = image.width > 0 && image.height > 0;

        let (gpu_cache, atlas_id, inner) = if should_rasterize {
            let mut inner = system.resources.inner_for_content_mut(image.content_type);

            // Find a position in the packer
            let allocation = loop {
                match inner.try_allocate(image.width as usize, image.height as usize) {
                    Some(a) => break a,
                    None => {
                        if !system.resources.grow(
                            context,
                            system.font_system,
                            system.cache,
                            image.content_type,
                            metadata.scale_factor,
                        ) {
                            return Err(PrepareError::AtlasFull);
                        }

                        inner = system.resources.inner_for_content_mut(image.content_type);
                    }
                }
            };
            let atlas_min = allocation.rectangle.min;

            context.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &inner.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: atlas_min.x as u32,
                        y: atlas_min.y as u32,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &image.data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(image.width as u32 * inner.num_channels() as u32),
                    rows_per_image: None,
                },
                wgpu::Extent3d {
                    width: image.width as u32,
                    height: image.height as u32,
                    depth_or_array_layers: 1,
                },
            );

            (
                GpuCacheStatus::InAtlas {
                    x: atlas_min.x as u16,
                    y: atlas_min.y as u16,
                    content_type: image.content_type,
                },
                Some(allocation.id),
                inner,
            )
        } else {
            let inner = &mut system.resources.color_atlas;

            (GpuCacheStatus::SkipRasterization, None, inner)
        };

        inner.glyphs_in_use.insert(metadata.cache_key);
        // Insert the glyph into the cache and return the details reference
        inner
            .glyph_cache
            .get_or_insert(metadata.cache_key, || GlyphDetails {
                width: image.width,
                height: image.height,
                gpu_cache,
                atlas_id,
                top: image.top,
                left: image.left,
            })
    };

    let mut x = metadata.x + details.left as i32;
    let mut y =
        (metadata.line_y * metadata.scale_factor).round() as i32 + metadata.y - details.top as i32;

    let (mut atlas_x, mut atlas_y, content_type) = match details.gpu_cache {
        GpuCacheStatus::InAtlas { x, y, content_type } => (x, y, content_type),
        GpuCacheStatus::SkipRasterization => return Ok(None),
    };

    let mut width = details.width as i32;
    let mut height = details.height as i32;

    // Starts beyond right edge or ends beyond left edge
    let max_x = x + width;
    if x > bounds.x.max || max_x < bounds.x.min {
        return Ok(None);
    }

    // Starts beyond bottom edge or ends beyond top edge
    let max_y = y + height;
    if y > bounds.y.max || max_y < bounds.y.min {
        return Ok(None);
    }

    // Clip left ege
    if x < bounds.x.min {
        let right_shift = bounds.x.min - x;

        x = bounds.x.min;
        width = max_x - bounds.x.min;
        atlas_x += right_shift as u16;
    }

    // Clip right edge
    if x + width > bounds.x.max {
        width = bounds.x.max - x;
    }

    // Clip top edge
    if y < bounds.y.min {
        let bottom_shift = bounds.y.min - y;

        y = bounds.y.min;
        height = max_y - bounds.y.min;
        atlas_y += bottom_shift as u16;
    }

    // Clip bottom edge
    if y + height > bounds.y.max {
        height = bounds.y.max - y;
    }

    Ok(Some(VectorData {
        boundary: [x as f32, y as f32, width as f32, height as f32],
        shape_type: 6,
        _pad0: [0; 3],
        fill_color: to_color(metadata.color),
        border_color_left: [0.0; 4],
        border_color_top: [0.0; 4],
        border_color_right: [0.0; 4],
        border_color_bottom: [0.0; 4],
        border_widths: [0.0; 4],
        border_radii: [0.0; 4],
        box_shadow: [0.0; 4],
        gradient_info: [0; 4],
        gradient_params: [0.0; 4],
        content_type_with_srgb: [
            content_type as u16,
            match system.resources.color_mode {
                ColorMode::Accurate => TextColorConversion::ConvertToLinear,
                ColorMode::Web => TextColorConversion::None,
            } as u16,
        ],
        uv: [atlas_x as f32, atlas_y as f32],
        _pad3: 0,
    }))
}
