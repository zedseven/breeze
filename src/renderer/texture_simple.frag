#version 150 core

uniform sampler2D t_Current;

in vec2 v_Uv;

out vec4 Target0;

void main() {
    vec4 textureColour = texture(t_Current, v_Uv);

    // https://learnopengl.com/Advanced-OpenGL/Blending
    if (textureColour.a < 0.1)
        discard;

    Target0 = textureColour;
}
