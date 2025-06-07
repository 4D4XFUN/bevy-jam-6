// film_grain.wgsl
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> settings: FilmGrainSettings;

struct FilmGrainSettings {
    grain_intensity: f32,
    grain_size: f32,
    grain_speed: f32,
    tint_intensity: f32,
    vignette_intensity: f32,
    vignette_radius: f32,
    time: f32,
    _padding: f32,
}

// Simple pseudo-random function
fn random(co: vec2<f32>) -> f32 {
    return fract(sin(dot(co, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

// Film grain noise
fn film_grain(uv: vec2<f32>, time: f32) -> f32 {
    let noise_uv = uv * settings.grain_size + vec2<f32>(time * settings.grain_speed);
    return random(floor(noise_uv));
}

// Vignette effect
fn vignette(uv: vec2<f32>) -> f32 {
    let center = vec2<f32>(0.5, 0.5);
    let dist = distance(uv, center);
    return smoothstep(settings.vignette_radius, settings.vignette_radius - 0.2, dist);
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    var color = textureSample(screen_texture, texture_sampler, uv);

    // Apply film grain
    let grain = film_grain(uv, settings.time);
    let grain_effect = mix(1.0, grain, settings.grain_intensity);
    color = color * grain_effect;

    // Apply yellow/sepia tint
    let sepia = vec3<f32>(
        dot(color.rgb, vec3<f32>(0.393, 0.769, 0.189)),
        dot(color.rgb, vec3<f32>(0.349, 0.686, 0.168)),
        dot(color.rgb, vec3<f32>(0.272, 0.534, 0.131))
    );
    color = vec4<f32>(mix(color.rgb, sepia, settings.tint_intensity), color.a);

    // Apply vignette
    let vignette_factor = mix(1.0, vignette(uv), settings.vignette_intensity);
    color = color * vignette_factor;

    return color;
}