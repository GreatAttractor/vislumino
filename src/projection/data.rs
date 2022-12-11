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

use cgmath::{Angle, Deg, Rad};
use crate::config::ProjectionConfig;
use crate::data::{BaseProgramData, Vertex2, Vertex3};
use crate::gui::long_task_dialog::LongTaskDialog;
use crate::long_fg_task::LongForegroundTask;
use crate::projection::{ExportDialog, GlobeView, ProjectionView, SourceView, worker};
use glium::{glutin, program};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Copy, Clone)]
pub struct LonLatVertex {
    // values in degrees; -180° ⩽ longitude ⩽ 180°, -90° ⩽ latitude ⩽ 90°
    lonlat_position: [f32; 2]
}
glium::implement_vertex!(LonLatVertex, lonlat_position);

#[derive(Clone)]
pub struct LonLatGlBuffers {
    pub vertices: Rc<glium::VertexBuffer<LonLatVertex>>,
    pub indices: Rc<glium::IndexBuffer<u32>>,
}

pub struct OpenGlObjects {
    pub texture_copy_single: Rc<glium::Program>,
    pub texture_copy_multi: Rc<glium::Program>,
    pub projection: Rc<glium::Program>,
    pub solid_color_2d: Rc<glium::Program>,
    pub solid_color_3d: Rc<glium::Program>,
    pub globe_texturing: Rc<glium::Program>,
    pub unit_quad: Rc<glium::VertexBuffer<Vertex2>>,
    pub unit_circle: Rc<glium::VertexBuffer<Vertex3>>,
    pub globe_mesh: LonLatGlBuffers
}

pub struct ImageLoading {
    pub textures: Vec<Rc<glium::Texture2d>>,
    pub receiver: crossbeam::channel::Receiver<worker::LoadImagesResultMsg>
}

pub struct ProgramData {
    base: RefCell<BaseProgramData>,

    id_counter: Rc<RefCell<u32>>,

    pub gl_objects: OpenGlObjects,

    source_view: Option<SourceView>, // empty until images are loaded for the first time

    globe_views: RefCell<Vec<Rc<RefCell<GlobeView>>>>,

    projection_views: RefCell<Vec<Rc<RefCell<ProjectionView>>>>,

    long_task_dialog: RefCell<Option<LongTaskDialog>>,

    long_fg_task: RefCell<Option<Box<dyn LongForegroundTask>>>,

    bg_task_sender: crossbeam::channel::Sender<crate::projection::worker::MainToWorkerMsg>,

    export_dialog: RefCell<ExportDialog>,

    image_loading: Option<ImageLoading>
}

impl ProgramData {
    pub fn new(
        base: BaseProgramData,
        display: &glium::Display,
        worker_context: glutin::Context<glutin::NotCurrent>
    ) -> ProgramData {
        let texture_copy_single = Rc::new(program!(display,
            330 => {
                vertex: include_str!("../resources/shaders/pass-through.vert"),
                fragment: include_str!("../resources/shaders/texturing.frag"),
            }
        ).unwrap());

        let texture_copy_multi = Rc::new(program!(display,
            330 => {
                vertex: include_str!("../resources/shaders/pass-through.vert"),
                fragment: include_str!("../resources/shaders/texturing_multi-sample.frag"),
            }
        ).unwrap());

        let projection = Rc::new(program!(display,
            330 => {
                vertex: include_str!("../resources/shaders/transform_2d.vert"),
                fragment: include_str!("../resources/shaders/projection.frag"),
            }
        ).unwrap());

        let solid_color_2d = Rc::new(program!(display,
            330 => {
                vertex: include_str!("../resources/shaders/transform_2d.vert"),
                fragment: include_str!("../resources/shaders/solid_color.frag"),
            }
        ).unwrap());

        let solid_color_3d = Rc::new(program!(display,
            330 => {
                vertex: include_str!("../resources/shaders/transform_3d.vert"),
                fragment: include_str!("../resources/shaders/solid_color.frag"),
            }
        ).unwrap());

        let globe_texturing = Rc::new(program!(display,
            330 => {
                vertex: include_str!("../resources/shaders/globe.vert"),
                fragment: include_str!("../resources/shaders/globe_texturing.frag")
            }
        ).unwrap());

        let globe_mesh = create_globe_mesh(cgmath::Deg(2.0), display);

        let gl_objects = OpenGlObjects{
            texture_copy_single,
            texture_copy_multi,
            projection,
            solid_color_2d,
            solid_color_3d,
            globe_texturing,
            unit_quad: create_unit_quad(display),
            unit_circle: create_unit_circle(256, display),
            globe_mesh
        };

        let (bg_task_sender, bg_task_receiver) = crossbeam::channel::unbounded();

        std::thread::spawn(move || { crate::projection::worker::worker(worker_context, bg_task_receiver); });

        let export_dialog = RefCell::new(ExportDialog::new(
            "Export images".to_string(),
            base.config.projection_export_path().into()
        ));

        ProgramData{
            base: RefCell::new(base),
            id_counter: Rc::new(RefCell::new(0)),
            gl_objects,
            source_view: None,
            globe_views: RefCell::new(vec![]),
            projection_views: RefCell::new(vec![]),
            long_fg_task: RefCell::new(None),
            long_task_dialog: RefCell::new(None),
            bg_task_sender,
            export_dialog,
            image_loading: None
        }
    }

    pub fn base(&self) -> &RefCell<BaseProgramData> { &self.base }

    pub fn image_loading(&self) -> &Option<ImageLoading> { &self.image_loading }

    pub fn image_loading_mut(&mut self) -> &mut Option<ImageLoading> { &mut self.image_loading }

    pub fn long_fg_task(&self) -> &RefCell<Option<Box<dyn LongForegroundTask>>> { &self.long_fg_task }

    pub fn long_task_dialog(&self) -> &RefCell<Option<LongTaskDialog>> { &self.long_task_dialog }

    pub fn new_unique_id(&self) -> u32 {
        let new_id = *self.id_counter.borrow();
        *self.id_counter.borrow_mut() += 1;

        new_id
    }

    pub fn source_view(&self) -> &Option<SourceView> { &self.source_view }

    pub fn source_view_mut(&mut self) -> &mut Option<SourceView> { &mut self.source_view }

    pub fn globe_views(&self) -> &RefCell<Vec<Rc<RefCell<GlobeView>>>> { &self.globe_views }

    pub fn projection_views(&self) -> &RefCell<Vec<Rc<RefCell<ProjectionView>>>> { &self.projection_views }

    pub fn add_projection_view(
        &mut self,
        display: &glium::Display,
        renderer: &Rc<RefCell<imgui_glium_renderer::Renderer>>
    ) {
        let id = self.new_unique_id();

        let source_view = self.source_view.as_mut().unwrap();

        let projection_view = Rc::new(RefCell::new(ProjectionView::new(
            id,
            &self.gl_objects,
            display,
            renderer,
            &source_view.current_image(),
            source_view.current_image_idx(),
            source_view.src_params().clone(),
            0.0
        )));

        source_view.subscribe_current_img(Rc::downgrade(&projection_view) as _);
        source_view.subscribe_src_params(Rc::downgrade(&projection_view) as _);

        self.projection_views.borrow_mut().push(projection_view);
    }

    pub fn add_globe_view(
        &mut self,
        display: &glium::Display,
        renderer: &Rc<RefCell<imgui_glium_renderer::Renderer>>
    ) {
        let id = self.new_unique_id();

        let source_view = self.source_view.as_mut().unwrap();

        let globe_view = Rc::new(RefCell::new(GlobeView::new(
            id,
            &self.gl_objects,
            display,
            renderer,
            &source_view.current_image(),
            source_view.current_image_idx(),
            source_view.src_params().clone()
        )));

        source_view.subscribe_current_img(Rc::downgrade(&globe_view) as _);
        source_view.subscribe_src_params(Rc::downgrade(&globe_view) as _);

        self.globe_views.borrow_mut().push(globe_view);
    }

    pub fn bg_task_sender(&self) -> &crossbeam::channel::Sender<worker::MainToWorkerMsg> { &self.bg_task_sender }

    pub fn export_dialog(&self) -> &RefCell<ExportDialog> { &self.export_dialog }
}

pub fn create_unit_quad(display: &dyn glium::backend::Facade) -> Rc<glium::VertexBuffer<Vertex2>> {
    let unit_quad_data = [
        Vertex2{ position: [-1.0, -1.0] },
        Vertex2{ position: [ 1.0, -1.0] },
        Vertex2{ position: [ 1.0,  1.0] },
        Vertex2{ position: [-1.0,  1.0] }
    ];

    Rc::new(glium::VertexBuffer::new(display, &unit_quad_data).unwrap())
}

fn create_unit_circle(num_segments: usize, display: &impl glium::backend::Facade) -> Rc<glium::VertexBuffer<Vertex3>> {
    let mut circle_points = vec![];
    for i in 0..num_segments {
        let angle = Rad::from(Deg::<f32>(360.0) / num_segments as f32) * i as f32;
        circle_points.push(Vertex3{ position: [angle.0.cos(), angle.0.sin(), 0.0] });
    }

    Rc::new(glium::VertexBuffer::new(display, &circle_points).unwrap())
}

/// Generates user-facing half of parallel.
pub fn create_half_parallel(
    latitude: Deg<f32>,
    num_segments: usize,
    display: &impl glium::backend::Facade
) -> glium::VertexBuffer<Vertex3> {
    let mut points = vec![];

    let y = latitude.sin();
    let radius = latitude.cos();

    for i in 0..num_segments {
        let angle = Deg::<f32>(180.0) / num_segments as f32 * i as f32;
        let x = radius * angle.cos();
        let z = radius * angle.sin();

        points.push(Vertex3{ position: [x, y, z] });
    }

    glium::VertexBuffer::new(display, &points).unwrap()
}

fn create_globe_mesh(
    step: cgmath::Deg<f64>,
    display: &glium::Display
) -> LonLatGlBuffers {
    assert!((360.0 / step.0).fract() == 0.0);

    let grid_size_lon = (360.0 / step.0) as usize + 1;
    let grid_size_lat = (180.0 / step.0) as usize - 1;

    let mut vertex_data: Vec<LonLatVertex> = vec![];

    let mut latitude = -90.0 + step.0;
    for _ in 0..grid_size_lat {
        let mut longitude = -180.0;
        for _ in 0..grid_size_lon {
            vertex_data.push(LonLatVertex{ lonlat_position: [longitude as f32, latitude as f32] });
            longitude += step.0;
        }
        latitude += step.0;
    }

    let mut index_data: Vec<u32> = vec![];

    macro_rules! v_index {
        ($i_lon:expr, $i_lat:expr) => { (($i_lon) % grid_size_lon + ($i_lat) * grid_size_lon) as u32 }
    }

    for i_lon in 0..grid_size_lon {
        for i_lat in 0..grid_size_lat - 1 {
            index_data.push(v_index!(i_lon,     i_lat));
            index_data.push(v_index!(i_lon,     i_lat + 1));
            index_data.push(v_index!(i_lon + 1, i_lat));

            index_data.push(v_index!(i_lon + 1, i_lat));
            index_data.push(v_index!(i_lon + 1, i_lat + 1));
            index_data.push(v_index!(i_lon,     i_lat + 1));
        }
    }

    vertex_data.push(LonLatVertex{ lonlat_position: [0.0, -90.0] }); // south cap
    let s_cap_idx = vertex_data.len() as u32 - 1;
    vertex_data.push(LonLatVertex{ lonlat_position: [0.0,  90.0] }); // north cap
    let n_cap_idx = vertex_data.len() as u32 - 1;

    for i_lon in 0..grid_size_lon {
        // south cap
        index_data.push(v_index!(i_lon, 0));
        index_data.push(v_index!(i_lon + 1, 0));
        index_data.push(s_cap_idx);

        // north cap
        index_data.push(v_index!(i_lon, grid_size_lat - 1));
        index_data.push(v_index!(i_lon + 1, grid_size_lat - 1));
        index_data.push(n_cap_idx);
    }

    let vertices = Rc::new(glium::VertexBuffer::new(display, &vertex_data).unwrap());
    let indices = Rc::new(glium::IndexBuffer::new(display, glium::index::PrimitiveType::TrianglesList, &index_data).unwrap());

    LonLatGlBuffers{ vertices, indices }
}
