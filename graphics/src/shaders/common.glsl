// common code for all object shaders

// ==== inputs and outputs ====

#define COMN_UNIS 3

layout(set=0, binding=0) uniform u {
    mat4 u_transform;
    vec4 u_color;
    mat4 u_screen_to_world;
    float u_fog_mul;
    float u_fog_add;
    float u_day_night_time;
};
