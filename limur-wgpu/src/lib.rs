mod gpu_vec;
mod text;
mod vector_renderer;
mod vector_resources;

use std::sync::Arc;

use glam::Vec2;
use limur::Renderer;

const MSAA_SAMPLES: u32 = 4;

pub struct WgpuRenderer {
    surface_texture_format: wgpu::TextureFormat,
    msaa_texture_view: wgpu::TextureView,

    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    // Index into shape_data of the first shape in the current vector batch.
    vector_batch_start: u32,

    resources: Resources,
    renderers: Renderers,
}

struct Renderers {}

struct Resources {
    globals_buffer: wgpu::Buffer,
}

impl WgpuRenderer {
    pub async fn new(window: Arc<winit::window::Window>) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance.create_surface(window.clone()).unwrap();

        #[cfg(target_os = "macos")]
        #[allow(invalid_reference_casting)]
        unsafe {
            if let Some(hal_surface) = surface.as_hal::<wgpu::hal::api::Metal>() {
                let raw = (&*hal_surface) as *const wgpu::hal::metal::Surface
                    as *mut wgpu::hal::metal::Surface;
                (*raw).present_with_transaction = true;
            }
        }

        let (adapter, device, queue) = request_device(&instance, &surface).await;

        let size = window.inner_size();
        let view_size = Vec2::new(size.width as f32, size.height as f32);

        let caps = surface.get_capabilities(&adapter);
        let surface_texture_format = caps.formats[0];
        // let surface_texture_format = caps.formats[0].remove_srgb_suffix();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_texture_format,
            width: view_size.x as u32,
            height: view_size.y as u32,
            present_mode: wgpu::PresentMode::AutoNoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            desired_maximum_frame_latency: 2,
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("MSAA Texture"),
            size: wgpu::Extent3d {
                width: view_size.x as u32,
                height: view_size.y as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: MSAA_SAMPLES,
            dimension: wgpu::TextureDimension::D2,
            format: surface_texture_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let msaa_texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let globals_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Screen Size Uniform"),
            size: 8, // vec2<f32>
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            surface_texture_format,
            msaa_texture_view,
            surface,
            device,
            queue,
            config,
            vector_batch_start: 0,
            resources: Resources { globals_buffer },
            renderers: Renderers {},
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    screen_size: [f32; 2],
}

pub async fn request_device(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface<'_>,
) -> (wgpu::Adapter, wgpu::Device, wgpu::Queue) {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::from_env()
                .unwrap_or(wgpu::PowerPreference::HighPerformance),
            force_fallback_adapter: false,
            compatible_surface: Some(surface),
        })
        .await
        .expect("No suitable GPU adapters found on the system!");

    let adapter_info = adapter.get_info();

    log::debug!("Using {} ({:?})", adapter_info.name, adapter_info.backend);

    let base_dir = std::env::var("CARGO_MANIFEST_DIR");
    let _trace_path = if let Ok(base_dir) = base_dir {
        Some(std::path::PathBuf::from(&base_dir).join("WGPU_TRACE_ERROR"))
    } else {
        None
    };

    let res = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: adapter.features(),
            required_limits: adapter.limits(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
            experimental_features: unsafe { wgpu::ExperimentalFeatures::enabled() },
        })
        .await;

    match res {
        Err(err) => {
            panic!("request_device failed: {err:?}");
        }
        Ok((device, queue)) => (adapter, device, queue),
    }
}

pub struct GraphicsContext<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub surface_texture_format: wgpu::TextureFormat,
    pub view_size: Vec2,
    pub scale_factor: f32,
    pub sample_count: u32,
}

impl Renderer for WgpuRenderer {
    fn process_commands(
        &mut self,
        view: &limur::View,
        composition_layers: &[limur::render::RenderCompositionLayer],
        fill_color: Option<limur::ColorRgba>,
        fonts: &mut limur::text::FontResources,
        text: &mut limur::text::TextsResources,
        assets: &limur::assets::Assets,
    ) {
        let (optimal, output) = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => (true, surface_texture),
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                log::warn!("Get current surface texture: suboptimal");
                (false, surface_texture)
            }
            wgpu::CurrentSurfaceTexture::Timeout => panic!("surface texture: timeout"),
            wgpu::CurrentSurfaceTexture::Occluded => panic!("surface texture: occluded"),
            wgpu::CurrentSurfaceTexture::Outdated => panic!("surface texture: outdated"),
            wgpu::CurrentSurfaceTexture::Lost => panic!("surface texture: lost"),
            wgpu::CurrentSurfaceTexture::Validation => panic!("surface texture: validation error"),
        };
        let surface_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let width = view.physical_size.width;
        let height = view.physical_size.height;

        self.queue.write_buffer(
            &self.resources.globals_buffer,
            0,
            bytemuck::bytes_of(&Globals {
                screen_size: [width as f32, height as f32],
            }),
        );

        for layer in composition_layers {
            for command in &layer.commands {}
        }

        output.present();

        if !optimal {
            self.surface.configure(&self.device, &self.config);
        }
    }
}
