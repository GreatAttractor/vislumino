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

in vec3 position;
uniform mat4 vertex_transform;
out vec2 tex_coord;

void main()
{
    // apply texture coords (0,1)-(1,0) to unit quad (-1,-1)-(1,1)
    tex_coord.xy = position.xy / 2 + vec2(0.5, 0.5);

    gl_Position.xyz = (vertex_transform * vec4(position, 1.0)).xyz;
    gl_Position.w = 1.0;
}
