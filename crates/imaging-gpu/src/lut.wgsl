struct Parameters {
    extent: vec4<u32>, // width, height, cube size, reserved
    domain_min: vec4<f32>,
    domain_scale: vec4<f32>,
}
@group(0) @binding(0) var input_image: texture_2d<f32>;
@group(0) @binding(1) var output_image: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var color_cube: texture_3d<f32>;
@group(0) @binding(3) var<uniform> params: Parameters;

fn sample_cube(p: vec3<f32>) -> vec3<f32> {
    let cell = clamp((p - params.domain_min.xyz) * params.domain_scale.xyz,
                     vec3(0.0), vec3(1.0)) * f32(params.extent.z - 1u);
    let lo = vec3<i32>(floor(cell));
    let hi = min(lo + vec3(1), vec3<i32>(i32(params.extent.z) - 1));
    let t = fract(cell);
    let a = mix(textureLoad(color_cube, vec3(lo.x, lo.y, lo.z), 0).rgb,
                textureLoad(color_cube, vec3(hi.x, lo.y, lo.z), 0).rgb, t.x);
    let b = mix(textureLoad(color_cube, vec3(lo.x, hi.y, lo.z), 0).rgb,
                textureLoad(color_cube, vec3(hi.x, hi.y, lo.z), 0).rgb, t.x);
    let c = mix(textureLoad(color_cube, vec3(lo.x, lo.y, hi.z), 0).rgb,
                textureLoad(color_cube, vec3(hi.x, lo.y, hi.z), 0).rgb, t.x);
    let d = mix(textureLoad(color_cube, vec3(lo.x, hi.y, hi.z), 0).rgb,
                textureLoad(color_cube, vec3(hi.x, hi.y, hi.z), 0).rgb, t.x);
    return mix(mix(a, b, t.y), mix(c, d, t.y), t.z);
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= params.extent.x || id.y >= params.extent.y { return; }
    let position = vec2<i32>(id.xy);
    let source = textureLoad(input_image, position, 0);
    // Core Graphics adapters supply premultiplied sRGB. Keep alpha independent
    // of the color transform, including fully transparent pixels.
    let straight = source.rgb / max(source.a, 1.0 / 255.0);
    let result = clamp(sample_cube(straight), vec3(0.0), vec3(1.0));
    textureStore(output_image, position, vec4(result * source.a, source.a));
}
