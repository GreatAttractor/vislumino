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

use cgmath::{Matrix3, Rotation3, Vector2, SquareMatrix};
use crate::config::{Configuration, ProjectionConfig};
use crate::data;
use crate::data::ToArray;
use crate::gui;
use crate::gui::draw_buffer::Sampling;
use crate::gui::DrawBuffer;
use crate::gui::long_task_dialog::LongTaskDialog;
use crate::projection;
use crate::projection::{ExportDialog, handle_export_dialog, SourceView, source_view::SourceParameters, worker};
use crate::subscriber::Subscriber;
use glium::{Surface, uniform};
use glium::Texture2d;
use std::cell::RefCell;
use std::rc::Rc;

const PI_2: f32 = std::f32::consts::PI / 2.0;

#[derive(Copy, Clone, PartialEq)]
pub enum ProjectionType {
    Equirectangular,
    LambertCylindricalEqualArea
}

struct Grid {
    show: bool,
    horz_spacing: f32,
    vert_spacing: f32,
    horz_lines: glium::VertexBuffer<data::Vertex2>,
    vert_lines: glium::VertexBuffer<data::Vertex2>,
    color: [f32; 4]
}

pub struct ProjectionView {
    unique_id: u32,
    display: glium::Display,
    source_image: Rc<Texture2d>,
    source_image_idx: usize,
    src_params: SourceParameters,
    /// Used to generate projection of `source_image`; updated only if `source_image` or projection parameters change.
    projection_draw_buf: DrawBuffer,
    /// Used to create the displayed view contents; updated if `projection_draw_buf` changes and on resize.
    display_draw_buf: DrawBuffer,
    projection_prog: Rc<glium::Program>,
    texture_copy_prog: Rc<glium::Program>,
    solid_color_2d_prog: Rc<glium::Program>,
    unit_quad: Rc<glium::VertexBuffer<data::Vertex2>>,
    wh_ratio: f32,
    rotation_comp: Option<f32>, // `None` means "automatic" (based on rotation period, disk diameter and frame interval)
    grid: Grid,
    projection_type: ProjectionType
}

impl ProjectionView {
    pub fn new(
        unique_id: u32,
        gl_objects: &projection::data::OpenGlObjects,
        display: &glium::Display,
        renderer: &Rc<RefCell<imgui_glium_renderer::Renderer>>,
        source_image: &Rc<Texture2d>,
        source_image_idx: usize,
        src_params: SourceParameters,
        rotation_comp: f32
    ) -> ProjectionView {
        assert!(rotation_comp >= 0.0);

        let projection_draw_buf = DrawBuffer::new_with_size(
            Sampling::Single,
            &gl_objects.texture_copy_single,
            &gl_objects.texture_copy_multi,
            &gl_objects.unit_quad,
            display,
            renderer,
            (src_params.disk_diameter * PI_2 + (src_params.num_images - 1) as f32 * rotation_comp).ceil() as u32,
            (src_params.disk_diameter * PI_2).ceil() as u32,
        );

        let display_draw_buf = DrawBuffer::new(
            Sampling::Single,
            &gl_objects.texture_copy_single,
            &gl_objects.texture_copy_multi,
            &gl_objects.unit_quad,
            display,
            renderer
        );

        let wh_ratio = projection_draw_buf.width() as f32 / projection_draw_buf.height() as f32;

        let mut projection_view = ProjectionView{
            unique_id,
            display: display.clone(),
            projection_prog: Rc::clone(&gl_objects.projection),
            texture_copy_prog: Rc::clone(&gl_objects.texture_copy_single),
            solid_color_2d_prog: Rc::clone(&gl_objects.solid_color_2d),
            projection_draw_buf,
            display_draw_buf,
            unit_quad: Rc::clone(&gl_objects.unit_quad),
            source_image: Rc::clone(source_image),
            source_image_idx,
            src_params,
            wh_ratio,
            rotation_comp: Some(0.0),
            grid: create_grid(display, false, wh_ratio, 0.25, 0.25, 0.75),
            projection_type: ProjectionType::Equirectangular
        };

        projection_view.on_image_or_projection_changed();

        projection_view
    }

    /// Size in pixels of the generated projected view.
    fn projection_size(&self) -> [u32; 2] {
        [
            self.projection_draw_buf.width(),
            self.projection_draw_buf.height()
        ]
    }

    fn rotation_comp_value(&self) -> f32 {
        match self.rotation_comp {
            None => {
                let sp = &self.src_params;
                PI_2 * sp.disk_diameter / (0.5 * sp.sidereal_rotation_period.as_secs_f32() / sp.frame_interval.as_secs_f32())
            },

            Some(value) => value
        }
    }

    fn on_image_or_projection_changed(&mut self) {
        render_projection(
            true,
            self.source_image_idx,
            &self.source_image,
            &mut self.projection_draw_buf.frame_buf(),
            &self.unit_quad,
            &self.projection_prog,
            &self.src_params,
            self.rotation_comp_value(),
            self.projection_type
        );

        self.projection_draw_buf.update_storage_buf();

        self.render();
    }

    pub fn set_source_image(&mut self, source_image: &Rc<Texture2d>) {
        self.source_image = Rc::clone(&source_image);
        self.on_image_or_projection_changed();
    }

    pub fn update_size(&mut self, width: u32, height: u32) {
        if height == 0 { return; }

        if self.display_draw_buf.update_size(width, height) {
            self.render()
        }
    }

    fn render(&self) {
        let mut target = self.display_draw_buf.frame_buf();

        let uniforms = uniform! {
            source_texture: self.projection_draw_buf.storage_buf().sampled(),
        };

        target.draw(
            &*self.unit_quad,
            &glium::index::NoIndices(glium::index::PrimitiveType::TriangleFan),
            &self.texture_copy_prog,
            &uniforms,
            &Default::default()
        ).unwrap();

        if self.grid.show {
            let uniforms = uniform! {
                color: self.grid.color,
                vertex_transform: Matrix3::<f32>::identity().to_array()
            };

            target.draw(
                &self.grid.vert_lines,
                &glium::index::NoIndices(glium::index::PrimitiveType::LinesList),
                &self.solid_color_2d_prog,
                &uniforms,
                &glium::DrawParameters{
                    blend: glium::Blend::alpha_blending(),
                    ..Default::default()
                }
            ).unwrap();

            target.draw(
                &self.grid.horz_lines,
                &glium::index::NoIndices(glium::index::PrimitiveType::LinesList),
                &self.solid_color_2d_prog,
                &uniforms,
                &glium::DrawParameters{
                    blend: glium::Blend::alpha_blending(),
                    ..Default::default()
                }
            ).unwrap();
        }

        self.display_draw_buf.update_storage_buf();
    }

    fn display_buf_id(&self) -> imgui::TextureId { self.display_draw_buf.id() }

    pub fn id(&self) -> u32 { self.unique_id }

    pub fn set_projection_type(&mut self, value: ProjectionType) {
        self.projection_type = value;
        self.update_projection_buf_size();
        self.grid.vert_lines = create_grid_lines(&self.display, self.grid.vert_spacing / self.wh_ratio, false);
        self.on_image_or_projection_changed();
    }

    pub fn set_rotation_comp(&mut self, value: Option<f32>) {
        self.rotation_comp = value;

        self.update_projection_buf_size();

        self.grid.vert_lines = create_grid_lines(&self.display, self.grid.vert_spacing / self.wh_ratio, false);

        self.on_image_or_projection_changed();
    }

    fn update_projection_buf_size(&mut self) {
        let new_width = (self.src_params.disk_diameter * PI_2 +
            (self.src_params.num_images - 1) as f32 * self.rotation_comp_value()).ceil() as u32;

        let new_height = match self.projection_type {
            ProjectionType::Equirectangular => (self.src_params.disk_diameter * PI_2).ceil() as u32,

            ProjectionType::LambertCylindricalEqualArea => self.src_params.disk_diameter as u32
        };

        self.projection_draw_buf.update_size(new_width, new_height);

        self.wh_ratio = new_width as f32 / new_height as f32;
    }

    pub fn set_grid_horz_spacing(&mut self, spacing: f32) {
        self.grid.horz_spacing = spacing;
        self.grid.vert_lines = create_grid_lines(&self.display, spacing / self.wh_ratio, false);
        self.render();
    }

    pub fn set_grid_vert_spacing(&mut self, spacing: f32) {
        self.grid.vert_spacing = spacing;
        self.grid.horz_lines = create_grid_lines(&self.display, spacing, true);
        self.render();
    }
}

impl Subscriber<(usize, Rc<Texture2d>)> for ProjectionView {
    fn notify(&mut self, value: &(usize, Rc<Texture2d>)) {
        self.source_image_idx = value.0;
        self.set_source_image(&value.1);
    }
}

impl Subscriber<SourceParameters> for ProjectionView {
    fn notify(&mut self, value: &SourceParameters) {
        let dd_changed = value.disk_diameter != self.src_params.disk_diameter;
        let num_images_changed = value.num_images != self.src_params.num_images;
        self.src_params = value.clone();
        if dd_changed || num_images_changed {
            self.update_projection_buf_size();
        }
        self.on_image_or_projection_changed();
    }
}

fn create_grid_lines(display: &glium::Display, spacing: f32, horizontal: bool) -> glium::VertexBuffer<data::Vertex2> {
    assert!(spacing > 0.0 && spacing < 2.0);

    let mut vertices = vec![];

    let mut pos = -1.0 + spacing;
    while pos < 1.0 {
        vertices.push(data::Vertex2{ position: if horizontal { [-1.0, pos ] } else { [pos, -1.0] } });
        vertices.push(data::Vertex2{ position: if horizontal { [1.0, pos] } else { [pos, 1.0] } });
        pos += spacing;
    }

    glium::VertexBuffer::dynamic(display, &vertices).unwrap()
}

fn create_grid(
    display: &glium::Display,
    show: bool,
    wh_ratio: f32,
    horz_spacing: f32,
    vert_spacing: f32,
    opacity: f32
) -> Grid {
    Grid{
        show,
        horz_spacing,
        vert_spacing,
        horz_lines: create_grid_lines(display, horz_spacing, true),
        vert_lines: create_grid_lines(display, vert_spacing * wh_ratio, false),
        color: [1.0, 0.0, 0.0, opacity]
    }
}

pub fn render_projection(
    vertical_flip: bool,
    source_image_idx: usize,
    source_image: &glium::Texture2d,
    target: &mut impl glium::Surface,
    unit_quad: &glium::VertexBuffer<data::Vertex2>,
    projection_prog: &glium::Program,
    src_params: &SourceParameters,
    rotation_comp: f32,
    projection_type: ProjectionType
) {
    let flattening_transform = Matrix3::<f32>::from_nonuniform_scale(1.0, 1.0 - src_params.flattening);
    let inclination_transform = cgmath::Basis3::from_angle_x(src_params.inclination);
    let roll_transform = cgmath::Basis3::from_angle_z(src_params.roll);
    let globe_transform = Matrix3::from(roll_transform) * Matrix3::from(inclination_transform) * flattening_transform;

    let img_width = PI_2 * src_params.disk_diameter;
    let total_width = img_width + (src_params.num_images - 1) as f32 * rotation_comp;
    let rel_img_w = img_width / total_width;
    let rel_comp = rotation_comp / total_width;

    let image_transform: Matrix3<f32> =
        Matrix3::from_translation(Vector2{
            x: 1.0 - rel_img_w - 2.0 * rel_comp * source_image_idx as f32,
            y: 0.0
        }) *
        Matrix3::from_nonuniform_scale(rel_img_w, if vertical_flip { -1.0 } else { 1.0 });

    let uniforms = uniform! {
        source_image: source_image.sampled(),
        disk_diameter: src_params.disk_diameter,
        disk_center: src_params.disk_center.to_array(),
        globe_transform: globe_transform.to_array(),
        vertex_transform: image_transform.to_array(),
        equirectangular: match projection_type {
            ProjectionType::Equirectangular => true,
            ProjectionType::LambertCylindricalEqualArea => false,
        }
    };

    target.clear_color(0.0, 0.0, 0.0, 1.0);

    target.draw(
        unit_quad,
        &glium::index::NoIndices(glium::index::PrimitiveType::TriangleFan),
        projection_prog,
        &uniforms,
        &Default::default()
    ).unwrap();
}

/// Returns `false` if view should be closed.
pub fn handle_projection_view(
    ui: &imgui::Ui,
    gui_state: &mut gui::GuiState,
    config: &mut Configuration,
    view: &mut ProjectionView,
    source_view: &SourceView,
    long_task_dialog: &RefCell<Option<LongTaskDialog>>,
    task_sender: &crossbeam::channel::Sender<worker::MainToWorkerMsg>,
    export_dialog: &RefCell<ExportDialog>
) -> bool {
    let mut opened = true;

    let mut export_clicked = false;

    imgui::Window::new(ui, &format!("Projection###projection-view-{}", view.id()))
        .size([640.0, 640.0], imgui::Condition::FirstUseEver)
        .opened(&mut opened)
        .horizontal_scrollbar(true)
        .build(|| {
            if ui.button("Export...") { export_clicked = true; }

            ui.separator();

            if ui.radio_button_bool("equirectangular", view.projection_type == ProjectionType::Equirectangular) {
                view.set_projection_type(ProjectionType::Equirectangular);
            }

            ui.same_line();
            if ui.radio_button_bool("Lambert equal-area", view.projection_type == ProjectionType::LambertCylindricalEqualArea) {
                view.set_projection_type(ProjectionType::LambertCylindricalEqualArea);
            }

            gui::add_text_before(ui, "rotation comp.");
            gui::tooltip(ui, "Planet rotation compensation.");

            let mut rot_comp_auto = view.rotation_comp.is_none();
            if ui.checkbox("auto##rotation-comp-auto", &mut rot_comp_auto) {
                view.set_rotation_comp(if rot_comp_auto { None } else { Some(view.rotation_comp_value()) });
            }
            ui.same_line();

            let token = ui.begin_disabled(rot_comp_auto);
            let mut value = view.rotation_comp_value();
            if imgui::Slider::new("##rotation-comp", 0.0, 10.0)
                .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                .display_format("%0.3f px/frame")
                .build(ui, &mut value)
            {
                view.set_rotation_comp(Some(value));
            }
            token.end();

            ui.tree_node_config("grid").build(|| {
                if ui.checkbox("show", &mut view.grid.show) {
                    view.render();
                }

                let token = ui.begin_disabled(!view.grid.show);

                ui.same_line();
                if imgui::ColorEdit4::new("color##grid-color", &mut view.grid.color)
                    .alpha(false)
                    .inputs(false)
                    .build(ui)
                {
                    view.render();
                }

                gui::add_text_before(ui, "opacity");
                let mut value = view.grid.color[3] * 100.0;
                if imgui::Slider::new("##grid-opacity", 5.0, 100.0)
                    .display_format("%0.1f%%")
                    .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                    .build(ui, &mut value)
                {
                    if value >= 5.0 && value <= 100.0 {
                        view.grid.color[3] = value / 100.0;
                        view.render();
                    }
                }

                gui::add_text_before(ui, "horz. spacing");
                let mut value = view.grid.horz_spacing;
                if imgui::Slider::new("##grid-horz-spacing", 0.05, 0.5)
                    .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                    .display_format("%0.4f")
                    .build(ui, &mut value)
                {
                    view.set_grid_horz_spacing(value);
                }

                gui::add_text_before(ui, "vert. spacing");
                let mut value = view.grid.vert_spacing;
                if imgui::Slider::new("##grid-vert-spacing", 0.05, 0.5)
                    .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                    .display_format("%0.4f")
                    .build(ui, &mut value)
                {
                    view.set_grid_vert_spacing(value);
                }

                token.end();
            });

            if view.projection_size()[1] != 0 {
                let adjusted_logical_sz = gui::fill_vertically(view.projection_size(), ui.content_region_avail());

                let hidpi_f = gui_state.hidpi_factor() as f32;
                let adjusted = gui::adjust_pos_size_for_exact_hidpi_scaling(ui, hidpi_f, adjusted_logical_sz);

                view.update_size(
                    adjusted.physical_size[0],
                    adjusted.physical_size[1]
                );

                imgui::Image::new(view.display_buf_id(), adjusted.logical_size).build(ui);
            }
        }
    );

    if export_clicked {
        ui.open_popup(&export_dialog.borrow().title());
    }

    handle_export(
        ui, gui_state, config, view, source_view, long_task_dialog, task_sender, &mut export_dialog.borrow_mut()
    );

    opened
}

fn handle_export(
    ui: &imgui::Ui,
    gui_state: &mut gui::GuiState,
    config: &mut Configuration,
    view: &ProjectionView,
    source_view: &SourceView,
    long_task_dialog: &RefCell<Option<LongTaskDialog>>,
    task_sender: &crossbeam::channel::Sender<worker::MainToWorkerMsg>,
    export_dialog: &mut ExportDialog
) {
    if handle_export_dialog(ui, gui_state, export_dialog) {
        let (progress_sender, progress_receiver) = crossbeam::channel::bounded(1);

        let sz = source_view.image_size();

        task_sender.send(worker::MainToWorkerMsg::Projection(worker::Projection{
            output_dir: export_dialog.output_path(),
            sender: progress_sender,
            source_texture_ids: source_view.texture_ids(),
            bounce_back: export_dialog.bounce_back(),
            image_size: glium::texture::Dimensions::Texture2d{ width: sz[0], height: sz[1] },
            src_params: view.src_params.clone(),
            rotation_comp: view.rotation_comp_value(),
            projection_type: view.projection_type
        })).unwrap();

        *long_task_dialog.borrow_mut() =
            Some(LongTaskDialog::new("Exporting".to_string(), "".to_string(), progress_receiver));

        config.set_projection_export_path(export_dialog.output_path().to_str().unwrap()); //TODO: handle non-UTF-8 paths
    }
}
