#version 450

layout(set=0, binding=0) uniform u {
    mat4 u_transform;
    vec4 u_color;
    mat4 u_screen_to_world;
};

layout(set=1, binding=0) uniform texture2D u_clip_min_texture;
layout(set=1, binding=1) uniform sampler u_clip_min_sampler;

layout(set=2, binding=0) uniform texture2D u_clip_max_texture;
layout(set=2, binding=1) uniform sampler u_clip_max_sampler; // TODO: dedupe samplers?

layout(location=0) in vec4 i_pos;

layout(location=0) out vec4 o_color;


#define PI 3.1415926535897932384626433832795


// baseline sky color at no-rain daytime
const vec3 SKY_DAY = vec3(0.45, 0.62, 1.00);
// baseline sky color at no-rain nighttime
const vec3 SKY_NIGHT = vec3(0.00, 0.02, 0.05);
// baseline sky color at rainy daytime
const vec3 SKY_DAY_RAIN = vec3(0.24, 0.26, 0.32);
// baseline sky color at rainy nighttime
const vec3 SKY_NIGHT_RAIN = vec3(0.00, 0.01, 0.01);
// baseline fog color at no-rain daytime
const vec3 FOG_DAY = vec3(0.70, 0.82, 1.00);
// baseline fog color at no-rain nighttime
const vec3 FOG_NIGHT = vec3(0.02, 0.05, 0.13);
// baseline fog color at rainy daytime
const vec3 FOG_DAY_RAIN = vec3(0.48, 0.52, 0.60);
// baseline fog color at rainy nighttime
const vec3 FOG_NIGHT_RAIN = vec3(0.02, 0.04, 0.07);
// baseline color of sunset fog (fog with sun behind it during sunset)
const vec3 SKY_SUNSET = vec3(1.00, 0.35, 0.10);


float sq(float n) {
    return n * n;
}


void main() {
    // inputs
    vec4 player_pos = u_screen_to_world * vec4(0, 0, 0, 1);
    float time = player_pos.z / player_pos.w / 50;
    vec3 sun_dir = vec3(0, sin(time * PI * 2), cos(time * PI * 2));
    float rain = 0;

    // compute view dir
    vec4 a = u_screen_to_world * vec4(i_pos.xy, 1, i_pos.w);
    vec4 b = u_screen_to_world * vec4(i_pos.xy, 0, i_pos.w);
    vec3 view_dir = normalize((a.xyz / a.w) - (b.xyz / b.w));

    // compute sky color

    // intensity of it being day as opposed to night
    float day = clamp(sin(time * PI * 2) + 0.6, 0, 1);

    // intensity of the sunset being actively happening
    float sunset = pow(cos(time * PI * 4) * 0.5 + 0.5, 25);
    
    // intensity of this fragment being in the direction of the sun
    float sun = sq(dot(view_dir, sun_dir) * 0.5 + 0.5);

    // intensity of this fragment being in the direction of fog
    // we make it intensify when in the direction of the sun
    float fragment_fog = smoothstep(-sun * 0.5 - 0.1, 0.05, dot(view_dir, vec3(0, -1, 0)));

    // intensity of this fragment's fog being purely sunset-colored
    float fragment_sunset = (sun * 10 / (0.45 + sun * 9)) * sunset;

    // then it's mixing them together
    vec3 sky_color = mix(
        // sky color:
        mix(
            // no-rain sky color:
            mix(SKY_NIGHT, SKY_DAY, day),
            // rain sky color:
            mix(SKY_NIGHT_RAIN, SKY_DAY_RAIN, day),
            rain
        ),
        // sunset-altered fog color:
        mix(
            // baseline fog color:
            mix(
                // no-rain fog color:
                mix(FOG_NIGHT, FOG_DAY, day),
                // rain fog color:
                mix(FOG_NIGHT_RAIN, FOG_DAY_RAIN, day),
                rain
            ),
            SKY_SUNSET,
            clamp(fragment_sunset, 0, 1)
        ),
        clamp(fragment_fog, 0, 1)
    );
    
    // output
    o_color = vec4(sky_color, 1) * u_color;

    // debug sun
    if (dot(sun_dir, view_dir) > 0.99) {
        o_color = vec4(1, 1, 0, 1);
    }

    // clipping
    vec4 pos = i_pos / i_pos.w;
    vec2 clip_uv = vec2(
        pos.x / 2 + 0.5,
        pos.y / -2 + 0.5
    );
    float min_z = texture(sampler2D(u_clip_min_texture, u_clip_min_sampler), clip_uv).r;
    float max_z = texture(sampler2D(u_clip_max_texture, u_clip_max_sampler), clip_uv).r;
    if (pos.z < min_z) {
        discard;
    }
    if (pos.z > max_z) {
        discard;
    }
}
