// labels.rs --------------------------------------------------------------------------------------
//! Static world-space label atlas; shares the viewport camera and hierarchy depth.

use crate::fleck::geometry::{GeometryLabels, LabelVertex};

//-------------------------------------------------------------------------------------------------

pub( super) struct GpuLabels
{
    _Vertices: wgpu::Buffer,
    _Atlas:    wgpu::BindGroup,
}

pub( super) struct LabelRenderer
{
    _Layout:   wgpu::BindGroupLayout,
    _Sampler:  wgpu::Sampler,
    _Pipeline: wgpu::RenderPipeline,
}

impl LabelRenderer
{
    pub fn New( device: &wgpu::Device, cameraLayout: &wgpu::BindGroupLayout) -> Self
    {
        let layout = device.create_bind_group_layout( &wgpu::BindGroupLayoutDescriptor {
            label: Some( "Label atlas layout"), entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler( wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let pipelineLayout = device.create_pipeline_layout( &wgpu::PipelineLayoutDescriptor {
            label: Some( "Label pipeline layout"), bind_group_layouts: &[cameraLayout, &layout],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module( wgpu::ShaderModuleDescriptor {
            label: Some( "World-space labels"),
            source: wgpu::ShaderSource::Wgsl( include_str!( "../symph/labels.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline( &wgpu::RenderPipelineDescriptor {
            label: Some( "World-space labels"), layout: Some( &pipelineLayout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some( "vs_label"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<LabelVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2],
                }] },
            fragment: Some( wgpu::FragmentState { module: &shader, entry_point: Some( "fs_label"),
                compilation_options: Default::default(),
                targets: &[Some( wgpu::ColorTargetState { format: super::COLOR_FORMAT,
                    blend: Some( wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })] }),
            primitive: Default::default(),
            depth_stencil: Some( wgpu::DepthStencilState { format: super::DEPTH_FORMAT,
                depth_write_enabled: true, depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(), bias: Default::default() }),
            multisample: Default::default(), multiview: None, cache: None,
        });
        let sampler = device.create_sampler( &wgpu::SamplerDescriptor { mag_filter:
                                                                           wgpu::FilterMode::Linear,
                                                                       min_filter:
                                                                           wgpu::FilterMode::Linear,
                                                                       ..Default::default() });
        Self { _Layout:   layout,
               _Sampler:  sampler,
               _Pipeline: pipeline, }
    }

    pub fn Upload( &self, device: &wgpu::Device, queue: &wgpu::Queue, labels: &GeometryLabels)
                  -> GpuLabels
    {
        let [width, height] = labels.Size();
        let texture = device.create_texture( &wgpu::TextureDescriptor {
            label: Some( "Label atlas"), size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[],
        });
        queue.write_texture( wgpu::TexelCopyTextureInfo { texture:   &texture,
                                                         mip_level: 0,
                                                         origin:    wgpu::Origin3d::ZERO,
                                                         aspect:    wgpu::TextureAspect::All, },
                            labels.Pixels().into(),
                            wgpu::TexelCopyBufferLayout { offset:         0,
                                                          bytes_per_row:  Some( width),
                                                          rows_per_image: Some( height), },
                            wgpu::Extent3d { width,
                                             height,
                                             depth_or_array_layers: 1 });
        let view = texture.create_view( &Default::default());
        let atlas = device.create_bind_group( &wgpu::BindGroupDescriptor {
            label: Some( "Label atlas"), layout: &self._Layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView( &view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler( &self._Sampler) },
            ],
        });
        GpuLabels { _Vertices: super::ViewportRenderer::Upload( device,
                                                               "Label vertices",
                                                               labels.Vertices(),
                                                               wgpu::BufferUsages::VERTEX),
                    _Atlas:    atlas, }
    }

    pub fn Draw( &self, pass: &mut wgpu::RenderPass<'_>, labels: &GpuLabels, count: u32)
    {
        pass.set_pipeline( &self._Pipeline);
        pass.set_bind_group( 1, &labels._Atlas, &[]);
        pass.set_vertex_buffer( 0, labels._Vertices.slice( ..));
        pass.draw( 0..count, 0..1);
    }
}

//-------------------------------------------------------------------------------------------------
