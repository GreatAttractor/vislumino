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

#version 330 core

in vec2 lonlat_out;

uniform sampler2D source_image;
uniform float disk_diameter; // value in pixels
uniform vec2 disk_center; // value in pixels
/// Transformation from normalized (within [-1; 1]) globe coordinates to normalized (within [-1; 1]) image coordinates
/// within the disk; compensates for planet flattening, planet inclination and image roll.
///
/// Globe coordinates: assuming spherical planet centered at (0, 0, 0), X points right, Y points up, Z points
/// to observer. Observer faces latitude 0°, longitude 0°.
///
/// Image disk coordinates: disk center is (0, 0), disk spans [-1; 1] in X, proportionally less in Y (depending
/// on flattening and inclination).
///
uniform mat3 globe_transform;
uniform bool texture_vertical_flip;

out vec4 output_color;

void main()
{
    vec2 source_size = vec2(textureSize(source_image, 0));

    float lon = radians(lonlat_out.x);
    float lat = radians(lonlat_out.y);

    vec3 globe_pos = vec3(
        cos(lat) * sin(lon),
        sin(lat),
        cos(lat) * cos(lon)
    );

    vec3 corrected_disk_pos = globe_transform * globe_pos;
    if (texture_vertical_flip)
    {
        corrected_disk_pos.y = -corrected_disk_pos.y;
    }

    vec2 image_disk_pos = disk_center / source_size + (corrected_disk_pos.xy * disk_diameter / 2) / source_size;

    if (corrected_disk_pos.z >= 0.0)
    {
        output_color = vec4(texture(source_image, image_disk_pos).rgb, 1.0);
    }
    else
    {
        output_color = vec4(vec3(0.0), 1.0);
    }
}
