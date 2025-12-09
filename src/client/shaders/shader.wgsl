const QUAD_VERTICES: array<vec3<f32>, 4> = array<vec3<f32>, 4>(
    vec3<f32>(0.0, 0.0, 0.0),
    vec3<f32>(1.0, 0.0, 0.0),
    vec3<f32>(0.0, 1.0, 0.0),
    vec3<f32>(1.0, 1.0, 0.0),
);

struct Vertex {
    position: u32,
    id: u32,
}

@group(0) @binding(0) var<storage, read> chunk_SSBO: array<u32>;

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
    @location(0) color: vec4<f32>,
};

const FACE_TRANSFORMS: array<mat4x4<f32>, 6> = array<mat4x4<f32>, 6>(
    // 0: +X (Right)
    mat4x4<f32>(
        vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(0.0, 1.0, 0.0, 0.0),
        vec4<f32>(1.0, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0)
    ),
    // 1: -X (Left)
    mat4x4<f32>(
        vec4<f32>(0.0, 0.0, -1.0, 0.0),
        vec4<f32>(0.0, 1.0, 0.0, 0.0),
        vec4<f32>(1.0, 0.0, 0.0, 0.0),
        vec4<f32>(1.0, 0.0, 1.0, 1.0)
    ),
    // 2: +Y (Top)
    mat4x4<f32>(
        vec4<f32>(-1.0, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(0.0, -1.0, 0.0, 0.0),
        vec4<f32>(1.0, 1.0, 0.0, 1.0)
    ),
    // 3: -Y (Bottom)
    mat4x4<f32>(
        vec4<f32>(1.0, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(0.0, -1.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0)
    ),
    // 4: +Z (Front)
    mat4x4<f32>(
        vec4<f32>(1.0, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 1.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0, 0.0),
        vec4<f32>(0.0, 0.0, 1.0, 1.0)
    ),
    // 5: -Z (Back)
    mat4x4<f32>(
        vec4<f32>(-1.0, 0.0, 0.0, 0.0),
        vec4<f32>(0.0, 1.0, 0.0, 0.0),
        vec4<f32>(0.0, 0.0, -1.0, 0.0),
        vec4<f32>(1.0, 0.0, 0.0, 1.0)
    ),
);

@vertex
fn vertex_main(in: VertexInput) -> VertexOutput {
    let proper_offset = push.ssbo_offset + (in.instance_id * 2);
    let pos = chunk_SSBO[proper_offset];
    let id = chunk_SSBO[proper_offset + 1];

    let block_id = (id >> 0u) & 65535;
    let face_normal = (id >> 16u) & 7;

    // 0b111111 = 63
    var x = pos & 63;
    var y = (pos >> 6u) & 63;
    var z = (pos >> 12u) & 63;
    let h = ((pos >> 18u) & 63) + 1u;
    let w = ((pos >> 24u) & 63) + 1u;

    var transparency: f32 = 1.0;
    if block_id == 0 {
        transparency = 0.5;
    }

    let render_textures = bool(push.app_render_config & 1);

    var out: VertexOutput;
    var quad_pos: vec3<f32>;
    switch in.vertex_id {
        case 0u: {
            out.color = vec4<f32>(1.0, 0.0, 0.0, transparency);
            quad_pos = QUAD_VERTICES[0];
        }
        case 1u: { 
            out.color = vec4<f32>(0.0, 1.0, 0.0, transparency);
            quad_pos = QUAD_VERTICES[1];
        }
        case 2u: {
            out.color = vec4<f32>(0.0, 0.0, 1.0, transparency);
            quad_pos = QUAD_VERTICES[2];
        }
        case 3u: { 
            out.color = vec4<f32>(0.0, 0.0, 0.0, transparency);
            quad_pos = QUAD_VERTICES[3];
        }
        default: {
            out.color = vec4<f32>(1.0, 1.0, 1.0, 1.0); 
            quad_pos = vec3<f32>(0.0, 0.0, 0.0);
        }
    }

    if render_textures {
        switch block_id {
            case 1u: { // stone
                out.color = vec4<f32>(0.32, 0.32, 0.32, 1.0);
            }
            case 2u: { // dirt
                out.color = vec4<f32>(0.36, 0.27, 0.17, 1.0);
            }
            case 3u: { // grass
                out.color = vec4<f32>(0.35, 0.54, 0.28, 1.0);
            }
            default: {
                out.color = vec4<f32>(1.0, 1.0, 1.0, 1.0); 
            }
        }
    }

    let instance_local_pos = vec4<f32>(f32(x), f32(y), f32(z), 0.0);

    let multiplied_chunk_pos = push.chunk_pos * 64;
    let final_chunk_pos = vec4<f32>(f32(multiplied_chunk_pos.x), f32(multiplied_chunk_pos.y), f32(multiplied_chunk_pos.z), 0);

    let face_transform = FACE_TRANSFORMS[face_normal];

    var stretched_quad_pos = quad_pos;
    if face_normal == 1 || face_normal == 2 || face_normal == 5 {
        stretched_quad_pos.x = quad_pos.x * f32(h) - f32(h - 1);
    } else {
        stretched_quad_pos.x = quad_pos.x * f32(h);
    }

    out.position = push.pv * ((instance_local_pos + face_transform * vec4<f32>(stretched_quad_pos, 1.0)) + final_chunk_pos);
    return out;
};

@fragment
fn fragment_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color);
}