use glam::Vec3;
use iced_wgpu::wgpu::{self, Device, SurfaceConfiguration};
use iced_winit::winit::dpi::PhysicalSize;
use obj_scene::ObjScene;
use terrain_scene::TerrainScene;

use crate::controls::Controls;

pub mod obj_scene;
pub mod terrain_scene;

#[derive(Clone, Copy)]
pub enum UnitScene {
    ObjScene,
    TerrainScene,
}

pub enum SceneData {
    ObjScene(ObjScene),
    TerrainScene(TerrainScene),
}

pub struct Scene {
    scene_data: SceneData,
}

impl Scene {
    pub fn new(
        scene_type: UnitScene,
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        queue: &wgpu::Queue,
        sample_count: u32,
    ) -> Self {
        Self {
            scene_data: match scene_type {
                UnitScene::ObjScene => {
                    SceneData::ObjScene(ObjScene::init(device, config, queue, sample_count))
                }
                UnitScene::TerrainScene => {
                    SceneData::TerrainScene(TerrainScene::init(device, config, queue, sample_count))
                }
            },
        }
    }

    pub fn resize(
        &mut self,
        new_size: PhysicalSize<u32>,
        device: &Device,
        config: &SurfaceConfiguration,
    ) {
        match &mut self.scene_data {
            SceneData::ObjScene(obj_scene) => obj_scene.resize(new_size, device, config),
            SceneData::TerrainScene(terrain_scene) => {
                terrain_scene.resize(new_size, device, config)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        controls: &Controls,
        view: &wgpu::TextureView,
        camera: Vec3,
        zoom: f32,
        show_wireframe: bool,
        aspect: f32,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        match &mut self.scene_data {
            SceneData::ObjScene(obj_scene) => obj_scene.render(
                controls,
                view,
                camera, //TODO: make a "render config" type that bundles these params
                // together
                zoom,           // |
                show_wireframe, // |
                aspect,
                device,
                queue,
            ),
            SceneData::TerrainScene(terrain_scene) => terrain_scene.render(
                controls,
                view,
                camera,
                zoom,
                show_wireframe,
                aspect,
                device,
                queue,
            ),
        }
    }
}
