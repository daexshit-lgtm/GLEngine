#version 330

in vec2 v_uv;
in vec4 v_col;

uniform sampler2D tex;

out vec4 color;

void main() {
    color = v_col * texture(tex, v_uv);
}