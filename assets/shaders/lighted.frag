#version 330 core

// Outputs colors in RGBA
out vec4 FragmentColorRGBA;


/** Imports from the Vertex Shader **/
in vec3 color;
in vec2 textureCoordinates;
in vec3 normal;
in vec3 currentPosition;

// Gets data from the C++ code
uniform sampler2D uTextureUnits;
uniform vec4 uLightColor;
uniform vec3 uLightPosition;
uniform vec3 uCameraPosition;
uniform bool uHasTexture;
uniform vec3 uDiffuseColor;

void main()
{
	// ambient lighting
	float ambient = 0.20f;

	// diffuse lighting
	vec3 normalVec = normalize(normal);
	vec3 lightDirection = normalize(uLightPosition - currentPosition);
	float diffuse = max(dot(normalVec, lightDirection), 0.0f);

	// specular lighting
	float specularLight = 0.50f;
	vec3 viewDirection = normalize(uCameraPosition - currentPosition);
	vec3 reflectionDirection = reflect(-lightDirection, normalVec);
	float specAmount = pow(max(dot(viewDirection, reflectionDirection), 0.0f), 8);
	float specular = specAmount * specularLight;

	// outputs final color
	vec4 texColor = uHasTexture ? texture(uTextureUnits, textureCoordinates) : vec4(uDiffuseColor, 1.0);
	FragmentColorRGBA = texColor;
}