use glam::{Mat4, Vec3};
use iced_wgpu::wgpu::{self, util::DeviceExt, Device, SurfaceConfiguration};
use iced_winit::winit::dpi::PhysicalSize;
use std::f32::consts::{self, PI};

use crate::{
    controls::Controls,
    model::{self, DrawModel, Vertex},
    resources, texture,
};

struct Instance {
    transform: glam::Mat4,
}

impl Instance {
    fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: self.transform.to_cols_array_2d(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw {
    #[allow(dead_code)]
    model: [[f32; 4]; 4],
}

impl InstanceRaw {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    // While our vertex shader only uses locations 0, and 1 now, in later tutorials we'll
                    // be using 2, 3, and 4, for Vertex. We'll start at slot 5 not conflict with them later
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
                // for each vec4. We don't have to do this in code though.
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

pub struct ObjScene {
    pipeline: wgpu::RenderPipeline,

    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
    obj_model: model::Model,
    bind_group: wgpu::BindGroup,
    _uniform_buf: wgpu::Buffer,
    //_pipeline_wire: Option<wgpu::RenderPipeline>,
    depth_texture: texture::Texture,
    multisampled_framebuffer: wgpu::TextureView,
    sample_count: u32,
}

impl ObjScene {
    fn generate_matrix(aspect_ratio: f32, camera: Vec3, zoom: f32) -> glam::Mat4 {
        let projection = glam::Mat4::perspective_rh(consts::FRAC_PI_4, aspect_ratio, 1.0, 10_000.0);
        let view = glam::Mat4::look_at_rh(
            glam::Vec3::new(100.0f32, 100.0, 100.0) * 5.,
            glam::Vec3::ZERO,
            glam::Vec3::Z,
        ) * glam::Mat4::from_rotation_x(camera.x * 2. * PI)
            * glam::Mat4::from_rotation_y(camera.y * 2. * PI)
            * glam::Mat4::from_rotation_z(camera.z * 2. * PI)
            * glam::Mat4::from_scale(Vec3::splat(zoom));
        projection * view
    }

    fn create_multisampled_framebuffer(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        sample_count: u32,
    ) -> wgpu::TextureView {
        let multisampled_texture_extent = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };
        let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
            size: multisampled_texture_extent,
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[],
        };

        device
            .create_texture(multisampled_frame_descriptor)
            .create_view(&wgpu::TextureViewDescriptor::default())
    }
}
impl ObjScene {
    pub fn init(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        queue: &wgpu::Queue,
        sample_count: u32,
    ) -> ObjScene {
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        const SPACE_BETWEEN: f32 = 300.0;
        const NUM_LAYERS: i32 = 3;
        let instances = (-NUM_LAYERS..=NUM_LAYERS)
            .flat_map(|y| {
                (-NUM_LAYERS..=NUM_LAYERS).map(move |x| {
                    let x = SPACE_BETWEEN * (x as f32);
                    let y = SPACE_BETWEEN * (y as f32);

                    let position = Vec3 { x, y, z: 0. };

                    let rotation = if position == Vec3::ZERO {
                        Mat4::IDENTITY
                    } else {
                        Mat4::from_rotation_x(
                            PI / 4. * (position.x / (SPACE_BETWEEN * NUM_LAYERS as f32)),
                        ) * Mat4::from_rotation_y(
                            PI / 4. * (position.y / (SPACE_BETWEEN * NUM_LAYERS as f32)),
                        )
                    };

                    //rotate, then transform
                    Instance {
                        transform: Mat4::from_translation(position) * rotation,
                    }
                })
            })
            .collect::<Vec<_>>();

        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        log::warn!("Load model");
        let obj_model = resources::load_model(
            "teapot",
            "teapot_smooth.obj",
            device,
            queue,
            &texture_bind_group_layout,
        )
        .unwrap();

        // Create pipeline layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(64),
                    },
                    count: None,
                },
                //wgpu::BindGroupLayoutEntry {
                //    binding: 1,
                //    visibility: wgpu::ShaderStages::FRAGMENT,
                //    ty: wgpu::BindingType::Texture {
                //        multisampled: false,
                //        sample_type: wgpu::TextureSampleType::Uint,
                //        view_dimension: wgpu::TextureViewDimension::D2,
                //    },
                //    count: None,
                //},
            ],
        });

        let multisampled_framebuffer =
            ObjScene::create_multisampled_framebuffer(device, config, sample_count);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            push_constant_ranges: &[],
            bind_group_layouts: &[&texture_bind_group_layout, &bind_group_layout],
        });

        // Create the texture
        //let size = 256u32;
        //let texels = create_texels(size as usize);
        //let texture_extent = wgpu::Extent3d {
        //    width: size,
        //    height: size,
        //    depth_or_array_layers: 1,
        //};
        //let texture = device.create_texture(&wgpu::TextureDescriptor {
        //    label: None,
        //    size: texture_extent,
        //    mip_level_count: 1,
        //    sample_count: 1,
        //    dimension: wgpu::TextureDimension::D2,
        //    format: wgpu::TextureFormat::R8Uint,
        //    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        //    view_formats: &[],
        //});
        //let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        //queue.write_texture(
        //    texture.as_image_copy(),
        //    &texels,
        //    wgpu::ImageDataLayout {
        //        offset: 0,
        //        bytes_per_row: Some(size),
        //        rows_per_image: None,
        //    },
        //    texture_extent,
        //);

        // Create other resources
        let mx_total = Self::generate_matrix(
            config.width as f32 / config.height as f32,
            [1., 1., 1.].into(),
            1.,
        );
        let mx_ref: &[f32; 16] = mx_total.as_ref();
        let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(mx_ref),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buf.as_entire_binding(),
                },
                //wgpu::BindGroupEntry {
                //    binding: 1,
                //    resource: wgpu::BindingResource::TextureView(&texture_view),
                //},
            ],
            label: None,
        });

        let shader = device.create_shader_module(wgpu::include_wgsl!("../shader/shader.wgsl"));

        let vertex_buffers = [model::ModelVertex::desc(), InstanceRaw::desc()];

        let depth_texture =
            texture::Texture::create_depth_texture(device, config, "depth_texture", sample_count);

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &vertex_buffers,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },

            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: sample_count,
                ..Default::default()
            },
            multiview: None,
        });

        //let pipeline_wire = if device
        //    .features()
        //    .contains(wgpu::Features::POLYGON_MODE_LINE)
        //{
        //    let pipeline_wire = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        //        label: None,
        //        layout: Some(&pipeline_layout),
        //        vertex: wgpu::VertexState {
        //            module: &shader,
        //            entry_point: "vs_main",
        //            buffers: &vertex_buffers,
        //        },
        //        fragment: Some(wgpu::FragmentState {
        //            module: &shader,
        //            entry_point: "fs_wire",
        //            targets: &[Some(wgpu::ColorTargetState {
        //                format: config.view_formats[0],
        //                blend: Some(wgpu::BlendState {
        //                    color: wgpu::BlendComponent {
        //                        operation: wgpu::BlendOperation::Add,
        //                        src_factor: wgpu::BlendFactor::SrcAlpha,
        //                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        //                    },
        //                    alpha: wgpu::BlendComponent::REPLACE,
        //                }),
        //                write_mask: wgpu::ColorWrites::ALL,
        //            })],
        //        }),
        //        primitive: wgpu::PrimitiveState {
        //            front_face: wgpu::FrontFace::Ccw,
        //            cull_mode: Some(wgpu::Face::Back),
        //            polygon_mode: wgpu::PolygonMode::Line,
        //            ..Default::default()
        //        },
        //        depth_stencil: None,
        //        multisample: wgpu::MultisampleState::default(),
        //        multiview: None,
        //    });
        //    Some(pipeline_wire)
        //} else {
        //    None
        //};
        ObjScene {
            instances,
            instance_buffer,
            obj_model,
            bind_group,
            depth_texture,
            _uniform_buf: uniform_buf,
            pipeline,
            //_pipeline_wire: pipeline_wire,
            sample_count,
            multisampled_framebuffer,
        }
    }

    pub fn resize(
        &mut self,
        new_size: PhysicalSize<u32>,
        device: &Device,
        config: &SurfaceConfiguration,
    ) {
        if new_size.width > 0 && new_size.height > 0 {
            //TODO: make generic across scenes/handled at the main level?
            self.multisampled_framebuffer =
                Self::create_multisampled_framebuffer(device, config, self.sample_count);
            self.depth_texture = texture::Texture::create_depth_texture(
                device,
                config,
                "depth_texture",
                self.sample_count,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        _controls: &Controls,
        view: &wgpu::TextureView,
        camera: Vec3,
        zoom: f32,
        _show_wireframe: bool, //TODO: add pipeling for wireframe, make pipeline creation more
        //generic across scenes
        aspect: f32,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let mx_total = Self::generate_matrix(aspect, camera, zoom);
        let mx_ref: &[f32; 16] = mx_total.as_ref();
        queue.write_buffer(&self._uniform_buf, 0, bytemuck::cast_slice(mx_ref));

        let clear_color = {
            wgpu::Color {
                r: 0.9,
                g: 0.9,
                b: 0.8,
                a: 1.0,
            }
        };

        let rpass_color_attachment = if self.sample_count == 1 {
            wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_color),
                    store: wgpu::StoreOp::Store,
                },
            }
        } else {
            wgpu::RenderPassColorAttachment {
                view: &self.multisampled_framebuffer,
                resolve_target: Some(view),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_color),
                    // Storing pre-resolve MSAA data is unnecessary if it isn't used later.
                    // On tile-based GPU, avoid store can reduce your app's memory footprint.
                    store: wgpu::StoreOp::Discard,
                },
            }
        };

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(rpass_color_attachment)],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),

                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rpass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            rpass.set_pipeline(&self.pipeline);
            rpass.draw_model_instanced(
                &self.obj_model,
                0..self.instances.len() as u32,
                &self.bind_group,
            );
        }

        queue.submit(Some(encoder.finish()));
    }
}
