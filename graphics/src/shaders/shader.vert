#version 450

void main() {
    int x = gl_VertexIndex - 1;
    int y = (gl_VertexIndex & 1) * 2 - 1;
    gl_Position = vec4(x, y, 0, 1);
}
