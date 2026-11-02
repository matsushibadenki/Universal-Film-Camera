struct Params {
    extent: vec4<f32>, // target w/h, source w/h
    color: vec4<u32>, // 0=BGRA, 1=709 YUV, 2=601 YUV; full; 709 transfer; left chroma
    domain_min: vec4<f32>,
    domain_scale: vec4<f32>,
    cube: vec4<u32>,
}
@group(0) @binding(0) var y_image: texture_2d<f32>;
@group(0) @binding(1) var uv_image: texture_2d<f32>;
@group(0) @binding(2) var image_sampler: sampler;
@group(0) @binding(3) var color_cube: texture_3d<f32>;
@group(0) @binding(4) var<uniform> params: Params;

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
@fragment fn fragment_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let source_aspect = params.extent.z / params.extent.w;
    let target_aspect = params.extent.x / params.extent.y;
    let crop = vec2(min(1.0,target_aspect/source_aspect), min(1.0,source_aspect/target_aspect));
    let uv = (position.xy / params.extent.xy - vec2(0.5)) * crop + vec2(0.5);
    var rgb = textureSample(y_image, image_sampler, uv).rgb;
    if params.color.x != 0u {
        let offset = vec2(select(0.0, 0.5 / params.extent.z, params.color.w == 1u), 0.0);
        var y = rgb.r;
        var cbcr = textureSample(uv_image, image_sampler, uv + offset).rg - vec2(128.0 / 255.0);
        if params.color.y == 0u { y = (y - 16.0 / 255.0) * (255.0/219.0); cbcr *= 255.0/224.0; }
        let c = select(vec4(1.5748,-0.187324,-0.468124,1.8556), vec4(1.402,-0.344136,-0.714136,1.772), params.color.x == 2u);
        rgb = vec3(y+c.x*cbcr.y, y+c.y*cbcr.x+c.z*cbcr.y, y+c.w*cbcr.x);
    }
    rgb = clamp(rgb, vec3(0.0), vec3(1.0));
    if params.color.z == 1u {
        let linear = select(pow((rgb + vec3(0.099)) / 1.099, vec3(1.0/0.45)), rgb / 4.5, rgb < vec3(0.081));
        rgb = select(1.055 * pow(linear, vec3(1.0/2.4)) - vec3(0.055), 12.92 * linear, linear <= vec3(0.0031308));
    }
    return vec4(clamp(cube_sample(rgb),vec3(0.0),vec3(1.0)),1.0);
}
