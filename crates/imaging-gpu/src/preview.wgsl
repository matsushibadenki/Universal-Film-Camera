struct Params {
    extent: vec4<f32>, // target w/h, source w/h
    color: vec4<u32>, // 0=BGRA, 1=709 YUV, 2=601 YUV; full; 709 transfer; left chroma
    domain_min: vec4<f32>,
    domain_scale: vec4<f32>,
    cube: vec4<u32>,
    peaking: vec4<f32>, // RGB; enabled
}
@group(0) @binding(0) var y_image: texture_2d<f32>;
@group(0) @binding(1) var uv_image: texture_2d<f32>;
@group(0) @binding(2) var image_sampler: sampler;
@group(0) @binding(3) var color_cube: texture_3d<f32>;
@group(0) @binding(4) var<uniform> params: Params;
@group(0) @binding(5) var<storage, read_write> histogram: array<atomic<u32>, 96>;

@vertex fn vertex_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let p = array(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    return vec4(p[index], 0.0, 1.0);
}
fn cube_sample(p: vec3<f32>) -> vec3<f32> {
    let cell = clamp((p - params.domain_min.xyz) * params.domain_scale.xyz, vec3(0.0), vec3(1.0)) * f32(params.cube.x - 1u);
    let lo = vec3<i32>(floor(cell));
    let hi = min(lo + vec3(1), vec3<i32>(i32(params.cube.x) - 1));
    let t = fract(cell);
    let a = mix(textureLoad(color_cube, vec3(lo.x,lo.y,lo.z),0).rgb, textureLoad(color_cube,vec3(hi.x,lo.y,lo.z),0).rgb,t.x);
    let b = mix(textureLoad(color_cube, vec3(lo.x,hi.y,lo.z),0).rgb, textureLoad(color_cube,vec3(hi.x,hi.y,lo.z),0).rgb,t.x);
    let c = mix(textureLoad(color_cube, vec3(lo.x,lo.y,hi.z),0).rgb, textureLoad(color_cube,vec3(hi.x,lo.y,hi.z),0).rgb,t.x);
    let d = mix(textureLoad(color_cube, vec3(lo.x,hi.y,hi.z),0).rgb, textureLoad(color_cube,vec3(hi.x,hi.y,hi.z),0).rgb,t.x);
    return mix(mix(a,b,t.y), mix(c,d,t.y), t.z);
}
fn source_rgb(uv: vec2<f32>) -> vec3<f32> {
    var rgb = textureSampleLevel(y_image, image_sampler, uv, 0.0).rgb;
    if params.color.x != 0u {
        let offset = vec2(select(0.0, 0.5 / params.extent.z, params.color.w == 1u), 0.0);
        var y = rgb.r;
        var cbcr = textureSampleLevel(uv_image, image_sampler, uv + offset, 0.0).rg - vec2(128.0 / 255.0);
        if params.color.y == 0u { y = (y - 16.0 / 255.0) * (255.0/219.0); cbcr *= 255.0/224.0; }
        let c = select(vec4(1.5748,-0.187324,-0.468124,1.8556), vec4(1.402,-0.344136,-0.714136,1.772), params.color.x == 2u);
        rgb = vec3(y+c.x*cbcr.y, y+c.y*cbcr.x+c.z*cbcr.y, y+c.w*cbcr.x);
    }
    rgb = clamp(rgb, vec3(0.0), vec3(1.0));
    if params.color.z == 1u {
        let linear = select(pow((rgb + vec3(0.099)) / 1.099, vec3(1.0/0.45)), rgb / 4.5, rgb < vec3(0.081));
        rgb = select(1.055 * pow(linear, vec3(1.0/2.4)) - vec3(0.055), 12.92 * linear, linear <= vec3(0.0031308));
    }
    return rgb;
}
fn luma(uv: vec2<f32>) -> f32 {
    return dot(source_rgb(uv), vec3(54.0,183.0,19.0));
}
fn preview_uv(normalized: vec2<f32>) -> vec2<f32> {
    let source_aspect = params.extent.z / params.extent.w;
    let target_aspect = params.extent.x / params.extent.y;
    let crop = vec2(min(1.0,target_aspect/source_aspect), min(1.0,source_aspect/target_aspect));
    return (normalized - vec2(0.5)) * crop + vec2(0.5);
}
fn graded_rgb(uv: vec2<f32>) -> vec3<f32> {
    let rgb = source_rgb(uv);
    if params.cube.z == 1u { return clamp(cube_sample(rgb),vec3(0.0),vec3(1.0)); }
    return rgb;
}
// A bounded 128x72 grid over the visible crop. Scopes exclude display aids.
@compute @workgroup_size(8, 8)
fn histogram_main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= 128u || id.y >= 72u { return; }
    let uv = preview_uv((vec2<f32>(id.xy) + vec2(0.5)) / vec2(128.0,72.0));
    let bins = min(vec3<u32>(floor(graded_rgb(uv) * 32.0)), vec3(31u));
    atomicAdd(&histogram[bins.r], 1u);
    atomicAdd(&histogram[32u + bins.g], 1u);
    atomicAdd(&histogram[64u + bins.b], 1u);
}
@fragment fn fragment_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = preview_uv(position.xy / params.extent.xy);
    let rgb = source_rgb(uv);
    var output = rgb;
    if params.cube.z == 1u { output = clamp(cube_sample(rgb),vec3(0.0),vec3(1.0)); }
    // Use the same cropped source sample as the image. LUTs must not move the
    // exposure-warning threshold or create warnings on otherwise dark pixels.
    if params.cube.y == 1u && dot(rgb, vec3(0.2126, 0.7152, 0.0722)) >= 0.9216 {
        let phase = (u32(floor(position.x)) + u32(floor(position.y))) % 12u;
        let stripe = select(0.047, 1.0, phase < 6u);
        output = mix(output, vec3(stripe), 0.745);
    }
    if params.peaking.w == 1.0 {
        // One source texel, not an independently scaled overlay. Clamp-to-edge
        // sampling avoids wraparound warnings at image borders.
        let d = vec2(1.0) / params.extent.zw;
        let tl = luma(uv + d * vec2(-1.0,-1.0));
        let tc = luma(uv + d * vec2(0.0,-1.0));
        let tr = luma(uv + d * vec2(1.0,-1.0));
        let ml = luma(uv + d * vec2(-1.0,0.0));
        let mr = luma(uv + d * vec2(1.0,0.0));
        let bl = luma(uv + d * vec2(-1.0,1.0));
        let bc = luma(uv + d * vec2(0.0,1.0));
        let br = luma(uv + d * vec2(1.0,1.0));
        let gradient = vec2(-tl+tr-2.0*ml+2.0*mr-bl+br, -tl-2.0*tc-tr+bl+2.0*bc+br);
        if length(gradient) > 180.0 { output = params.peaking.rgb; }
    }
    return vec4(output,1.0);
}
