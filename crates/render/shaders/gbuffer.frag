#version 450
layout(location = 0) in vec3 v_normal;
layout(location = 1) in vec4 v_color;
layout(location = 2) in vec2 v_uv;
layout(location = 0) out vec4 out_albedo;
layout(location = 1) out vec4 out_normal;

layout(set = 1, binding = 0) uniform sampler2D u_albedo;

void main() {
    out_albedo = texture(u_albedo, v_uv) * v_color;
    out_normal = vec4(normalize(v_normal) * 0.5 + 0.5, 1.0);
}
