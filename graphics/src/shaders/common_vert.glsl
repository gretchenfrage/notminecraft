// common code for object vertex shaders

#include "common.glsl"

// ==== inputs and outputs ====

#define COMN_INS 0

#define COMN_OUTS 1

layout(location=0) out vec4 o_pos;


// ==== functions ====

// set o_pos based on the given object-space pos. user should then set
// `gl_Position = o_pos;`.
void set_pos(vec3 obj_pos) {
    o_pos = u_transform * vec4(obj_pos, 1);
}

// get position for hard-coded unit squares based on gl_VertexIndex.
vec3 unit_square_pos() {
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

    return vec3(pos, 0);
}
