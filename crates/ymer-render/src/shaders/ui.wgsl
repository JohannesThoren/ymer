// Gränssnitt i skärmrymd.
//
// Egen shader, inte mesh-shadern, av två skäl. Ljussättningen där skulle
// skugga en HUD efter en ljusriktning i världen, vilket är meningslöst
// för något som ligger platt på skärmen. Och kameran här är en
// ortografisk pixelmatris, inte scenens – en knapp ska ligga still när
// spelaren vrider sig.

struct ScreenUniform {
    // Pixlar in, klippkoordinater ut.
    projection: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> screen: ScreenUniform;

@group(1) @binding(0) var base_texture: texture_2d<f32>;
@group(1) @binding(1) var base_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(7) uv: vec2<f32>,
};

// Samma instansformat som resten av motorn, så att ritlistan kan packas
// i en enda buffert.
struct InstanceInput {
    @location(2) model_0: vec4<f32>,
    @location(3) model_1: vec4<f32>,
    @location(4) model_2: vec4<f32>,
    @location(5) model_3: vec4<f32>,
    @location(6) color: vec4<f32>,
    // [offset_x, offset_y, scale_x, scale_y] – väljer ut en glyf ur
    // atlasen, eller hela texturen för en fylld yta.
    @location(8) uv_transform: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
};

@vertex
fn vs_main(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    let model = mat4x4<f32>(
        instance.model_0,
        instance.model_1,
        instance.model_2,
        instance.model_3,
    );

    var out: VertexOutput;
    out.clip_position = screen.projection * model * vec4<f32>(vertex.position, 1.0);
    out.color = instance.color;
    out.uv = vertex.uv * instance.uv_transform.zw + instance.uv_transform.xy;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Ingen ljussättning: färgen är den som begärdes. Fyllda ytor binder
    // den vita pixeln, så samma rad duger för både text och rektanglar.
    let sampled = textureSample(base_texture, base_sampler, in.uv);
    return in.color * sampled;
}
