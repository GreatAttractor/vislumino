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

use ga_image;
use glium::GlObject;
use image;
use image::GenericImageView;
use std::error::Error;
use std::path::Path;

/// Returns (width, height, pixel format).
pub fn get_metadata<P: AsRef<Path>>(path: P) -> Result<(u32, u32, ga_image::PixelFormat), Box<dyn Error>> {
    let image = image::open(path)?;
    get_metadata_from_image(&image)
}

fn get_metadata_from_image(image: &image::DynamicImage) -> Result<(u32, u32, ga_image::PixelFormat), Box<dyn Error>> {
    use ga_image::PixelFormat;

    let dims = image.dimensions();

    let pixel_format = match image {
        image::DynamicImage::ImageLuma8(_)  => PixelFormat::Mono8,
        image::DynamicImage::ImageRgb8(_)   => PixelFormat::RGB8,
        image::DynamicImage::ImageRgba8(_)  => PixelFormat::RGBA8,
        image::DynamicImage::ImageLuma16(_) => PixelFormat::Mono16,
        image::DynamicImage::ImageRgb16(_)  => PixelFormat::RGB16,
        image::DynamicImage::ImageRgba16(_) => PixelFormat::RGBA16,
        image::DynamicImage::ImageRgb32F(_) => PixelFormat::RGB32f,

        other => return Err(format!("unsupported pixel format {:?}", other).into())
    };

    Ok((dims.0, dims.1, pixel_format))
}

pub fn load_image(path: &std::path::Path) -> Result<ga_image::Image, Box<dyn Error>> {
    let src_image = image::open(path)?;

    let (width, height, _) = get_metadata_from_image(&src_image)?;

    let src_buffer = src_image.into_rgb8(); //TODO: handle other bit depths

    let layout = src_buffer.as_flat_samples().layout;
    assert!(layout.height_stride == layout.width as usize * layout.channels as usize); //TODO: handle line padding
    let pixels = src_buffer.into_vec();

    let image = ga_image::Image::new_from_pixels(width, height, None, ga_image::PixelFormat::RGB8, None, pixels);

    Ok(image)
}

pub fn image_from_texture(texture: &glium::Texture2d) -> ga_image::Image {
    let mut image = ga_image::Image::new(
        texture.width(),
        texture.height(),
        None,
        ga_image::PixelFormat::RGB8,
        None,
        false
    );

    unsafe {
        gl::BindBuffer(gl::PIXEL_PACK_BUFFER, 0);
        gl::PixelStorei(gl::PACK_ALIGNMENT, 1);
        gl::PixelStorei(gl::PACK_ROW_LENGTH, 0);
        gl::BindTexture(gl::TEXTURE_2D, texture.get_id());
        gl::GetTexImage(gl::TEXTURE_2D, 0, gl::RGB, gl::UNSIGNED_BYTE, image.raw_pixels_mut().as_ptr() as _);
    }

    image
}
