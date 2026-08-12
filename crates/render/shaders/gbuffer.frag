#version 450
layout(location = 0) in vec3 v_normal;
layout(location = 1) in vec4 v_color;
layout(location = 2) in vec2 v_uv;
layout(location = 3) in vec3 v_tangent;
layout(location = 4) in vec3 v_binormal;
layout(location = 0) out vec4 out_albedo;
layout(location = 1) out vec4 out_normal;
layout(location = 2) out vec4 out_spec;

layout(set = 1, binding = 0) uniform sampler2D u_albedo;
layout(set = 1, binding = 1) uniform sampler2D u_normal;
layout(set = 1, binding = 2) uniform sampler2D u_spec;

layout(push_constant) uniform PC {
    mat4 model;
    vec4 color;
    float spec_power;
    vec3 _pad;
} pc;

void main() {
    out_albedo = texture(u_albedo, v_uv) * v_color;
    vec3 n_ts = normalize(texture(u_normal, v_uv).xyz * 2.0 - 1.0);
    vec3 t = normalize(v_tangent);
    vec3 b = normalize(v_binormal);
    vec3 n = normalize(v_normal);
    vec3 n_ws = normalize(mat3(t, b, n) * n_ts);
    out_normal = vec4(n_ws * 0.5 + 0.5, 1.0);
    vec4 spec = texture(u_spec, v_uv);
    out_spec = vec4(spec.rgb, spec.a * (pc.spec_power / 255.0));
}
