const QUAD_VERTICES: array<vec3<f32>, 4> = array<vec3<f32>, 4>(
    vec3<f32>(0.0, 0.0, 0.0),
    vec3<f32>(1.0, 0.0, 0.0),
    vec3<f32>(0.0, 1.0, 0.0),
    vec3<f32>(1.0, 1.0, 0.0),
);

struct Vertex {
    position: u32,
    block_id: u32,
}

struct GlobalUniforms {
    atlas_size: u32,
    time: f32,
};

@group(0) @binding(0) var<storage, read> chunk_SSBO: array<u32>;

@group(1) @binding(0) var t_diffuse: texture_2d<f32>;
@group(1) @binding(1) var s_diffuse: sampler;

@group(2) @binding(0)
var<uniform> globals: GlobalUniforms;

@group(3) @binding(0)
var<storage, read> texture_mappings: array<u32>;

struct PushConstants {
    pv: mat4x4<f32>,
    app_render_config: u32,
    ssbo_offset: u32,
    byte_count: u32,
    chunk_pos: vec3<i32>,
}

var<push_constant> push: PushConstants;

struct VertexInput {
    @builtin(vertex_index) vertex_id: u32,
    @builtin(instance_index) instance_id: u32,
};
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) atlas_offset: vec2<f32>,
    @location(2) debug_color: vec3<f32>,
    @location(3) brightness: f32,
};

const FACE_TRANSFORMS: array<mat4x4<f32>, 6> = array<mat4x4<f32>, 6>(
    // 0: +X (Right)
    mat4x4<f32>(
        vec4<f32>(0.0, 0.0, -1.0, 0.0),
        vec4<f32>(0.0, -1.0, 0.0, 0.0),
        vec4<f32>(-1.0, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 1.0, 1.0, 1.0)
    ),
    // 1: -X (Left)
    mat4x4<f32>(
        vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(0.0, -1.0, 0.0, 0.0),
        vec4<f32>(-1.0, 0.0, 0.0, 0.0),
        vec4<f32>(1.0, 1.0, 0.0, 1.0)
    ),
    // 2: +Y (Top)
    mat4x4<f32>(
        vec4<f32>(0.0, 0.0, -1.0, 0.0),
        vec4<f32>(-1.0, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, -1.0, 0.0, 0.0),
        vec4<f32>(1.0, 1.0, 1.0, 1.0)
    ),
    // 3: -Y (Bottom)
    mat4x4<f32>(
        vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(-1.0, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, -1.0, 0.0, 0.0),
        vec4<f32>(1.0, 0.0, 0.0, 1.0)
    ),
    // 4: +Z (Front)
    mat4x4<f32>(
        vec4<f32>(-1.0, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, -1.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, -1.0, 0.0),
        vec4<f32>(1.0, 1.0, 1.0, 1.0)
    ),
    // 5: -Z (Back)
    mat4x4<f32>(
        vec4<f32>(1.0, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, -1.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(0.0, 1.0, 0.0, 1.0)
    ),
);

@vertex
fn vertex_main(in: VertexInput) -> VertexOutput {
    let proper_offset = push.ssbo_offset + (in.instance_id * 2);
    let pos = chunk_SSBO[proper_offset];
    let id = chunk_SSBO[proper_offset + 1];

    var block_id = (id >> 0u) & 65535;
    let face_normal = (id >> 16u) & 7;

    // 0b11111 = 31
    var x = pos & 31;
    var y = (pos >> 5u) & 31;
    var z = (pos >> 10u) & 31;
    let h = f32(((pos >> 15u) & 31) + 1u);
    let w = f32(((pos >> 20u) & 31) + 1u);

    var transparency: f32 = 1.0;
    if block_id == 0 {
        transparency = 0.5;
    }

    let render_textures = bool(push.app_render_config & 1);

    var out: VertexOutput;
    var quad_pos: vec3<f32>;
    var local_uv = vec2<f32>(0.0, 0.0);
    out.debug_color = vec3<f32>(0.0, 0.0, 0.0);
    switch in.vertex_id {
        case 0u: {
            out.debug_color = vec3<f32>(1.0, 0.0, 0.0);
            quad_pos = QUAD_VERTICES[0];
        }
        case 1u: {
            out.debug_color = vec3<f32>(0.0, 1.0, 0.0);
            quad_pos = QUAD_VERTICES[1];
            local_uv = vec2<f32>(h, 0.0);
        }
        case 2u: {
            out.debug_color = vec3<f32>(0.0, 0.0, 1.0);
            quad_pos = QUAD_VERTICES[2];
            local_uv = vec2<f32>(0.0, w);
        }
        case 3u: {
            out.debug_color = vec3<f32>(0.0, 0.0, 0.0);
            quad_pos = QUAD_VERTICES[3];
            local_uv = vec2<f32>(h, w);
        }
        default: {
            quad_pos = vec3<f32>(0.0, 0.0, 0.0);
        }
    }

    switch face_normal {
        case 0u: {
            out.brightness = 0.6;
        }
        case 1u: {
            out.brightness = 0.6;
        }
        case 2u: {
            out.brightness = 1.0;
        }
        case 3u: {
            out.brightness = 0.5;
        }
        case 4u: {
            out.brightness = 0.8;
        }
        default: {
            out.brightness = 0.8;
        }
    }

    let instance_local_pos = vec4<f32>(f32(x), f32(y), f32(z), 0.0);

    let multiplied_chunk_pos = push.chunk_pos * 32;
    let final_chunk_pos = vec4<f32>(f32(multiplied_chunk_pos.x), f32(multiplied_chunk_pos.y), f32(multiplied_chunk_pos.z), 0);

    let face_transform = FACE_TRANSFORMS[face_normal];

    var stretched_quad_pos = quad_pos;
    if face_normal == 0 || face_normal == 2 || face_normal == 4 {
        stretched_quad_pos.x = (quad_pos.x * h) - (h - 1.0);
    } else {
        stretched_quad_pos.x = quad_pos.x * h;
    }

    stretched_quad_pos.y = (quad_pos.y * w) - (w - 1.0);

    let atlas_dim: f32 = f32(globals.atlas_size) / 16.0;
    let inv_atlas_dim: f32 = 1.0 / atlas_dim;

    let atlas_index = texture_mappings[((block_id) * 6u) + face_normal];

    let u_idx = f32((atlas_index) % u32(atlas_dim));
    let v_idx = f32((atlas_index) / u32(atlas_dim));
    let base_uv = vec2<f32>(u_idx, v_idx) * inv_atlas_dim;

    out.uv = local_uv * inv_atlas_dim;
    out.atlas_offset = base_uv;
    out.position = push.pv * ((instance_local_pos + face_transform * vec4<f32>(stretched_quad_pos, 1.0)) + final_chunk_pos);
    return out;
};

@fragment
fn fragment_main(in: VertexOutput) -> @location(0) vec4<f32> {
let render_textures = bool(push.app_render_config & 1);
    var color: vec4<f32>;
    let atlas_dim: f32 = f32(globals.atlas_size) / 16.0;

    if !render_textures {
        color = vec4<f32>(in.debug_color, 1.0);
    } else {
        let atlas_tile_size = 1.0 / atlas_dim;
        let ddx = dpdx(in.uv);
        let ddy = dpdy(in.uv);
        let local_uv = in.uv % atlas_tile_size;
        let normalized_uv = local_uv / atlas_tile_size;
        let shrink = 0.0005;
        let inset_uv = normalized_uv * (1.0 - 2.0 * shrink) + shrink;
        let final_uv = (inset_uv * atlas_tile_size) + in.atlas_offset;

        color = textureSampleGrad(t_diffuse, s_diffuse, final_uv, ddx, ddy);
    }

    let b = in.brightness;
    return color * vec4<f32>(b, b, b, 1.0);
}
