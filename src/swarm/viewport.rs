// viewport.rs ------------------------------------------------------------------------------------
//! Persistent GPU geometry and per-view targets. The host supplies its device, queue and encoder.
use crate::fleck::geometry::{ GeometryAsset, GeometryVertex };
use crate::silo::Arr;
use std::collections::BTreeMap;
use std::sync::{ Arc, Weak };
use wgpu::util::DeviceExt;
const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

//-------------------------------------------------------------------------------------------------

#[derive( Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode
{
    Solid,
    Wireframe,
    ShadedWire,
    Points,
}
#[repr( C)]
#[derive( Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ViewUniforms
{
    _Matrix:        [f32; 16],
    _Settings:      [f32; 4],
}
pub struct ViewFrame
{
    _Uniforms:      ViewUniforms,
    _Size:          [u32; 2],
    _Region:        [f32; 4],
    _Clear:         [f32; 4],
    _Mode:          RenderMode,
}
impl ViewFrame
{
    pub fn  New(
        matrix: [f32; 16], size: [u32; 2], region: [f32; 4], clear: [f32; 4], mode: RenderMode,
        colorMode: u32, pointSize: f32,
    ) -> Self
    {
        Self {
            _Uniforms:          ViewUniforms {
                _Matrix:        matrix,
                _Settings:      [size[0] as f32, size[1] as f32, pointSize, colorMode as f32],
            },
            _Size:      size,
            _Region:    region,
            _Clear:     clear,
            _Mode:      mode,
        }
    }
}
struct GpuView
{
    _Owner:         Weak< GeometryAsset>,
    _Vertices:      wgpu::Buffer,
    _Triangles:     wgpu::Buffer,
    _Edges:         wgpu::Buffer,
    _VertexCount:   u32,
    _IndexCount:    u32,
    _EdgeCount:     u32,
    _Uniforms:      wgpu::Buffer,
    _BindGroup:     wgpu::BindGroup,
    _Region:        wgpu::Buffer,
    _Target:        wgpu::TextureView,
    _Depth:         wgpu::TextureView,
    _Composite:     wgpu::BindGroup,
    _Frame:         ViewFrame,
}
pub struct ViewportRenderer
{
    _Views:         BTreeMap< u64, GpuView>,
    _Layout:        wgpu::BindGroupLayout,
    _QuadLayout:    wgpu::BindGroupLayout,
    _Sampler:       wgpu::Sampler,
    _Solid:         wgpu::RenderPipeline,
    _Wire:          wgpu::RenderPipeline,
    _Points:        wgpu::RenderPipeline,
    _Quad:          wgpu::RenderPipeline,
    _Uploads:       u64,
}
impl ViewportRenderer
{
    pub fn  New( device: &wgpu::Device, format: wgpu::TextureFormat) -> Self
    {
        let     layout = device.create_bind_group_layout( &wgpu::BindGroupLayoutDescriptor {
            label: Some( "Geometry uniforms"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let     pipelineLayout = device.create_pipeline_layout( &wgpu::PipelineLayoutDescriptor {
            label: Some( "Geometry layout"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let     shader = device.create_shader_module( wgpu::ShaderModuleDescriptor {
            label: Some( "Geometry shader"),
            source: wgpu::ShaderSource::Wgsl( include_str!( "../symph/viewport.wgsl").into()),
        });
        let     pipeline = |label, entry, fragment, topology, step, write, bias| {
            device.create_render_pipeline( &wgpu::RenderPipelineDescriptor {
                label: Some( label), layout: Some( &pipelineLayout),
                vertex: wgpu::VertexState { module: &shader, entry_point: Some( entry), compilation_options: Default::default(),
                    buffers: &[wgpu::VertexBufferLayout { array_stride: std::mem::size_of::< GeometryVertex>() as u64,
                        step_mode: step, attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32, 2 => Float32x4] }] },
                fragment: Some( wgpu::FragmentState { module: &shader, entry_point: Some( fragment), compilation_options: Default::default(),
                    targets: &[Some( wgpu::ColorTargetState { format: COLOR_FORMAT, blend: Some( wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })] }),
                primitive: wgpu::PrimitiveState { topology, cull_mode: None, ..Default::default() },
                depth_stencil: Some( wgpu::DepthStencilState { format: DEPTH_FORMAT, depth_write_enabled: write,
                    depth_compare: wgpu::CompareFunction::LessEqual, stencil: Default::default(),
                    bias: wgpu::DepthBiasState { constant: bias, slope_scale: 0.0, clamp: 0.0 } }),
                multisample: Default::default(), multiview: None, cache: None,
            })
        };
        let     solid = pipeline(
            "Solid mesh",
            "vs_mesh",
            "fs_mesh",
            wgpu::PrimitiveTopology::TriangleList,
            wgpu::VertexStepMode::Vertex,
            true,
            0,
        );
        let     wire = pipeline(
            "Mesh edges",
            "vs_mesh",
            "fs_wire",
            wgpu::PrimitiveTopology::LineList,
            wgpu::VertexStepMode::Vertex,
            true,
            -2,
        );
        let     points = pipeline(
            "Point sprites",
            "vs_point",
            "fs_point",
            wgpu::PrimitiveTopology::TriangleList,
            wgpu::VertexStepMode::Instance,
            true,
            0,
        );
        let     quadLayout = device.create_bind_group_layout( &wgpu::BindGroupLayoutDescriptor {
            label: Some( "Viewport composite layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler( wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let     quadPipelineLayout = device.create_pipeline_layout( &wgpu::PipelineLayoutDescriptor {
            label: Some( "Viewport composition"),
            bind_group_layouts: &[&quadLayout],
            push_constant_ranges: &[],
        });
        let     quadShader = device.create_shader_module( wgpu::ShaderModuleDescriptor {
            label: Some( "Viewport composite shader"),
            source: wgpu::ShaderSource::Wgsl( include_str!( "../symph/composite.wgsl").into()),
        });
        let     quad = device.create_render_pipeline( &wgpu::RenderPipelineDescriptor {
            label: Some( "Viewport composite"),
            layout: Some( &quadPipelineLayout),
            vertex: wgpu::VertexState {
                module: &quadShader,
                entry_point: Some( "vs_quad"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some( wgpu::FragmentState {
                module: &quadShader,
                entry_point: Some( if format.is_srgb() {
                    "fs_quad"
                }
                else
                {
                    "fs_encoded"
                }),
                compilation_options: Default::default(),
                targets: &[Some( wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        Self {
            _Views:         BTreeMap::new(),
            _Layout:        layout,
            _QuadLayout:    quadLayout,
            _Sampler:       device.create_sampler( &wgpu::SamplerDescriptor {
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
            _Solid:     solid,
            _Wire:      wire,
            _Points:    points,
            _Quad:      quad,
            _Uploads:   0,
        }
    }
    pub fn  Prepare(
        &mut self, id: u64, asset: &Arc< GeometryAsset>, device: &wgpu::Device, queue: &wgpu::Queue,
        frame: ViewFrame,
    ) -> Result< (), String>
    {
        let     limits = device.limits();
        if frame._Size.contains( &0)
        {
            return Ok( ());
        }
        if frame
            ._Size
            .iter()
            .any( |s| *s > limits.max_texture_dimension_2d)
        {
            return Err( "Viewport exceeds this GPU's texture limit.".into());
        }
        if u64::from( asset.VertexCount()) * std::mem::size_of::< GeometryVertex>() as u64
            > limits.max_buffer_size
            || u64::from( asset.Triangles().Size()) * 12 > limits.max_buffer_size
            || u64::from( asset.Edges().Size()) * 8 > limits.max_buffer_size
            || asset.Triangles().Size() > u32::MAX / 3
            || asset.Edges().Size() > u32::MAX / 2 {
            return Err(
                "Geometry exceeds this GPU's buffer limit. Split the file into smaller parts."
                    .into(),
            );
        }
        let     replace = self
            ._Views
            .get( &id)
            .is_none_or( |v| !v._Owner.ptr_eq( &Arc::downgrade( asset)));
        if replace
        {
            let     vertices = Self::Upload(
                device,
                "Geometry vertices",
                asset.Vertices(),
                wgpu::BufferUsages::VERTEX,
            );
            let     triangles = Self::Upload(
                device,
                "Geometry triangles",
                asset.Triangles(),
                wgpu::BufferUsages::INDEX,
            );
            let     edges = Self::Upload(
                device,
                "Geometry edges",
                asset.Edges(),
                wgpu::BufferUsages::INDEX,
            );
            let     uniforms = device.create_buffer_init( &wgpu::util::BufferInitDescriptor {
                label: Some( "Camera uniforms"),
                contents: bytemuck::bytes_of( &frame._Uniforms),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
            let     bindGroup = device.create_bind_group( &wgpu::BindGroupDescriptor {
                label: Some( "Camera"),
                layout: &self._Layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniforms.as_entire_binding(),
                }],
            });
            let     region = device.create_buffer_init( &wgpu::util::BufferInitDescriptor {
                label: Some( "Viewport region"),
                contents: bytemuck::bytes_of( &frame._Region),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
            let     ( target, depth, composite) = self.Targets( device, frame._Size, &region);
            self._Views.insert(
                id,
                GpuView {
                    _Owner:         Arc::downgrade( asset),
                    _Vertices:      vertices,
                    _Triangles:     triangles,
                    _Edges:         edges,
                    _VertexCount:   asset.VertexCount(),
                    _IndexCount:    asset.Triangles().Size() * 3,
                    _EdgeCount:     asset.Edges().Size() * 2,
                    _Uniforms:      uniforms,
                    _BindGroup:     bindGroup,
                    _Region:        region,
                    _Target:        target,
                    _Depth:         depth,
                    _Composite:     composite,
                    _Frame:         frame,
                },
            );
            self._Uploads += 1;
        }
        else
        {
            if self._Views[&id]._Frame._Size != frame._Size
            {
                let     ( target, depth, composite) =
                    self.Targets( device, frame._Size, &self._Views[&id]._Region);
                let     view = self._Views.get_mut( &id).unwrap();
                view._Target = target;
                view._Depth = depth;
                view._Composite = composite;
            }
            let     view = self._Views.get_mut( &id).unwrap();
            queue.write_buffer( &view._Uniforms, 0, bytemuck::bytes_of( &frame._Uniforms));
            queue.write_buffer( &view._Region, 0, bytemuck::bytes_of( &frame._Region));
            view._Frame = frame;
        }
        Ok( ())
    }
    fn  Upload< T: bytemuck::Pod>(
        device: &wgpu::Device, label: &str, data: Arr< '_, T>, usage: wgpu::BufferUsages,
    ) -> wgpu::Buffer
    {
        // Arr borrows initialized contiguous storage; Pod guarantees a padding-free byte representation.
        let     bytes = bytemuck::cast_slice( data.into());
        device.create_buffer_init( &wgpu::util::BufferInitDescriptor {
            label: Some( label),
            contents: if bytes.is_empty() { &[0; 4] } else { bytes },
            usage,
        })
    }
    fn  Targets(
        &self, device: &wgpu::Device, size: [u32; 2], region: &wgpu::Buffer,
    ) -> ( wgpu::TextureView, wgpu::TextureView, wgpu::BindGroup)
    {
        let     texture = |label, format, usage| {
            device
                .create_texture( &wgpu::TextureDescriptor {
                    label: Some( label),
                    size: wgpu::Extent3d {
                        width: size[0],
                        height: size[1],
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                })
                .create_view( &Default::default())
        };
        let     target = texture(
            "Viewport color",
            COLOR_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let     depth = texture(
            "Viewport depth",
            DEPTH_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        );
        let     composite = device.create_bind_group( &wgpu::BindGroupDescriptor {
            label: Some( "Viewport texture"),
            layout: &self._QuadLayout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView( &target),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler( &self._Sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: region.as_entire_binding(),
                },
            ],
        });
        ( target, depth, composite)
    }
    pub fn  Render(
        &self, id: u64, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView,
        clip: [u32; 4],
    )
    {
        let     Some( view) = self._Views.get( &id) else {
            return;
        };
        if clip[2] == 0 || clip[3] == 0
        {
            return;
        }
        {
            let     clear = view._Frame._Clear;
            let     mut pass = encoder.begin_render_pass( &wgpu::RenderPassDescriptor {
                label: Some( "Geometry depth pass"),
                color_attachments: &[Some( wgpu::RenderPassColorAttachment {
                    view: &view._Target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear( wgpu::Color {
                            r: f64::from( clear[0]),
                            g: f64::from( clear[1]),
                            b: f64::from( clear[2]),
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some( wgpu::RenderPassDepthStencilAttachment {
                    view: &view._Depth,
                    depth_ops: Some( wgpu::Operations {
                        load: wgpu::LoadOp::Clear( 1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_bind_group( 0, &view._BindGroup, &[]);
            pass.set_vertex_buffer( 0, view._Vertices.slice( ..));
            let     mode = view._Frame._Mode;
            if mode == RenderMode::Points || view._IndexCount == 0
            {
                pass.set_pipeline( &self._Points);
                pass.draw( 0..6, 0..view._VertexCount);
            }
            else
            {
                if mode != RenderMode::Wireframe
                {
                    pass.set_pipeline( &self._Solid);
                    pass.set_index_buffer( view._Triangles.slice( ..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed( 0..view._IndexCount, 0, 0..1);
                }
                if mode != RenderMode::Solid
                {
                    pass.set_pipeline( &self._Wire);
                    pass.set_index_buffer( view._Edges.slice( ..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed( 0..view._EdgeCount, 0, 0..1);
                }
            }
        }
        let     mut pass = encoder.begin_render_pass( &wgpu::RenderPassDescriptor {
            label: Some( "Viewport UI composite"),
            color_attachments: &[Some( wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_scissor_rect( clip[0], clip[1], clip[2], clip[3]);
        pass.set_pipeline( &self._Quad);
        pass.set_bind_group( 0, &view._Composite, &[]);
        pass.draw( 0..3, 0..1);
    }
    pub fn  Trim( &mut self)
    {
        self._Views.retain( |_, v| v._Owner.strong_count() != 0);
    }
    pub fn  UploadCount( &self) -> u64
    {
        self._Uploads
    }
    pub fn  ResidentViews( &self) -> usize
    {
        self._Views.len()
    }
}

//-------------------------------------------------------------------------------------------------
