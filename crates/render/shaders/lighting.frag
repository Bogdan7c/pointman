#version 450
layout(location = 0) in vec2 v_uv;
layout(location = 0) out vec4 out_color;

layout(set = 0, binding = 0) uniform Frame {
    mat4 view_proj;
    mat4 inv_view_proj;
    vec4 camera_pos;
    vec4 ambient;
    vec4 pos_radius[8];
    vec4 color_intensity[8];
    vec4 dir_cone[8];
    uint light_count;
    uint _pad0;
    uint _pad1;
    uint _pad2;
} frame;

layout(set = 1, binding = 0) uniform sampler2D u_albedo;
layout(set = 1, binding = 1) uniform sampler2D u_normal;
layout(set = 1, binding = 2) uniform sampler2D u_spec;
layout(set = 1, binding = 3) uniform sampler2D u_depth;

void main() {
    vec4 albedo = texture(u_albedo, v_uv);
    float depth = texture(u_depth, v_uv).r;
    if (depth >= 0.9999) {
        out_color = vec4(frame.ambient.rgb * 0.35, 1.0);
        return;
    }

    vec3 n = normalize(texture(u_normal, v_uv).xyz * 2.0 - 1.0);
    vec4 spec = texture(u_spec, v_uv);
    float power = max(spec.a * 255.0, 1.0);
    vec4 clip = vec4(v_uv * 2.0 - 1.0, depth, 1.0);
    vec4 world = frame.inv_view_proj * clip;
    vec3 pos = world.xyz / world.w;
    vec3 view = normalize(frame.camera_pos.xyz - pos);

    vec3 lit = albedo.rgb * max(frame.ambient.rgb, vec3(0.08));
    uint count = min(frame.light_count, 8u);
    for (uint i = 0u; i < count; i++) {
        vec3 lpos = frame.pos_radius[i].xyz;
        float radius = max(frame.pos_radius[i].w, 0.001);
        vec3 L = lpos - pos;
        float dist = length(L);
        L /= max(dist, 0.0001);
        float att = clamp(1.0 - dist / radius, 0.0, 1.0);
        att *= att;
        // Спот фонарика: xyz — направление луча, w — cos внешнего угла. w<=0 = omni.
        vec3 cone_dir = frame.dir_cone[i].xyz;
        float outer_cos = frame.dir_cone[i].w;
        float spot = 1.0;
        if (outer_cos > 0.0 && dot(cone_dir, cone_dir) > 0.01) {
            float inner_cos = mix(outer_cos, 1.0, 0.4);
            float cd = dot(normalize(cone_dir), -L);
            spot = smoothstep(outer_cos, inner_cos, cd);
        }
        att *= spot;
        float ndotl = max(dot(n, L), 0.0);
        float spec_term = pow(max(dot(n, normalize(L + view)), 0.0), power);
        vec3 col = frame.color_intensity[i].rgb * frame.color_intensity[i].a;
        lit += (albedo.rgb * ndotl + spec.rgb * spec_term) * col * att;
    }
    out_color = vec4(lit, 1.0);
}
