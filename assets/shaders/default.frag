#version 140

in vec2 textureCoordinates;

out vec4 fragColor;

uniform sampler2D texture2D;

void main() {
    vec2 chunk_uv = textureCoordinates;
    fragColor = texture(texture2D, chunk_uv);
}
