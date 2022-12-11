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

in vec2 tex_coord;
out vec4 output_color;

uniform sampler2DMS source_texture;

void main()
{
    vec4 color = vec4(0.0);

    ivec2 texel = ivec2(tex_coord * textureSize(source_texture)); //TODO: provide texture size as a uniform for better speed?

    //TODO: provide additional input with sample mask, sum only edge samples?
    for (int i = 0; i < 8; ++i) //TODO: provide sample count as uniform
    {
        color += texelFetch(source_texture, texel, i);
    }
    color /= 8.0;

    output_color = color;
}
