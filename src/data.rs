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

use ga_image::PixelFormat;
use glium::{CapabilitiesSource};

pub type TextureId = <glium::Texture2d as glium::GlObject>::Id;

#[derive(Copy, Clone)]
pub struct Vertex2 {
    pub position: [f32; 2]
}
glium::implement_vertex!(Vertex2, position);

#[derive(Copy, Clone)]
pub struct Vertex3 {
    pub position: [f32; 3]
}
glium::implement_vertex!(Vertex3, position);

pub trait ToArray {
    type Output;
    fn to_array(&self) -> Self::Output;
}

impl<T: Copy> ToArray for cgmath::Point2<T>
{
    type Output = [T; 2];
    fn to_array(&self) -> Self::Output {
        (*self).into()
    }
}

impl<T: Copy> ToArray for cgmath::Matrix3<T>
{
    type Output = [[T; 3]; 3];
    fn to_array(&self) -> Self::Output {
        (*self).into()
    }
}

impl<T: Copy> ToArray for cgmath::Matrix4<T>
{
    type Output = [[T; 4]; 4];
    fn to_array(&self) -> Self::Output {
        (*self).into()
    }
}

pub struct BaseProgramData {
    pub config: crate::config::Configuration
}

pub enum ProgramData {
    Projection(crate::projection::ProgramData)
}

pub fn create_texture_from_image(image: &ga_image::Image, display: &glium::Display)
-> glium::Texture2d {
    let max_texture_size = display.get_capabilities().max_texture_size as u32;

    if image.width() > max_texture_size || image.height() > max_texture_size {
        panic!("image too big"); //TODO: handle gracefully
    }

    //TODO: handle other formats
    assert!(image.pixel_format() == PixelFormat::RGB8);

    let texture = glium::Texture2d::with_format(
        display,
        glium::texture::RawImage2d{
            data: std::borrow::Cow::<[u8]>::from(image.pixels::<u8>()),
            width: image.width(),
            height: image.height(),
            format: glium::texture::ClientFormat::U8U8U8
        },
        glium::texture::UncompressedFloatFormat::U8U8U8,
        glium::texture::MipmapsOption::NoMipmap
    ).unwrap();

    texture
}
