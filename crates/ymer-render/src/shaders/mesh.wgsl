struct CameraUniform {
    view_proj: mat4x4<f32>,
    // xyz = riktning ljuset färdas i, w oanvänd (padding till 16 byte)
    light_dir: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(7) uv: vec2<f32>,
};

@group(1) @binding(0) var base_texture: texture_2d<f32>;
@group(1) @binding(1) var base_sampler: sampler;

// Per instans: modellmatrisen som fyra kolumner plus färg.
struct InstanceInput {
    @location(2) model_0: vec4<f32>,
    @location(3) model_1: vec4<f32>,
    @location(4) model_2: vec4<f32>,
    @location(5) model_3: vec4<f32>,
    @location(6) color: vec4<f32>,
    // [offset_x, offset_y, scale_x, scale_y] – väljer ut en spritesheet-ruta.
    @location(8) uv_transform: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vs_main(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    let model = mat4x4<f32>(
        instance.model_0,
        instance.model_1,
        instance.model_2,
        instance.model_3,
    );

    let world_position = model * vec4<f32>(vertex.position, 1.0);

    // Antar uniform skalning – annars behövs den inversa transponatet.
    let normal_matrix = mat3x3<f32>(model[0].xyz, model[1].xyz, model[2].xyz);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * world_position;
    out.world_normal = normalize(normal_matrix * vertex.normal);
    out.color = instance.color;
    out.uv = vertex.uv * instance.uv_transform.zw + instance.uv_transform.xy;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.world_normal);
    let to_light = normalize(-camera.light_dir.xyz);

    let diffuse = max(dot(normal, to_light), 0.0);
    // Billig "himmelsljus"-term så att skuggsidor inte blir helt döda.
    let sky = 0.5 + 0.5 * normal.y;
    let light = 0.18 * sky + 0.82 * diffuse;

    // Otexturerade objekt binder en vit 1x1-pixel, så samma rad duger åt båda.
    let base = textureSample(base_texture, base_sampler, in.uv);
    let albedo = in.color * base;

    return vec4<f32>(albedo.rgb * light, albedo.a);
}
