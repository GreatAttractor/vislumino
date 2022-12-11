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

use cgmath::{Basis3, Deg, EuclideanSpace, Matrix3, Matrix4, Point2, Point3, Rotation3, Vector3, SquareMatrix};
use glium::GlObject;
use crate::data;
use crate::data::{TextureId, ToArray};
use crate::gui;
use crate::gui::{draw_buffer::{DrawBuffer, Sampling}, GuiState};
use crate::projection;
use crate::projection::{data::create_half_parallel, Planet};
use crate::subscriber::{Subscriber, SubscriberCollection};
use glium::{Surface, texture::Texture2d, uniform};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::time::Duration;

struct Playback {
    enabled: bool,
    tstart: Option<std::time::Instant>,
    first_frame: Option<usize>,
    initial_bouncing_back: Option<bool>,
    current_bouncing_back: Option<bool>
}

#[derive(Clone)]
pub struct SourceParameters {
    pub num_images: usize,
    pub inclination: Deg<f32>,
    pub frame_interval: Duration,
    pub roll: Deg<f32>,
    pub disk_center: Point2<f32>,
    pub disk_diameter: f32,
    /// Value: 1.0 - polar_radius / equatorial_radius.
    pub flattening: f32,
    pub sidereal_rotation_period: Duration,
}

/// Shows source images and planet outline.
pub struct SourceView {
    playback: Playback,
    fps: u32,
    draw_buffer: DrawBuffer,
    wh_ratio: f32,
    images: Vec<Rc<Texture2d>>,
    texture_copy_prog: Rc<glium::Program>,
    solid_color_3d_prog: Rc<glium::Program>,
    unit_quad: Rc<glium::VertexBuffer<data::Vertex2>>,
    unit_circle: Rc<glium::VertexBuffer<data::Vertex3>>,
    half_parallels: Vec<glium::VertexBuffer<data::Vertex3>>,
    current_img_idx: usize,
    image_size: [u32; 2],
    planet: Option<Planet>, // `None` means "custom",
    src_params: SourceParameters,
    current_image_subscribers: SubscriberCollection<(usize, Rc<Texture2d>)>,
    src_params_subscribers: SubscriberCollection<SourceParameters>
}

impl SourceView {
    pub fn new(
        gl_objects: &projection::data::OpenGlObjects,
        display: &glium::Display,
        renderer: &Rc<RefCell<imgui_glium_renderer::Renderer>>,
        src_images: Vec<Rc<Texture2d>>, // all images must have the same dimensions
        disk_center: Point2<f32>,
        disk_diameter: f32
    ) -> SourceView {
        let draw_buffer = DrawBuffer::new(
            Sampling::Single,
            &gl_objects.texture_copy_single,
            &gl_objects.texture_copy_multi,
            &gl_objects.unit_quad,
            display,
            renderer
        );

        let image_size = check_sizes_match(&src_images);
        if image_size[0] == 0 || image_size[1] == 0 { panic!("image width nor height cannot be zero"); }

        let num_images = src_images.len();

        SourceView{
            playback: Playback {
                enabled: false,
                first_frame: None,
                tstart: None,
                initial_bouncing_back: Some(false),
                current_bouncing_back: Some(false)
            },
            fps: 25,
            draw_buffer,
            wh_ratio: image_size[0] as f32 / image_size[1] as f32,
            images: src_images,
            texture_copy_prog: Rc::clone(&gl_objects.texture_copy_single),
            solid_color_3d_prog: Rc::clone(&gl_objects.solid_color_3d),
            unit_quad: Rc::clone(&gl_objects.unit_quad),
            unit_circle: Rc::clone(&gl_objects.unit_circle),
            half_parallels: vec![
                create_half_parallel(Deg(-45.0), 128, display),
                create_half_parallel(Deg(0.0), 128, display),
                create_half_parallel(Deg(45.0), 128, display),
            ],
            current_img_idx: 0,
            image_size,
            src_params: SourceParameters{
                num_images,
                inclination: Deg(0.0),
                frame_interval: Duration::from_secs(60),
                roll: Deg(0.0),
                disk_center,
                disk_diameter,
                flattening: Planet::Jupiter.flattening(),
                sidereal_rotation_period: Planet::Jupiter.sidereal_rotation()
            },
            planet: Some(Planet::Jupiter),
            current_image_subscribers: Default::default(),
            src_params_subscribers: Default::default()
        }
    }

    pub fn texture_ids(&self) -> Vec<TextureId> {
        self.images.iter().map(|img| img.get_id()).collect()
    }

    pub fn set_images(
        &mut self,
        src_images: Vec<Rc<Texture2d>>, // all images must have the same dimensions
        disk_center: Point2<f32>,
        disk_diameter: f32
    ) {
        self.image_size = check_sizes_match(&src_images);
        self.images = src_images;

        self.src_params.num_images = self.images.len();
        self.src_params.disk_center = disk_center;
        self.src_params.disk_diameter = disk_diameter;

        self.current_img_idx = 0;
        let current_image = Rc::clone(&self.current_image());
        self.current_image_subscribers.notify(&(self.current_img_idx, current_image));
        self.src_params_subscribers.notify(&self.src_params);

        self.render();
        self.on_reset_playback();
    }

    pub fn num_images(&self) -> usize { self.images.len() }

    pub/*temp*/ fn current_image(&self) -> &Rc<Texture2d> { &self.images[self.current_img_idx] }

    pub fn image_size(&self) -> [u32; 2] { self.image_size }

    pub fn current_image_idx(&self) -> usize { self.current_img_idx }

    fn set_image_idx(&mut self, idx: usize) {
        if idx >= self.images.len() { return; }

        self.current_img_idx = idx;
        self.render();
        let current_image = Rc::clone(&self.current_image());
        self.current_image_subscribers.notify(&(self.current_img_idx, current_image));
    }

    pub fn update_size(&mut self, width: u32, height: u32) {
        if height == 0 { return; }

        if self.draw_buffer.update_size(width, height) {
            self.wh_ratio = width as f32 / height as f32;
            self.render();
        }
    }

    pub fn display_buf_id(&self) -> imgui::TextureId { self.draw_buffer.id() }

    fn disk_transform(&self, with_inclination: bool) -> Matrix4<f32> {
        let dc_f32 = self.src_params.disk_center.cast::<f32>().unwrap();
        let normalized_disk_center = Point3{
            x: dc_f32.x / self.image_size[0] as f32,
            y: -dc_f32.y / self.image_size[1] as f32,
            z: 0.0
        };

        let xy_scale = self.src_params.disk_diameter / self.images[0].width() as f32;

        Matrix4::<f32>::from_translation(Vector3{ x: -1.0, y: 1.0, z: 0.0 } + normalized_disk_center.to_vec() * 2.0) *
        Matrix4::<f32>::from_nonuniform_scale(xy_scale, xy_scale, 1.0) *
        Matrix4::<f32>::from_nonuniform_scale(1.0, self.wh_ratio, 1.0) *
        Matrix4::from(Matrix3::from(Basis3::<f32>::from_angle_z(-self.src_params.roll))) *
        if with_inclination {
            Matrix4::from(Matrix3::from(Basis3::<f32>::from_angle_x(-self.src_params.inclination)))
        } else {
            Matrix4::identity()
        } *
        Matrix4::<f32>::from_nonuniform_scale(1.0, 1.0/(1.0 + self.src_params.flattening), 1.0)
    }

    fn render(&self) {
        let mut target = self.draw_buffer.frame_buf();

        let uniforms = uniform! {
            source_texture: self.current_image().sampled()
        };

        target.draw(
            &*self.unit_quad,
            &glium::index::NoIndices(glium::index::PrimitiveType::TriangleFan),
            &self.texture_copy_prog,
            &uniforms,
            &Default::default()
        ).unwrap();

        let uniforms = uniform! {
            vertex_transform: self.disk_transform(false).to_array(),
            color: [1.0f32, 0.0f32, 0.0f32, 1.0f32]
        };

        target.draw(
            &*self.unit_circle,
            &glium::index::NoIndices(glium::index::PrimitiveType::LineLoop),
            &self.solid_color_3d_prog,
            &uniforms,
            &Default::default()
        ).unwrap();

        let uniforms = uniform! {
            vertex_transform: self.disk_transform(true).to_array(),
            color: [1.0f32, 0.0f32, 0.0f32, 1.0f32]
        };

        for half_parallel in &self.half_parallels {
            target.draw(
                half_parallel,
                &glium::index::NoIndices(glium::index::PrimitiveType::LineStrip),
                &self.solid_color_3d_prog,
                &uniforms,
                &Default::default()
            ).unwrap();
        }

        self.draw_buffer.update_storage_buf();
    }

    pub fn inclination(&self) -> Deg<f32> { self.src_params.inclination }

    pub fn set_inclination(&mut self, value: Deg<f32>) {
        self.src_params.inclination = value;
        self.src_params_subscribers.notify(&self.src_params);
        self.render();
    }

    pub fn roll(&self) -> Deg<f32> { self.src_params.roll }

    pub fn flattening(&self) -> f32 { self.src_params.flattening }

    pub fn set_flattening(&mut self, value: f32) {
        if self.planet.is_some() { panic!("cannot set flattening if a known planet is selected"); }
        self.src_params.flattening = value;
        self.src_params_subscribers.notify(&self.src_params);
        self.render();
    }

    pub fn set_roll(&mut self, value: Deg<f32>) {
        self.src_params.roll = value;
        self.src_params_subscribers.notify(&self.src_params);
        self.render();
    }

    pub fn subscribe_current_img(&mut self, subscriber: Weak<RefCell<dyn Subscriber<(usize, Rc<Texture2d>)>>>) {
        self.current_image_subscribers.add(subscriber);
    }

    pub fn subscribe_src_params(&mut self, subscriber: Weak<RefCell<dyn Subscriber<SourceParameters>>>) {
        self.src_params_subscribers.add(subscriber);
    }

    fn playing(&self) -> bool { self.playback.enabled }

    fn play(&mut self) {
        if self.playback.enabled {
            let t_from_start = self.playback.tstart.as_ref().unwrap().elapsed();
            let prev_frame = self.current_img_idx;
            self.current_img_idx = advance_current_frame(
                *self.playback.first_frame.as_ref().unwrap(),
                (t_from_start.as_secs_f32() * self.fps as f32) as usize,
                self.images.len(),
                &self.playback.initial_bouncing_back,
                &mut self.playback.current_bouncing_back
            );
            if self.current_img_idx != prev_frame {
                self.render();
                self.current_image_subscribers.notify(&(self.current_img_idx, Rc::clone(&self.current_image())));
            }
        }
    }

    fn fps(&self) -> u32 { self.fps }

    fn set_fps(&mut self, fps: u32) {
        self.fps = fps;
        self.on_reset_playback();
    }

    fn toggle_playing(&mut self) {
        self.playback.enabled = !self.playback.enabled;
        if self.playback.enabled {
            self.on_reset_playback();
        } else {
            self.playback.first_frame = None;
            self.playback.tstart = None;
        }
    }

    fn on_reset_playback(&mut self) {
        self.playback.tstart = Some(std::time::Instant::now());
        self.playback.first_frame = Some(self.current_img_idx);
        self.playback.initial_bouncing_back = self.playback.current_bouncing_back;
    }

    fn toggle_bouncing_back(&mut self) {
        match self.playback.initial_bouncing_back {
            None => {
                self.playback.initial_bouncing_back = Some(false);
                self.playback.current_bouncing_back = Some(false);
            },

            Some(_) => self.playback.initial_bouncing_back = None
        }
    }

    fn bouncing_back_enabled(&self) -> bool {
        self.playback.initial_bouncing_back.is_some()
    }

    fn planet(&self) -> Option<Planet> { self.planet }

    fn set_planet(&mut self, planet: Option<Planet>) {
        self.planet = planet;
        match &self.planet {
            Some(planet) => {
                self.src_params.flattening = planet.flattening();
                self.src_params.sidereal_rotation_period = planet.sidereal_rotation();
                self.src_params_subscribers.notify(&self.src_params);
                self.src_params_subscribers.notify(&self.src_params);
                self.render();
            },

            None => ()
        }
    }

    fn frame_interval(&self) -> Duration { self.src_params.frame_interval }

    fn set_frame_interval(&mut self, interval: Duration) {
        self.src_params.frame_interval = interval;
        self.src_params_subscribers.notify(&self.src_params);
    }

    pub fn src_params(&self) -> &SourceParameters { &self.src_params }

    fn sidereal_rotation_period(&self) -> Duration { self.src_params.sidereal_rotation_period }

    fn set_sidereal_rotation_period(&mut self, value: Duration) {
        self.src_params.sidereal_rotation_period = value;
        self.src_params_subscribers.notify(&self.src_params);
    }

    fn disk_diameter(&self) -> f32 { self.src_params.disk_diameter }

    fn set_disk_diameter(&mut self, value: f32) {
        self.src_params.disk_diameter = value;
        self.src_params_subscribers.notify(&self.src_params);
        self.render();
    }

    fn disk_center(&self) -> Point2<f32> { self.src_params.disk_center }

    fn set_disk_center(&mut self, value: Point2<f32>) {
        self.src_params.disk_center = value;
        self.src_params_subscribers.notify(&self.src_params);
        self.render();
    }
}

fn check_sizes_match(src_images: &[Rc<Texture2d>]) -> [u32; 2 ] {
    let mut image_size: Option<[u32; 2]> = None;

    for image in src_images {
        match image_size {
            None => image_size = Some([image.width(), image.height()]),
            Some(image_size) => assert!(image_size[0] == image.width() && image_size[1] == image.height())
        }
    }

    image_size.unwrap()
}

pub fn handle_source_view(
    ui: &imgui::Ui,
    gui_state: &mut GuiState,
    view: &mut SourceView,
    allow_playback: bool
) {
    imgui::Window::new(ui, &format!("Source images"))
        .size([640.0, 640.0], imgui::Condition::FirstUseEver)
        .build(|| {
            {
                let planet_names = [
                    Planet::Jupiter.name(),
                    Planet::Mars.name(),
                    "custom"
                ];
                let index_custom = planet_names.len() - 1;

                let prev_index: usize = if let Some(planet) = view.planet { planet.as_index() } else { index_custom };

                let mut index = prev_index;
                gui::add_text_before(ui, "planet");
                ui.combo_simple_string("##planet-list", &mut index, &planet_names);
                if index != prev_index {
                    if index == index_custom {
                        view.set_planet(None);
                    } else {
                        view.set_planet(Some(Planet::from(index)));
                    }
                }
            }

            // Flattening slider --------------------------------------------

            gui::add_text_before(ui, "flattening");
            gui::tooltip(ui, "Planet flattening.");
            let mut value = view.flattening();
            let token = ui.begin_disabled(view.planet().is_some());
            if imgui::Slider::new("##planet-flattening", 0.0, 0.07)
                .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                .display_format("%0.5f")
                .build(ui, &mut value)
            {
                view.set_flattening(value);
            }
            token.end();

            // Sidereal rotation period --------------------------------------

            gui::add_text_before(ui, "rotation period");
            gui::tooltip(ui, "Sidereal rotation period.");
            let token = ui.begin_disabled(view.planet().is_some());
            let mut value = view.sidereal_rotation_period().as_secs() as i32;
            if ui.input_int("##planet-rotation-period", &mut value)
                .display_format("%d s")
                .enter_returns_true(true)
                .build()
            {
                if value > 0 { view.set_sidereal_rotation_period(Duration::from_secs(value as u64)); }
            }
            token.end();

            // Inclination slider --------------------------------------------

            gui::add_text_before(ui, "inclination");
            gui::tooltip(ui, "Inclination of planet's rotation axis towards observer.");
            let mut value = view.inclination().0;
            if imgui::Slider::new("##planet-inclination", -5.0, 5.0)
                .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                .display_format("%0.2f°")
                .build(ui, &mut value)
            {
                view.set_inclination(Deg(value));
            }

            // Disk -----------------------------------

            ui.tree_node_config("disk").build(|| {
                gui::add_text_before(ui, "diameter");
                gui::tooltip(ui, "Disk diameter (equatorial) in pixels.");
                let mut value = view.disk_diameter();
                if ui.input_float("##disk-diameter", &mut value).step(0.1).step_fast(1.0).display_format("%0.1f").build() {
                    if value > 10.0 { view.set_disk_diameter(value); }
                }

                let mut value = view.disk_center();

                gui::add_text_before(ui, "center.X");
                if ui.input_float("##disk-center-x", &mut value.x).step(0.1).step_fast(1.0).display_format("%0.1f").build() {
                    view.set_disk_center(value);
                }

                gui::add_text_before(ui, "center.Y");
                if ui.input_float("##disk-center-y", &mut value.y).step(0.1).step_fast(1.0).display_format("%0.1f").build() {
                    view.set_disk_center(value);
                }
            });

            // Frame interval --------------------------------------------

            gui::add_text_before(ui, "frame interval");
            gui::tooltip(ui, "Time interval between frames.");
            let mut value = view.frame_interval().as_secs() as i32;
            if ui.input_int("##frame-interval", &mut value)
                .display_format("%d s")
                .enter_returns_true(true)
                .build()
            {
                if value > 0 && value < 10_000 { view.set_frame_interval(Duration::from_secs(value as u64)); }
            }

            // Roll --------------------------------------------

            handle_roll_controls(ui, view);

            // Playback controls -----------------------------------------------

            ui.separator();

            let bsize = [ui.calc_text_size("MM")[0], 0.0];

            gui::add_text_before(ui, "playback");

            if view.playing() {
                if ui.button_with_size("■", bsize) { view.toggle_playing(); }
                gui::tooltip(ui, "Stop playback.");
            } else {
                if ui.button_with_size("▶", bsize) { view.toggle_playing(); }
                gui::tooltip(ui, "Start playback.");
            }

            ui.same_line();
            let token: Option<_> = if view.bouncing_back_enabled() {
                Some(ui.push_style_color(imgui::StyleColor::Button, [0.0, 0.7, 0.0, 1.0]))
            } else {
                None
            };
            if ui.button_with_size("⇄", bsize) {
                view.toggle_bouncing_back();
            }
            if let Some(token) = token { token.pop(); }
            gui::tooltip(ui, "Play frames with bouncing back.");

            ui.same_line();
            gui::add_text_before(ui, "FPS");
            let mut value = view.fps();
            if imgui::Slider::new("###playback-fps", 1, 200)
                .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                .build(ui, &mut value)
            {
                view.set_fps(value);
            }

            // Current frame --------------------------------------------

            gui::add_text_before(ui, "frame");

            let token = ui.begin_disabled(view.playing());

            let current_idx = view.current_image_idx();
            if ui.arrow_button("##prev-frame", imgui::Direction::Left) {
                if current_idx > 0 {
                    view.set_image_idx(current_idx - 1)
                }
            }
            gui::tooltip(ui, "Previous frame.");
            ui.same_line();
            if ui.arrow_button("##next-frame", imgui::Direction::Right) {
                view.set_image_idx(current_idx + 1);
            }
            gui::tooltip(ui, "Next frame.");
            ui.same_line();

            let mut value = view.current_image_idx() as u32 + 1;
            if imgui::Slider::new(format!("{}/{}###source-image-idx", value, view.num_images()), 1, view.num_images() as u32)
                .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                .build(ui, &mut value)
            {
                let new_idx = value as usize - 1;
                view.set_image_idx(new_idx);
            }

            token.end();

            // Source image --------------------------------------------

            let hidpi_f = gui_state.hidpi_factor() as f32;
            let mut adjusted = gui::adjust_pos_for_exact_hidpi_scaling(ui, 0.0, hidpi_f);
            if adjusted.logical_size[1] != 0.0 && view.image_size()[1] != 0 {
                adjusted.logical_size = gui::touch_from_inside(view.image_size, adjusted.logical_size);
                adjusted.physical_size = [
                    (adjusted.logical_size[0] * hidpi_f).trunc() as u32,
                    (adjusted.logical_size[1] * hidpi_f).trunc() as u32
                ];
            }

            view.update_size(
                adjusted.physical_size[0],
                adjusted.physical_size[1]
            );

            imgui::Image::new(view.display_buf_id(), adjusted.logical_size).build(ui);
        }
    );

    if allow_playback {
        view.play(); //TODO: make it future-proof if e.g. Dear ImGUI moves to doing only limited number of refreshes on no user input
    }
}

fn handle_roll_controls(ui: &imgui::Ui, view: &mut SourceView) {
    gui::add_text_before(ui, "roll");
    gui::tooltip(ui, "Source image roll.");

    let mut value = view.roll().0;

    // TODO: extract into a widget, allow overlapping ranges (need to persist combo index)

    const COARSE_RANGES: [[f32; 2]; 5] = [
        [-50.0, -30.0],
        [-30.0, -10.0],
        [-10.0, 10.0],
        [10.0, 30.0],
        [30.0, 50.0]
    ];
    const COARSE_LABELS: [&str; 5] = [
        "-50°..-30°",
        "-30°..-10°",
        "-10°..10°",
        "10°..30°",
        "30°..50°"
    ];

    let mut index = {
        let mut idx = None;
        for (i, range) in COARSE_RANGES.iter().enumerate() {
            if value > range[0] && value < range[1] { idx = Some(i); break; }
        }
        idx.unwrap()
    };
    let w = ui.push_item_width(ui.calc_text_size("MMMMMMMMMM")[0]);
    if ui.combo_simple_string("##coarse-roll", &mut index, &COARSE_LABELS) {
        value = (COARSE_RANGES[index][0] + COARSE_RANGES[index][1]) / 2.0;
        view.set_roll(Deg(value));
    }
    w.end();

    ui.same_line();
    if imgui::Slider::new("##planet-roll", COARSE_RANGES[index][0] + 0.01, COARSE_RANGES[index][1] - 0.01)
        .flags(imgui::SliderFlags::ALWAYS_CLAMP)
        .display_format("%0.2f°")
        .build(ui, &mut value)
    {
        view.set_roll(Deg(value));
    }
}

fn advance_current_frame(
    start: usize,
    count_from_start: usize,
    total: usize,
    initial_bouncing_back: &Option<bool>,
    current_bouncing_back: &mut Option<bool>
) -> usize {
    if total == 1 { return 0; }

    match initial_bouncing_back {
        None => (start + count_from_start) % total,

        Some(initial_bouncing_back) => {
            if !*initial_bouncing_back {
                let mod_result = ((start + count_from_start) % (total + total - 2)) as i32;

                if mod_result < total as i32 {
                    *current_bouncing_back = Some(false);
                    mod_result as usize
                } else {
                    *current_bouncing_back = Some(true);
                    (-mod_result + 2 * total as i32 - 2) as usize
                }
            } else {
                let corrected_start = total as i32 - start as i32 - 2;
                let mod_result = (corrected_start + count_from_start as i32) % (total as i32 + total as i32 - 2);

                if mod_result < total as i32 - 2 {
                    *current_bouncing_back = Some(true);
                    (total as i32 - 2 - mod_result) as usize
                } else {
                    *current_bouncing_back = Some(false);
                    mod_result as usize - (total - 2)
                }
            }
        }
    }
}

mod tests {
    use super::*;

    #[test]
    fn count_frames_without_wrap() {
        // 0 1 2 3 4 5 6 | 0 1 2 3 4 5 6
        //     |     |
        //  start   end
        let initial_bouncing_back: Option<bool> = None;
        let mut current_bouncing_back: Option<bool> = None;
        assert_eq!(5, advance_current_frame(2, 3, 7, &initial_bouncing_back, &mut current_bouncing_back));
    }

    #[test]
    fn count_frames_with_wrap_no_bounce() {
        // 0 1 2 3 4 | 0 1 2 3 4
        //     |       |
        //  start     end
        let initial_bouncing_back: Option<bool> = None;
        let mut current_bouncing_back: Option<bool> = None;
        assert_eq!(0, advance_current_frame(2, 3, 5, &initial_bouncing_back, &mut current_bouncing_back));
    }

    #[test]
    fn count_frames_with_single_wrap_and_bounce_going_forward() {
        // 0 1 2 3 4 | 3 2 1 | 0 1 2 3 4
        //     |       |
        //  start     end
        let initial_bouncing_back = Some(false);
        let mut current_bouncing_back: Option<bool> = None;
        assert_eq!(3, advance_current_frame(2, 3, 5, &initial_bouncing_back, &mut current_bouncing_back));
        assert_eq!(true, *current_bouncing_back.as_ref().unwrap());
    }

    #[test]
    fn count_frames_with_double_wrap_and_bounce_going_forward() {
        // 0 1 2 3 4 | 3 2 1 | 0 1 2 3 4
        //     |                 |
        //  start               end
        let initial_bouncing_back = Some(false);
        let mut current_bouncing_back: Option<bool> = None;
        assert_eq!(1, advance_current_frame(2, 7, 5, &initial_bouncing_back, &mut current_bouncing_back));
        assert_eq!(false, *current_bouncing_back.as_ref().unwrap());
    }

    #[test]
    fn count_frames_with_single_wrap_and_bounce_going_backward() {
        // 0 1 2 3 4 | 3 2 1 | 0 1 2 3 4
        //               |     |
        //            start   end
        let initial_bouncing_back = Some(true);
        let mut current_bouncing_back: Option<bool> = None;
        assert_eq!(0, advance_current_frame(2, 2, 5, &initial_bouncing_back, &mut current_bouncing_back));
        assert_eq!(false, *current_bouncing_back.as_ref().unwrap());
    }

    #[test]
    fn count_frames_with_double_wrap_and_bounce_going_backward() {
        // 0 1 2 3 4 | 3 2 1 | 0 1 2 3 4 | 3 2 1
        //               |                 |
        //             start              end
        let initial_bouncing_back = Some(true);
        let mut current_bouncing_back: Option<bool> = None;
        assert_eq!(3, advance_current_frame(2, 7, 5, &initial_bouncing_back, &mut current_bouncing_back));
        assert_eq!(true, *current_bouncing_back.as_ref().unwrap());
    }
}
