#version 450
layout(location = 0) in vec3 in_pos;
layout(location = 1) in vec3 in_normal;

layout(set = 0, binding = 0) uniform Frame {
    mat4 view_proj;
    mat4 inv_view_proj;
    vec4 camera_pos;
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
} pc;

layout(location = 0) out vec3 v_normal;
layout(location = 1) out vec4 v_color;

void main() {
    vec4 world = pc.model * vec4(in_pos, 1.0);
    gl_Position = frame.view_proj * world;
    v_normal = mat3(pc.model) * in_normal;
    v_color = pc.color;
}
