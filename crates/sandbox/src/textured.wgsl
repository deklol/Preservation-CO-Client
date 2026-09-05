// Source: @digitalm1nd on x.com / _dek on Discord — Preservation Conquer project — https://discord.gg/CvKPXEHYRY
struct VertexOutput { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> }
@group(0) @binding(0) var image: texture_2d<f32>;
@group(0) @binding(1) var image_sampler: sampler;
@vertex fn vertex_main(@location(0) p: vec3<f32>, @location(1) uv: vec2<f32>) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(p, 1.0);
    output.uv = uv;
    return output;
}
@fragment fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let colour = textureSample(image, image_sampler, input.uv);
    if colour.a < 0.01 { discard; }
    return colour;
}
