use crate::model::{self, Model, ModelVertex};
use crate::texture;
use glam::{Vec3, Vec3Swizzles};
use iced_wgpu::wgpu::{self, util::DeviceExt};
use image::{ImageBuffer, Luma};
use noise::utils::*;
use noise::{utils::PlaneMapBuilder, Fbm, Perlin};

pub struct Chunk {
    pub position: Vec3,
    pub model: Model,
}

impl Chunk {
    pub fn new(
        x_index: i32,
        y_index: i32,
        fbm: &Fbm<Perlin>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        //// Terrain gen
        let height_map_res = 64;
        let chunk_width = 100.;

        let offset = Vec3::new(x_index as f32, y_index as f32, 0.);
        let noise_scale = 0.2;
        let z_scale = Vec3::new(1.0, 1.0, 5.);
        let noise_offset = offset * noise_scale as f32;
        // to make sure that on the edges of our texture, we sample values that will line up with
        // the textures on the next chunk, we need to sample a slightly larger region so that our
        // sampling points fall at the right value
        let sample_ratio = height_map_res as f64 / (height_map_res - 1) as f64;

        let noise = PlaneMapBuilder::new(fbm)
            .set_size(height_map_res, height_map_res)
            .set_x_bounds(
                noise_offset.x as f64,
                noise_offset.x as f64 + (noise_scale * sample_ratio),
            )
            .set_y_bounds(
                noise_offset.y as f64,
                noise_offset.y as f64 + (noise_scale * sample_ratio),
            )
            .build();
        let image: ImageBuffer<Luma<f32>, Vec<_>> = ImageBuffer::from_vec(
            height_map_res as u32 - 1,
            height_map_res as u32 - 1,
            noise
                .iter()
                .enumerate()
                //cut off the last row/column
                //it will be a repeat of the next chunks first row/column
                .filter_map(|(i, f)| {
                    if (i + 1) % height_map_res == 0 {
                        None
                    } else {
                        Some(0.5 + *f as f32)
                    }
                })
                .collect(),
        )
        .expect("valid image");

        let vertices = [
            Vec3::new(0.0, 0.0, noise.get_value(0, 0) as f32),
            Vec3::new(1.0, 0.0, noise.get_value(height_map_res - 1, 0) as f32),
            Vec3::new(0.0, 1.0, noise.get_value(0, height_map_res - 1) as f32),
            Vec3::new(
                1.0,
                1.0,
                noise.get_value(height_map_res - 1, height_map_res - 1) as f32,
            ),
        ];
        let vertices: Vec<_> = vertices
            .iter()
            .map(|v| ModelVertex {
                position: ((v + offset) * chunk_width * z_scale).into(),
                tex_coords: v.xy().into(),
                normal: [0., 0., 1.0], //TODO: calculate proper normals
            })
            .collect();
        let indices = vec![0, 1, 2, 3, 2, 1];

        let name = "terrain".to_string();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Vertex Buffer", name)),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Index Buffer", name)),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let diffuse_texture =
            texture::Texture::from_image(device, queue, &image.into(), Some("Height Map Texture"))
                .expect("valid texture");

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
            ],
            label: None,
        });
        let mesh = model::Mesh {
            name: name.clone(),
            vertex_buffer,
            index_buffer,
            num_elements: indices.len() as u32,
            material: 0,
        };
        let height_map = model::Material {
            name: name.clone(),
            diffuse_texture,
            bind_group,
        };
        let model = model::Model {
            meshes: vec![mesh],
            materials: vec![height_map],
        };

        log::info!("Mesh: {}", name);
        Self {
            position: offset * chunk_width,
            model,
        }
    }
}
