#version 330

in vec2 pos;
in vec2 uv;
in uint col;

out vec2 v_uv;
out vec4 v_col;

uniform mat4 proj;

void main() {
    v_uv = uv;

    v_col = vec4(
        ((col >> 0)  & 255u) / 255.0,
        ((col >> 8)  & 255u) / 255.0,
        ((col >> 16) & 255u) / 255.0,
        ((col >> 24) & 255u) / 255.0
    );

    gl_Position = proj * vec4(pos, 0.0, 1.0);
}