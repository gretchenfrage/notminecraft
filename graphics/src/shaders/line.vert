#version 450

layout(set=0, binding=0) uniform u {
    mat4 u_transform;
    vec4 u_color;
    mat4 u_screen_to_world;
    float u_fog_mul;
    float u_fog_add;
    float u_day_night_time;
};

layout(location=0) out vec4 o_pos;

void main() {
    o_pos = u_transform * vec4(gl_VertexIndex, 0, 0, 1);
    gl_Position = o_pos;
}
