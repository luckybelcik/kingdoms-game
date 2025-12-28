pub const TICK_RATE: u32 = 20;
pub const FIXED_TIMESTEP: f32 = 1.0 / TICK_RATE as f32;
pub const MOVE_SPEED: f32 = 2.0 * FIXED_TIMESTEP;

pub const MAX_ACCEPTABLE_POSITION_DELTA: f32 = 0.5;
pub const MAX_NEW_CHUNK_COUNT: usize = 5;
