#version 450
layout(location = 0) in vec3 in_pos;
layout(location = 1) in vec3 in_normal;
layout(location = 2) in vec2 in_uv;
layout(location = 3) in vec3 in_tangent;
layout(location = 4) in vec3 in_binormal;

layout(set = 0, binding = 0) uniform Frame {
    mat4 view_proj;
    mat4 inv_view_proj;
    vec4 camera_pos;
    vec4 ambient;
    vec4 pos_radius[8];
    vec4 color_intensity[8];
    uint light_count;
    uint _pad0;
    uint _pad1;
    uint _pad2;
} frame;

layout(push_constant) uniform PC {
    mat4 model;
    vec4 color;
    float spec_power;
    vec3 _pad;
} pc;

layout(location = 0) out vec3 v_normal;
layout(location = 1) out vec4 v_color;
layout(location = 2) out vec2 v_uv;
layout(location = 3) out vec3 v_tangent;
layout(location = 4) out vec3 v_binormal;

void main() {
    vec4 world = pc.model * vec4(in_pos, 1.0);
    gl_Position = frame.view_proj * world;
    mat3 nmat = mat3(pc.model);
    v_normal = nmat * in_normal;
    v_tangent = nmat * in_tangent;
    v_binormal = nmat * in_binormal;
    v_color = pc.color;
    v_uv = in_uv;
}
