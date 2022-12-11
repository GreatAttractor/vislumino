//
// Vislumino - Astronomy Visualization Tools
// Copyright (c) 2022 Filip Szczerek <ga.software@yahoo.com>
//
// This file is part of Vislumino.
//
// Vislumino is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.
//
// Vislumino is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Vislumino.  If not, see <http://www.gnu.org/licenses/>.
//

use cgmath::{Basis3, Deg, InnerSpace, Matrix3, One, Rad, Rotation3, Vector3};
use glium::{texture::Texture2d, uniform};
use crate::data::ToArray;
use crate::gui;
use crate::gui::draw_buffer::Sampling;
use crate::gui::DrawBuffer;
use crate::gui::long_task_dialog::LongTaskDialog;
use crate::projection;
use crate::projection::{
    data::LonLatGlBuffers,
    source_view::{SourceParameters},
    worker,
};
use crate::subscriber::Subscriber;
use std::cell::RefCell;
use std::rc::Rc;

const MOUSE_WHEEL_ZOOM_FACTOR: f64 = 1.1;
const PI_2: f32 = std::f32::consts::PI / 2.0;

#[derive(Copy, Clone, PartialEq)]
pub enum DragRotation {
    NSEW,
    Free
}

pub struct GlobeView {
    unique_id: u32,
    source_image: Rc<Texture2d>,
    source_image_idx: usize,
    src_params: SourceParameters,
    draw_buf: DrawBuffer,
    gl_prog: Rc<glium::Program>,
    globe_mesh: LonLatGlBuffers,
    wh_ratio: f32,
    orientation: Basis3<f64>,
    angle_ns: Rad<f64>,
    angle_ew: Rad<f64>,
    zoom: f64,
    drag_rotation: DragRotation,
}

impl GlobeView {
    pub fn new(
        unique_id: u32,
        gl_objects: &projection::data::OpenGlObjects,
        display: &glium::Display,
        renderer: &Rc<RefCell<imgui_glium_renderer::Renderer>>,
        source_image: &Rc<Texture2d>,
        source_image_idx: usize,
        src_params: SourceParameters
    ) -> GlobeView {
        let draw_buf = DrawBuffer::new(
            Sampling::Single,
            &gl_objects.texture_copy_single,
            &gl_objects.texture_copy_multi,
            &gl_objects.unit_quad,
            display,
            renderer
        );

        let globe_view = GlobeView{
            unique_id,
            source_image: Rc::clone(source_image),
            source_image_idx,
            src_params,
            gl_prog: Rc::clone(&gl_objects.globe_texturing),
            globe_mesh: gl_objects.globe_mesh.clone(),
            draw_buf,
            wh_ratio: 1.0,
            zoom: 0.75,
            orientation: Basis3::one(),
            drag_rotation: DragRotation::NSEW,
            angle_ew: Rad(0.0),
            angle_ns: Rad(0.0)
        };

        globe_view.render();

        globe_view
    }

    fn render(&self) {
        let mut target = self.draw_buf.frame_buf();
        render_globe(
            true,
            self.source_image_idx,
            &self.source_image,
            &mut target,
            &self.gl_prog,
            &self.src_params,
            self.orientation,
            &self.globe_mesh,
            self.zoom,
            self.wh_ratio
        );
        self.draw_buf.update_storage_buf();
    }

    pub fn update_size(&mut self, width: u32, height: u32) {
        if height == 0 { return; }

        if self.draw_buf.update_size(width, height) {
            self.wh_ratio = width as f32 / height as f32;
            self.render()
        }
    }

    pub fn id(&self) -> u32 { self.unique_id }

    fn display_buf_id(&self) -> imgui::TextureId { self.draw_buf.id() }

    pub fn zoom_by(&mut self, relative_zoom: f64) {
        self.zoom *= relative_zoom;
        if self.zoom < 0.5 { self.zoom = 0.5; }
        self.render();
    }

    /// Elements of `start` and `end` denote normalized mouse position within the view,
    /// with values from [-1, 1] (i.e., bottom-left is [-1, -1], and top-right is [1, 1]).
    pub fn rotate_by_dragging(&mut self, start: [f32; 2], end: [f32; 2]) {
        match self.drag_rotation {
            // simulates "space ball" rotation
            DragRotation::Free => {
                let start_vec = Vector3{ x: 1.0, y: start[0] as f64, z: start[1] as f64 };
                let end_vec = Vector3{ x: 1.0, y: end[0] as f64, z: end[1] as f64 };

                let axis_of_rotation = start_vec.cross(end_vec).normalize();
                let angle = Rad(
                    1.0 / self.zoom * ((start[0] - end[0]).powi(2) + (start[1] - end[1]).powi(2)).sqrt() as f64
                );

                let rotation = Basis3::from_axis_angle(axis_of_rotation, angle);

                self.orientation = rotation * self.orientation;
            },

            DragRotation::NSEW => {
                let new_angle_ns = self.angle_ns + Rad(1.0 / self.zoom * (start[1] - end[1]) as f64);
                if new_angle_ns.0.abs() <= Rad::from(Deg(90.0)).0 {
                    self.angle_ns = new_angle_ns;
                }

                self.angle_ew += Rad(1.0 / self.zoom * (end[0] - start[0]) as f64);

                let rotation_ns = Basis3::from_angle_y(self.angle_ns);
                let rotation_ew = Basis3::from_angle_z(self.angle_ew);

                self.orientation = rotation_ns * rotation_ew;
            }
        }

        self.render();
    }

    pub fn set_source_image(&mut self, source_image: &Rc<Texture2d>) {
        self.source_image = Rc::clone(&source_image);
        self.render();
    }
}

impl Subscriber<(usize, Rc<Texture2d>)> for GlobeView {
    fn notify(&mut self, value: &(usize, Rc<Texture2d>)) {
        self.source_image_idx = value.0;
        self.set_source_image(&value.1);
    }
}

impl Subscriber<SourceParameters> for GlobeView {
    fn notify(&mut self, value: &SourceParameters) {
        self.src_params = value.clone();
        self.render();
    }
}

pub fn render_globe(
    vertical_flip: bool,
    _source_image_idx: usize,
    source_image: &glium::Texture2d,
    target: &mut impl glium::Surface,
    gl_prog: &glium::Program,
    src_params: &SourceParameters,
    globe_orientation: Basis3<f64>,
    globe_mesh: &LonLatGlBuffers,
    zoom : f64,
    wh_ratio: f32
) {
    let flattening_transform = Matrix3::<f32>::from_nonuniform_scale(1.0, 1.0 - src_params.flattening);
    let inclination_transform = cgmath::Basis3::from_angle_x(src_params.inclination);
    let roll_transform = cgmath::Basis3::from_angle_z(-src_params.roll);
    let globe_transform = Matrix3::from(roll_transform) * Matrix3::from(inclination_transform) * flattening_transform;

    let uniforms = uniform! {
        source_image: source_image.sampled(),
        disk_diameter: src_params.disk_diameter,
        disk_center: src_params.disk_center.to_array(),
        globe_orientation: Matrix3::from(globe_orientation).cast::<f32>().unwrap().to_array(),
        globe_transform: globe_transform.to_array(),
        flattening: src_params.flattening,
        zoom: zoom as f32,
        wh_ratio: wh_ratio,
        texture_vertical_flip: vertical_flip
    };

    target.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);
    target.draw(
        &*globe_mesh.vertices,
        &*globe_mesh.indices,
        gl_prog,
        &uniforms,
        &glium::DrawParameters{
            depth: glium::Depth{
                test: glium::DepthTest::IfLess,
                write: true,
                ..Default::default()
            },
            ..Default::default()
        }
    ).unwrap();
}

/// Returns `false` if view should be closed.
pub fn handle_globe_view(
    ui: &imgui::Ui,
    gui_state: &mut gui::GuiState,
    view: &mut GlobeView,
    _long_task_dialog: &RefCell<Option<LongTaskDialog>>,
    _task_sender: &crossbeam::channel::Sender<worker::MainToWorkerMsg>
) -> bool {
    let mut opened = true;

    imgui::Window::new(ui, &format!("Globe###globe-view-{}", view.id()))
        .size([640.0, 640.0], imgui::Condition::FirstUseEver)
        .opened(&mut opened)
        .build(|| {
            let hidpi_f = gui_state.hidpi_factor() as f32;
            let adjusted = gui::adjust_pos_for_exact_hidpi_scaling(ui, 0.0, hidpi_f);

            view.update_size(
                adjusted.physical_size[0],
                adjusted.physical_size[1]
            );

            let img_pos_in_app_window = ui.cursor_screen_pos();
            let _image_start_pos = ui.cursor_pos();
            imgui::Image::new(view.display_buf_id(), adjusted.logical_size).build(ui);

            let mouse_pos_in_app_window = ui.io().mouse_pos;
            if ui.is_item_clicked_with_button(imgui::MouseButton::Left) {
                gui_state.mouse_drag_origin = [
                    mouse_pos_in_app_window[0] - img_pos_in_app_window[0],
                    mouse_pos_in_app_window[1] - img_pos_in_app_window[1]
                ];
            }
            if ui.is_item_hovered() {
                let wheel = ui.io().mouse_wheel;
                if wheel != 0.0 {
                    let zoom_factor = MOUSE_WHEEL_ZOOM_FACTOR.powf(wheel as f64);
                    view.zoom_by(zoom_factor);
                }

                if ui.is_mouse_dragging(imgui::MouseButton::Left) {
                    let delta = ui.mouse_drag_delta_with_button(imgui::MouseButton::Left);
                    if delta[0] != 0.0 || delta[1] != 0.0 {
                        let drag_start: [f32; 2] = [
                            -1.0 + 2.0 * (gui_state.mouse_drag_origin[0] / adjusted.logical_size[0]),
                            -(-1.0 + 2.0 * (gui_state.mouse_drag_origin[1] / adjusted.logical_size[1]))
                        ];

                        let drag_end = [
                            drag_start[0] + 2.0 * delta[0] / adjusted.logical_size[0],
                            drag_start[1] - 2.0 * delta[1] / adjusted.logical_size[1]
                        ];

                        view.rotate_by_dragging(drag_start, drag_end);
                    }
                    ui.reset_mouse_drag_delta(imgui::MouseButton::Left);
                    gui_state.mouse_drag_origin = [
                        mouse_pos_in_app_window[0] - img_pos_in_app_window[0],
                        mouse_pos_in_app_window[1] - img_pos_in_app_window[1]
                    ];
                }
            }
        }
    );
    opened
}
