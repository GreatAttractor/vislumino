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

use cgmath::Point2;
use crate::data;
use crate::data::TextureId;
use crate::gui::long_task_dialog::ProgressMsg;
use crate::image_utils;
use crate::projection;
use crate::projection::projection_view::ProjectionType;
use crossbeam::channel::TrySendError;
use glium::{glutin, Texture2d, program};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::rc::Rc;

const PI_2: f32 = std::f32::consts::PI / 2.0;

pub struct ProcessTexture {
    pub id: TextureId,
    pub dimensions: glium::texture::Dimensions
}

pub struct DummyJob {
    pub sender: crossbeam::channel::Sender<ProgressMsg>
}

pub struct Projection {
    pub sender: crossbeam::channel::Sender<ProgressMsg>,
    pub image_size: glium::texture::Dimensions,
    pub source_texture_ids: Vec<TextureId>,
    pub output_dir: std::path::PathBuf,
    /// If true, outputs processed images twice (except the last one), in forward and reverse order.
    pub bounce_back: bool,
    pub src_params: projection::source_view::SourceParameters,
    pub rotation_comp: f32,
    pub projection_type: projection::projection_view::ProjectionType
}

pub struct LoadImages {
    pub dimensions: [u32; 2],
    pub pixel_format: ga_image::PixelFormat,
    pub items: Vec<(TextureId, PathBuf)>,
    pub progress_sender: crossbeam::channel::Sender<ProgressMsg>,
    pub result_sender: crossbeam::channel::Sender<LoadImagesResultMsg>
}

pub struct DiskInfo {
    pub center: Point2<f32>,
    pub diameter: f32
}

pub enum LoadImagesResultMsg {
    Success(DiskInfo),
    Error(String),
    Cancelled
}

pub enum MainToWorkerMsg {
    Cancel,
    Projection(Projection),
    LoadImages(LoadImages)
}

pub fn worker(context: glutin::Context<glutin::NotCurrent>, receiver: crossbeam::channel::Receiver<MainToWorkerMsg>) {
    let headless = glium::HeadlessRenderer::new(context).unwrap();

    let unit_quad = projection::data::create_unit_quad(&headless);
    let projection = Rc::new(program!(&headless,
        330 => {
            vertex: include_str!("../resources/shaders/transform_2d.vert"),
            fragment: include_str!("../resources/shaders/projection.frag"),
        }
    ).unwrap());

    loop {
        match receiver.recv() {
            Ok(msg) => match msg {
                MainToWorkerMsg::Projection(task) => on_projection(
                    task,
                    &headless,
                    &unit_quad,
                    &projection,
                    &receiver
                ),

                MainToWorkerMsg::Cancel => panic!("unexpected message received"),

                MainToWorkerMsg::LoadImages(task) => on_load_images(task, &headless, &receiver)
            },

            Err(_) => break
        }
    }
}

fn on_projection(
    task: Projection,
    display: &dyn glium::backend::Facade,
    unit_quad: &glium::VertexBuffer<data::Vertex2>,
    projection_prog: &glium::Program,
    receiver: &crossbeam::channel::Receiver<MainToWorkerMsg>
) {
    //TODO: refactor DrawBuffer to also work w/out "imgui texture id"

    // let projection_draw_buf = DrawBuffer::new_with_size(
    //     Sampling::Single,
    //     &gl_objects.texture_copy_single,
    //     &gl_objects.texture_copy_multi,
    //     &unit_quad,
    //     display,
    //     //renderer,
    //     (disk_diameter * PI_2).ceil() as u32,
    //     (disk_diameter * PI_2).ceil() as u32,
    // );

    // using a plain texture as the render target for now
    let draw_buffer = Texture2d::empty_with_format(
        display,
        glium::texture::UncompressedFloatFormat::U8U8U8,
        glium::texture::MipmapsOption::NoMipmap,
        (task.src_params.disk_diameter * PI_2 + (task.src_params.num_images - 1) as f32 * task.rotation_comp).ceil() as u32,
        match task.projection_type {
            ProjectionType::Equirectangular => (task.src_params.disk_diameter * PI_2).ceil() as u32,
            ProjectionType::LambertCylindricalEqualArea => task.src_params.disk_diameter as u32
        }
    ).unwrap();

    let num_images = task.source_texture_ids.len();

    for (idx, source_texture_id) in task.source_texture_ids.iter().enumerate() {
        match receiver.try_recv() {
            Ok(msg) => match msg {
                MainToWorkerMsg::Cancel => break,
                _ => panic!("unexpected message received")
            },

            _ => ()
        }

        let source_texture = unsafe { glium::Texture2d::from_id(
            display,
            glium::texture::UncompressedFloatFormat::U8U8U8,
            *source_texture_id,
            false,
            glium::texture::MipmapsOption::NoMipmap,
            task.image_size
        ) };

        projection::projection_view::render_projection(
            false,
            idx,
            &source_texture,
            &mut draw_buffer.as_surface(),
            unit_quad,
            projection_prog,
            &task.src_params,
            task.rotation_comp,
            task.projection_type
        );

        let output_img = image_utils::image_from_texture(&draw_buffer);
        let output_path = Path::new(&task.output_dir).join(format!("output_{:05}.png", idx + 1));

        image::save_buffer(
            &output_path, output_img.raw_pixels(), output_img.width(), output_img.height(), image::ColorType::Rgb8
        ).unwrap();

        let mut progress_msg = format!("Saved {}", output_path.as_os_str().to_string_lossy());

        if task.bounce_back && idx < num_images - 1 {
            let output_path = Path::new(&task.output_dir).join(format!("output_{:05}.png", 2 * num_images - (idx + 1)));
            image::save_buffer(
                &output_path, output_img.raw_pixels(), output_img.width(), output_img.height(), image::ColorType::Rgb8
            ).unwrap();
            progress_msg += ", ";
            progress_msg += &output_path.file_name().unwrap().to_string_lossy();
        }

        progress_msg += ".";

        match task.sender.try_send(ProgressMsg::new(
            progress_msg,
            idx as f32 / task.source_texture_ids.len() as f32
        )) {
            Ok(()) => (),
            Err(err) => match err {
                TrySendError::Full(_) => (),
                TrySendError::Disconnected(_) => panic!("channel disconnected unexpectedly")
            }
        }
    }
}

fn load_single_image(
    expected_width: u32,
    expected_height: u32,
    expected_pix_fmt: ga_image::PixelFormat,
    path: &Path,
    texture: &glium::texture::Texture2d
) -> Result<ga_image::Image, Box<dyn Error>> {
    let image = image_utils::load_image(&path)?;
    if image.width() != expected_width || image.height() != expected_height {
        return Err(format!(
            "unexpected image dimensions (expected {}x{}, found {}x{})",
            expected_width, expected_height, image.width(), image.height()
        ).into());
    }

    if image.pixel_format() != expected_pix_fmt {
        return Err(format!(
            "unexpected pixel format (expected {:?}, found {:?})",
            expected_pix_fmt, image.pixel_format()
        ).into());
    }

    //TODO: handle more pixel formats
    let image = image.convert_pix_fmt(ga_image::PixelFormat::RGB8, None);

    let source = glium::texture::RawImage2d{
        data: std::borrow::Cow::<[u8]>::from(image.pixels::<u8>()),
        width: image.width(),
        height: image.height(),
        format: glium::texture::ClientFormat::U8U8U8
    };

    texture.write(glium::Rect{ left: 0, bottom: 0, width: image.width(), height: image.height() }, source);

    Ok(image)
}

fn on_load_images(
    task: LoadImages,
    display: &dyn glium::backend::Facade,
    receiver: &crossbeam::channel::Receiver<MainToWorkerMsg>
) {
    let mut disk_info: Option<DiskInfo> = None;

    for (idx, (texture_id, path)) in task.items.iter().enumerate() {
        match receiver.try_recv() {
            Ok(msg) => match msg {
                MainToWorkerMsg::Cancel => {
                    task.result_sender.send(LoadImagesResultMsg::Cancelled).unwrap();
                    return;
                },
                _ => panic!("unexpected message received")
            },

            _ => ()
        }

        let texture = unsafe { glium::Texture2d::from_id(
            display,
            glium::texture::UncompressedFloatFormat::U8U8U8,
            *texture_id,
            false,
            glium::texture::MipmapsOption::NoMipmap,
            glium::texture::Dimensions::Texture2d{ width: task.dimensions[0], height: task.dimensions[1] }
        ) };

        match load_single_image(task.dimensions[0], task.dimensions[1], task.pixel_format, path, &texture) {
            Err(e) => {
                task.result_sender.send(LoadImagesResultMsg::Error(e.to_string())).unwrap();
                return;
            },

            Ok(img) => if idx == 0 {
                match crate::disk::find_planetary_disk(&img) {
                    Ok((center, diameter)) => disk_info = Some(DiskInfo{ center, diameter }),

                    Err(_) => {
                        task.result_sender.send(
                            LoadImagesResultMsg::Error("could not find planetary disk".into())
                        ).unwrap();
                        return;
                    }
                }
            }
        }

        match task.progress_sender.try_send(ProgressMsg::new(
            format!("Loaded {}.", path.as_os_str().to_string_lossy()),
            idx as f32 / task.items.len() as f32
        )) {
            Ok(()) => (),
            Err(err) => match err {
                TrySendError::Full(_) => (),
                TrySendError::Disconnected(_) => panic!("channel disconnected unexpectedly")
            }
        }
    }

    unsafe { gl::Finish(); } // required, otherwise a few final textures would not be seen as loaded on the main thread
    task.result_sender.send(LoadImagesResultMsg::Success(disk_info.unwrap())).unwrap();
}
