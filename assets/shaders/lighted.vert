#version 330 core

// Positions/Coordinates
layout (location = 0) in vec3 aPos;
// Texture Coordinates
layout (location = 1) in vec2 aTex;
// normals (not necessarily normalized)
layout (location = 2) in vec3 anormal;


/** Outputs to the Fragment Shader **/
out vec2 textureCoordinates;
out vec3 normal;
out vec3 currentPosition;

/** Imports from the main function **/
uniform mat4 camMatrix;
uniform mat4 model;


void main()
{
	// Outputs the positions/coordinates of all vertices
	currentPosition = vec3(model * vec4(aPos, 1.0f));
	gl_Position = camMatrix * vec4(currentPosition, 1.0);

	textureCoordinates = aTex;
	normal = anormal;
}