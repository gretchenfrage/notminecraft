#version 450

layout(set=0, binding=0) uniform u {
    mat4 u_transform_2d;
    mat4 u_transform_3d;
    vec4 u_color;
};

layout(location=0) out vec4 o_pos;

void main() {
    int corner;
    switch (gl_VertexIndex) {
    case 0: corner = 0; break;
    case 1: corner = 2; break;
    case 2: corner = 1; break;
    case 3: corner = 0; break;
    case 4: corner = 3; break;
    case 5: corner = 2; break;
    }

    vec2 pos;
    switch (corner) {
    case 0: pos = vec2(0, 0); break;
    case 1: pos = vec2(1, 0); break;
    case 2: pos = vec2(1, 1); break;
    case 3: pos = vec2(0, 1); break;
    }

    o_pos = (u_transform_2d * u_transform_3d * vec4(pos, 0, 1));
    gl_Position = o_pos;
}
