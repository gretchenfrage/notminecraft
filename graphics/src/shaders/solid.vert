#version 450

layout(location = 0) out vec4 o_color;

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

    switch (corner) {
    case 0:
        gl_Position = vec4(-1, 1, 0, 1);
        o_color = vec4(0, 0, 0, 1);
        break;
    case 1:
        gl_Position = vec4(1, 1, 0, 1);
        o_color = vec4(1, 0, 0, 1);
        break;
    case 2:
        gl_Position = vec4(1, -1, 0, 1);
        o_color = vec4(0, 1, 0, 1);
        break;
    case 3:
        gl_Position = vec4(-1, -1, 0, 1);
        o_color = vec4(0, 0, 1, 1);
        break;
    }
}
