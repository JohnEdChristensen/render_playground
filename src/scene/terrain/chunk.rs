use crate::model::{self, Model, ModelVertex};
use crate::texture;
use glam::{Vec3, Vec3Swizzles};
use iced_wgpu::wgpu::{self, util::DeviceExt};
use image::{ImageBuffer, Luma, Rgb};
use ndarray::{s, Array2, IntoNdProducer};
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
        let height_map_res = 8;
        let chunk_width = 100.;

        let offset = Vec3::new(x_index as f32, y_index as f32, 0.);
        let z_scale = Vec3::new(1.0, 1.0, 5.);
        let noise_scale = 0.2;

        // to make sure that on the edges of our texture, we sample values that will line up with
        // the textures on the next chunk, we need to sample a slightly larger region so that our
        // sampling points fall on the right values
        let delta = 1.0 / height_map_res as f64;

        let noise = PlaneMapBuilder::new(fbm)
            .set_size(height_map_res + 2, height_map_res + 2)
            .set_x_bounds(
                (offset.x as f64 - delta) * noise_scale,
                (offset.x as f64 + (1. + delta)) * noise_scale,
            )
            .set_y_bounds(
                (offset.y as f64 - delta) * noise_scale,
                (offset.y as f64 + (1. + delta)) * noise_scale,
            )
            .build();

        let noise_2d: Array2<f64> = Array2::<f64>::from_shape_vec(
            (height_map_res + 2, height_map_res + 2),
            noise.into_iter().collect::<Vec<_>>(),
        )
        .unwrap();

        #[allow(clippy::reversed_empty_ranges)] //false positive
        let inner_noise = noise_2d.slice(s![1..-1, 1..-1]);

        let z_tex = inner_noise.map(|f| *f as f32);
        let z_tex = z_tex.flatten();

        let normal_map: Vec<_> = noise_2d
            .windows((3, 3))
            .into_producer()
            .into_iter()
            .flat_map(|a| {
                //let [nw, n, ne] = a.slice(s![0, 0..3]).to_vec()[..];
                //let nw = a[(0, 0)];
                let n = a[(0, 1)] as f32;
                //let ne = a[(2, 0)];
                let w = a[(1, 0)] as f32;
                //let center = a[(1, 1)];
                let e = a[(1, 2)] as f32;
                //let sw = a[(0, 2)];
                let s = a[(2, 1)] as f32;
                //let se = a[(2, 2)];
                let x = Vec3::new(delta as f32 * 2., 0.0, (e - w) * z_scale.z);
                let y = Vec3::new(0.0, delta as f32 * 2., (s - n) * z_scale.z);
                let norm = x.cross(y).normalize();
                // avoid image transform mangling the vector, needs to be reversed
                // in the shader
                ((norm + 1.0) / 2.0).to_array()
            })
            .collect();

        let height_image: ImageBuffer<Luma<f32>, Vec<_>> = ImageBuffer::from_vec(
            height_map_res as u32,
            height_map_res as u32,
            (z_tex + 0.5).to_vec(),
        )
        .expect("valid image");
        let normal_image: ImageBuffer<Rgb<f32>, Vec<_>> =
            ImageBuffer::from_vec(height_map_res as u32, height_map_res as u32, normal_map)
                .expect("valid image");

        let start = 1;
        let end = height_map_res + 1;
        let vertices = [
            Vec3::new(0.0, 0.0, *noise_2d.get((start, start)).unwrap() as f32),
            Vec3::new(1.0, 0.0, *noise_2d.get((start, end)).unwrap() as f32),
            Vec3::new(0.0, 1.0, *noise_2d.get((end, start)).unwrap() as f32),
            Vec3::new(1.0, 1.0, *noise_2d.get((end, end)).unwrap() as f32),
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

        let height_texture = texture::Texture::from_image(
            device,
            queue,
            &height_image.into(),
            Some("Height Map Texture"),
            false,
        )
        .expect("valid texture");
        let normal_texture = texture::Texture::from_image(
            device,
            queue,
            &normal_image.into(),
            Some("Normal Map Texture"),
            true,
        )
        .expect("valid texture");

        let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::MirrorRepeat,
            address_mode_v: wgpu::AddressMode::MirrorRepeat,
            address_mode_w: wgpu::AddressMode::MirrorRepeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let normal_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&height_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&normal_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&normal_sampler),
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
            diffuse_texture: height_texture,
            normal_texture,
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
