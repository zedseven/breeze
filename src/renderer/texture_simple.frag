#version 150 core

uniform sampler2D t_Current;

in vec2 v_Uv;

out vec4 Target0;

void main() {
    Target0 = texture(t_Current, v_Uv);
}
