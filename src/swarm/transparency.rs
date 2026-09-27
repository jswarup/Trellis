// transparency.rs --------------------------------------------------------------------------------
//! Weighted blended OIT. Targets persist per viewport; camera/opacity changes allocate nothing.

use crate::fleck::geometry::GeometryVertex;

//-------------------------------------------------------------------------------------------------

pub( super) struct TransparencyTargets
{
    _Accumulation: wgpu::TextureView,
    _Revealage:    wgpu::TextureView,
    _Resolve:      wgpu::BindGroup,
}

pub( super) struct Transparency
{
    _Layout:  wgpu::BindGroupLayout,
    _Mesh:    wgpu::RenderPipeline,
    _Points:  wgpu::RenderPipeline,
    _Resolve: wgpu::RenderPipeline,
}

impl Transparency
{
    pub fn New( device: &wgpu::Device, geometryLayout: &wgpu::PipelineLayout,
               shader: &wgpu::ShaderModule)
               -> Self
    {
        let additive = wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One,
                                              dst_factor: wgpu::BlendFactor::One,
                                              operation:  wgpu::BlendOperation::Add, };
        let reveal = wgpu::BlendComponent { src_factor: wgpu::BlendFactor::Zero,
                                            dst_factor: wgpu::BlendFactor::OneMinusSrc,
                                            operation:  wgpu::BlendOperation::Add, };
        let pipeline = |entry, fragment, step| {
            device.create_render_pipeline( &wgpu::RenderPipelineDescriptor {
            label: Some( "Transparent geometry"), layout: Some( geometryLayout),
            vertex: wgpu::VertexState { module: shader, entry_point: Some( entry),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GeometryVertex>() as u64, step_mode: step,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32, 2 => Float32x4],
                }] },
            fragment: Some( wgpu::FragmentState { module: shader, entry_point: Some( fragment),
                compilation_options: Default::default(), targets: &[
                    Some( wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba16Float,
                        blend: Some( wgpu::BlendState { color: additive, alpha: additive }),
                        write_mask: wgpu::ColorWrites::ALL }),
                    Some( wgpu::ColorTargetState { format: wgpu::TextureFormat::R16Float,
                        blend: Some( wgpu::BlendState { color: reveal, alpha: reveal }),
                        write_mask: wgpu::ColorWrites::ALL }),
                ] }),
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: Some( wgpu::DepthStencilState { format: super::DEPTH_FORMAT,
                depth_write_enabled: false, depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(), bias: Default::default() }),
            multisample: Default::default(), multiview: None, cache: None,
        })
        };
        let mesh = pipeline( "vs_mesh", "fs_transparent", wgpu::VertexStepMode::Vertex);
        let points = pipeline( "vs_point",
                              "fs_point_transparent",
                              wgpu::VertexStepMode::Instance);
        let entry = |binding| {
            wgpu::BindGroupLayoutEntry {
            binding, visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false },
                view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None,
        }
        };
        let layout = device.create_bind_group_layout( &wgpu::BindGroupLayoutDescriptor {
            label: Some( "Transparency resolve"), entries: &[entry( 0), entry( 1)],
        });
        let resolveLayout = device.create_pipeline_layout( &wgpu::PipelineLayoutDescriptor {
            label: Some( "Transparency resolve"), bind_group_layouts: &[&layout], push_constant_ranges: &[],
        });
        let resolveShader = device.create_shader_module( wgpu::ShaderModuleDescriptor {
            label: Some( "Transparency resolve"),
            source: wgpu::ShaderSource::Wgsl( include_str!( "../symph/transparency.wgsl").into()),
        });
        let resolve = device.create_render_pipeline( &wgpu::RenderPipelineDescriptor {
            label: Some( "Transparency resolve"), layout: Some( &resolveLayout),
            vertex: wgpu::VertexState { module: &resolveShader, entry_point: Some( "vs_resolve"),
                compilation_options: Default::default(), buffers: &[] },
            fragment: Some( wgpu::FragmentState { module: &resolveShader, entry_point: Some( "fs_resolve"),
                compilation_options: Default::default(),
                targets: &[Some( wgpu::ColorTargetState { format: super::COLOR_FORMAT,
                    blend: Some( wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })] }),
            primitive: Default::default(), depth_stencil: None, multisample: Default::default(),
            multiview: None, cache: None,
        });
        Self { _Layout:  layout,
               _Mesh:    mesh,
               _Points:  points,
               _Resolve: resolve, }
    }

    pub fn Targets( &self, device: &wgpu::Device, size: [u32; 2]) -> TransparencyTargets
    {
        let texture = |format| {
            device.create_texture( &wgpu::TextureDescriptor {
            label: Some( "Transparency target"),
            size: wgpu::Extent3d { width: size[0], height: size[1], depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        }).create_view( &Default::default())
        };
        let accumulation = texture( wgpu::TextureFormat::Rgba16Float);
        let revealage = texture( wgpu::TextureFormat::R16Float);
        let resolve = device.create_bind_group( &wgpu::BindGroupDescriptor {
            label: Some( "Transparency textures"), layout: &self._Layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView( &accumulation) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView( &revealage) },
            ],
        });
        TransparencyTargets { _Accumulation: accumulation,
                              _Revealage:    revealage,
                              _Resolve:      resolve, }
    }

    pub fn Draw( &self, encoder: &mut wgpu::CommandEncoder, targets: &TransparencyTargets,
                view: &super::GpuView, points: bool)
    {
        let attachment = |texture, clear| {
            Some( wgpu::RenderPassColorAttachment {
            view: texture, depth_slice: None, resolve_target: None,
            ops: wgpu::Operations { load: wgpu::LoadOp::Clear( clear), store: wgpu::StoreOp::Store },
        })
        };
        {
            let mut pass = encoder.begin_render_pass( &wgpu::RenderPassDescriptor {
                label: Some( "Transparent accumulation"),
                color_attachments: &[
                    attachment( &targets._Accumulation, wgpu::Color::TRANSPARENT),
                    attachment( &targets._Revealage, wgpu::Color::WHITE),
                ],
                depth_stencil_attachment: Some( wgpu::RenderPassDepthStencilAttachment {
                    view: &view._Depth, depth_ops: Some( wgpu::Operations {
                        load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store }), stencil_ops: None,
                }),
                timestamp_writes: None, occlusion_query_set: None,
            });
            pass.set_bind_group( 0, &view._BindGroup, &[]);
            pass.set_vertex_buffer( 0, view._Vertices.slice( ..));
            if points {
                pass.set_pipeline( &self._Points);
                pass.draw( 0..6, 0..view._VertexCount);
            } else {
                pass.set_pipeline( &self._Mesh);
                pass.set_index_buffer( view._Triangles.slice( ..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed( 0..view._IndexCount, 0, 0..1);
            }
        }
        let mut pass = encoder.begin_render_pass( &wgpu::RenderPassDescriptor {
            label: Some( "Transparent resolve"),
            color_attachments: &[Some( wgpu::RenderPassColorAttachment {
                view: &view._Target, depth_slice: None, resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None,
        });
        pass.set_pipeline( &self._Resolve);
        pass.set_bind_group( 0, &targets._Resolve, &[]);
        pass.draw( 0..3, 0..1);
    }
}

//-------------------------------------------------------------------------------------------------
