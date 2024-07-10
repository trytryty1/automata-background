struct SimulationParametersUniform {
    width: u32,
    height: u32,
};

@group(0) @binding(0)
var<uniform> simulation_parameters: SimulationParametersUniform;

// Vertex shader
struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct InstanceInput {
    @location(1) position: vec2<u32>,
    @location(2) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.color = instance.color;

    let paddingx = 2.0 / f32(simulation_parameters.width);
    let paddingy = 2.0 / f32(simulation_parameters.height);

    // Assuming viewport width and height are known and represent the size of your rendering area
    let cell_width = (2.0 - paddingx) / f32(simulation_parameters.width);
    let cell_height = (2.0 - paddingy) / f32(simulation_parameters.height);

    let screen_x = (f32(instance.position.x) * cell_width) + (model.position.x * cell_width) -1.0 + (cell_width / 2.0);
    let screen_y = (f32(instance.position.y) * cell_height) + (model.position.y * cell_height) -1.0 + (cell_height / 2.0);


    // Create the clip space position with z = 0.0 and w = 1.0
    out.clip_position = vec4<f32>(
        screen_x,
        screen_y,
        0.0,
        1.0
    );

    return out;
}



// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}