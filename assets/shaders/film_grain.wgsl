// film_grain.wgsl
#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var texture_sampler: sampler;
@group(0) @binding(2) var<uniform> settings: FilmGrainSettings;

struct FilmGrainSettings {
    grain_intensity: f32,
    grain_scale: f32,
    grain_speed: f32,
    tint_intensity: f32,
    vignette_intensity: f32,
    vignette_radius: f32,
    time: f32,
    artifact_intensity: f32,
    scratch_frequency: f32,
    dust_frequency: f32,
    hair_frequency: f32,
    _padding: f32,
}

// Simple pseudo-random function
fn random(co: vec2<f32>) -> f32 {
    return fract(sin(dot(co, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

// Hash function for better randomness
fn hash(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.xyx) * 0.13);
    p3 += dot(p3, p3.yzx + 3.333);
    return fract((p3.x + p3.y) * p3.z);
}

// Film grain noise
fn film_grain(uv: vec2<f32>, time: f32) -> f32 {
    let noise_uv = uv / settings.grain_scale + vec2<f32>(time * settings.grain_speed);
    return random(floor(noise_uv));
}

// Vertical scratch/hair effect
fn vertical_scratch(uv: vec2<f32>, time: f32) -> f32 {
    // Random vertical position that changes occasionally
    let scratch_time = floor(time * 3.0); // Change position ~3 times per second
    let x_pos = hash(vec2<f32>(scratch_time, 0.0));

    // Thin vertical line with some wobble
    let wobble = sin(uv.y * 40.0 + time * 10.0) * 0.001;
    let dist = abs(uv.x - x_pos + wobble);

    // Make it appear/disappear based on frequency setting
    let appear = step(1.0 - settings.scratch_frequency, hash(vec2<f32>(scratch_time, 1.0)));

    return appear * smoothstep(0.002, 0.0, dist);
}

// Dust speck effect
fn dust_speck(uv: vec2<f32>, time: f32) -> f32 {
    let grid_size = 8.0;
    let cell = floor(uv * grid_size);
    let cell_time = floor(time * 24.0); // Change 24 times per second (film framerate!)

    // Random chance for dust in each cell based on frequency setting
    let dust_chance = hash(cell + cell_time * 17.0);
    if (dust_chance > (1.0 - settings.dust_frequency)) {
        let local_uv = fract(uv * grid_size);
        let center = vec2<f32>(
            hash(cell + cell_time * 23.0),
            hash(cell + cell_time * 31.0)
        );
        let dist = distance(local_uv, center);
        return smoothstep(0.1, 0.0, dist);
    }
    return 0.0;
}

// Hair/fiber effect (diagonal lines)
fn hair_fiber(uv: vec2<f32>, time: f32) -> f32 {
    let hair_time = floor(time * 2.0); // Change less frequently
    let start_pos = vec2<f32>(
        hash(vec2<f32>(hair_time, 2.0)),
        hash(vec2<f32>(hair_time, 3.0))
    );

    // Diagonal line
    let dir = normalize(vec2<f32>(0.7, 1.0));
    let projected = dot(uv - start_pos, dir);
    let closest_point = start_pos + dir * clamp(projected, 0.0, 0.3); // Limited length
    let dist = distance(uv, closest_point);

    // Make it appear based on frequency setting
    let appear = step(1.0 - settings.hair_frequency, hash(vec2<f32>(hair_time, 4.0)));

    return appear * smoothstep(0.003, 0.0, dist) * 0.7;
}

// Vignette effect
fn vignette(uv: vec2<f32>) -> f32 {
    let center = vec2<f32>(0.5, 0.5);
    let dist = distance(uv, center);
    return smoothstep(settings.vignette_radius, settings.vignette_radius - 0.2, dist);
}

// Simulate 24fps stutter (optional - subtle effect)
fn framerate_stutter(color: vec3<f32>, time: f32) -> vec3<f32> {
    let frame_time = floor(time * 24.0) / 24.0; // Quantize to 24fps
    let stutter = smoothstep(0.0, 1.0 / 24.0, time - frame_time);
    return mix(color * 0.95, color, stutter); // Slight brightness fluctuation
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    var color = textureSample(screen_texture, texture_sampler, uv);

    // Apply film grain
    let grain = film_grain(uv, settings.time);
    let grain_effect = mix(1.0, grain, settings.grain_intensity);
    color = color * grain_effect;

    // Apply artifacts (dust, scratches, hairs)
    if (settings.artifact_intensity > 0.0) {
        let scratch = vertical_scratch(uv, settings.time);
        let dust = dust_speck(uv, settings.time);
        let hair = hair_fiber(uv, settings.time);

        let artifacts = max(max(scratch, dust), hair);
        color = mix(color, vec4<f32>(0.1, 0.1, 0.1, 1.0), artifacts * settings.artifact_intensity);
    }

    // Apply yellow/sepia tint
    let sepia = vec3<f32>(
        dot(color.rgb, vec3<f32>(0.393, 0.769, 0.189)),
        dot(color.rgb, vec3<f32>(0.349, 0.686, 0.168)),
        dot(color.rgb, vec3<f32>(0.272, 0.534, 0.131))
    );
    color = vec4<f32>(mix(color.rgb, sepia, settings.tint_intensity), color.a);

    // Optional: Apply 24fps stutter effect (subtle)
    // color.rgb = framerate_stutter(color.rgb, settings.time);

    // Apply vignette
    let vignette_factor = mix(1.0, vignette(uv), settings.vignette_intensity);
    color = color * vignette_factor;

    return color;
}
