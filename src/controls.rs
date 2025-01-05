use glam::Vec3;
use iced_wgpu::core::Alignment;
use iced_wgpu::Renderer;
use iced_widget::{checkbox, column, container, row, slider, text};
use iced_winit::core::{Element, Length::*, Theme};
use iced_winit::runtime::{Program, Task};

pub struct Controls {
    pub camera: Vec3,
    pub zoom: f32,
    pub show_wireframe: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    CameraChanged(Vec3),
    ZoomChanged(f32),
    ShowWireFrame(bool),
}

impl Controls {
    pub fn new() -> Controls {
        Controls {
            camera: [0.0, 0.0, 0.].into(),
            zoom: 1.,
            show_wireframe: false,
        }
    }
}

impl Default for Controls {
    fn default() -> Self {
        Self::new()
    }
}

impl Program for Controls {
    type Theme = Theme;
    type Message = Message;
    type Renderer = Renderer;

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::CameraChanged(color) => {
                self.camera = color;
            }
            Message::ZoomChanged(zoom) => {
                self.zoom = zoom;
            }
            Message::ShowWireFrame(v) => {
                self.show_wireframe = v;
            }
        }

        Task::none()
    }

    fn view(&self) -> Element<Message, Theme, Renderer> {
        let camera = self.camera;

        let zoom_slider = row![
            text("Zoom"),
            slider(0.1..=10.0, self.zoom, Message::ZoomChanged).step(0.05)
        ]
        .spacing(5.);
        let camera_slider = row![
            text("Rotation"),
            column![
                row![
                    text(format!("x:{:.0}", camera.x * 360.))
                        .width(50.)
                        .align_x(Alignment::End),
                    slider(-1.0..=1.0, camera.x, move |x| {
                        Message::CameraChanged([x, camera.y, camera.z].into())
                    })
                    .width(100.)
                    .step(0.01)
                ],
                row![
                    text(format!("y:{:.0}", camera.y * 360.))
                        .width(50.)
                        .align_x(Alignment::End),
                    slider(-1.0..=1.0, camera.y, move |y| {
                        Message::CameraChanged([camera.x, y, camera.z].into())
                    })
                    .width(100.)
                    .step(0.01)
                ],
                row![
                    text(format!("z:{:.0}", camera.z * 360.))
                        .width(50.)
                        .align_x(Alignment::End),
                    slider(-1.0..=1.0, camera.z, move |z| {
                        Message::CameraChanged([camera.x, camera.y, z].into())
                    })
                    .width(100.)
                    .step(0.01),
                ]
            ]
        ]
        .width(Fill);

        container(
            column![
                checkbox("wireframe", self.show_wireframe).on_toggle(Message::ShowWireFrame),
                text("Camera"),
                camera_slider,
                zoom_slider,
            ]
            .width(550.)
            .spacing(10),
        )
        .padding(10)
        .align_bottom(Fill)
        .into()
    }
}
