const QUAD_VERTICES: array<vec3<f32>, 4> = array<vec3<f32>, 4>(
    vec3<f32>(0.0, 0.0, 0.0),
    vec3<f32>(1.0, 0.0, 0.0),
    vec3<f32>(0.0, 1.0, 0.0),
    vec3<f32>(1.0, 1.0, 0.0),
);

//@group(0) @binding(0)
//var<uniform> ubo: Uniform;

struct PushConstants {
    pvm: mat4x4<f32>,
    app_render_config: u32,
}

var<push_constant> push: PushConstants;

struct VertexInput {
    @builtin(vertex_index) vertex_id: u32,
    @location(1) position: u32,
    @location(2) id: u32,
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
fn vertex_main(vert: VertexInput) -> VertexOutput {
    var transparency: f32 = 1.0;
    if vert.id == 0 {
        transparency = 0.5;
    }

    let render_textures = bool(push.app_render_config & 1);

    var out: VertexOutput;
    var quad_pos: vec3<f32>;
    switch vert.vertex_id {
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
        switch vert.id {
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

    // 0b11111 = 31
    let x = vert.position & 31;
    let y = (vert.position >> 5u) & 31;
    let z = (vert.position >> 10u) & 31;
    let index = (vert.position >> 15u) & 7;

    let instance_local_pos = vec4<f32>(f32(x), f32(y), f32(z), 0.0);

    let face_transform = FACE_TRANSFORMS[index];

    out.position = push.pvm * (instance_local_pos + face_transform * vec4<f32>(quad_pos, 1.0));
    return out;
};

@fragment
fn fragment_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color);
}