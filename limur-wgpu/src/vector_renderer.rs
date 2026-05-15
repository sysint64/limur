use crate::text::TextAtlasBindGroup;

pub(crate) struct VectorRenderer {
    render_pipeline: wgpu::RenderPipeline,
    pub(crate) bind_group: VectorBindGroup,
    pub(crate) text_atlas_bind_group: TextAtlasBindGroup,
}

struct VectorBindGroup {
    layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}
