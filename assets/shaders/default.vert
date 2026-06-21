#version 140

in vec3 position;
in vec2 uv;

out vec2 textureCoordinates;

uniform mat4 matrix;    // Transformación del Modelo/Mundo
uniform mat4 vp;

void main() {
    // Multiplicación limpia de matrices de 4x4 con un vec4
    gl_Position = vp * matrix * vec4(position, 1.0);
    
    // Pasamos los UVs intactos al Fragment Shader
    textureCoordinates = uv;
}
