/*
Copyright (c) 2019 Steven Wittens

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/

struct Uniforms {
    u_Matrix: mat4x4<f32>,
};

struct VertexInput {
    @location(0) a_Pos: vec2<f32>,
    @location(1) a_UV: vec2<f32>,
    @location(2) a_Color: vec4<f32>,
};

struct VertexOutput {
    @location(0) v_UV: vec2<f32>,
    @location(1) v_Color: vec4<f32>,
    @builtin(position) v_Position: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.v_UV = in.a_UV;
    out.v_Color = in.a_Color;
    out.v_Position = uniforms.u_Matrix * vec4<f32>(in.a_Pos.xy, 0.0, 1.0);
    return out;
}

struct FragmentOutput {
    @location(0) o_Target: vec4<f32>,
};

@group(1) @binding(0)
var u_Texture: texture_2d<f32>;
@group(1) @binding(1)
var u_Sampler: sampler;

fn srgb_to_linear(srgb: vec4<f32>) -> vec4<f32> {
    let color_srgb = srgb.rgb;
    let selector = ceil(color_srgb - 0.04045); // 0 if under value, 1 if over
    let under = color_srgb / 12.92;
    let over = pow((color_srgb + 0.055) / 1.055, vec3<f32>(2.4));
    let result = mix(under, over, selector);
    return vec4<f32>(result, srgb.a);
}

@fragment
fn fs_main_linear(in: VertexOutput) -> FragmentOutput {
    let color = srgb_to_linear(in.v_Color);

    return FragmentOutput(color * textureSample(u_Texture, u_Sampler, in.v_UV));
}

@fragment
fn fs_main_srgb(in: VertexOutput) -> FragmentOutput {
    let color = in.v_Color;

    return FragmentOutput(color * textureSample(u_Texture, u_Sampler, in.v_UV));
}