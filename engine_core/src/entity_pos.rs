use serde::{Deserialize, Serialize};

use crate::block_pos::BlockPos;

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct EntityPos(nalgebra_glm::Vec3);

impl EntityPos {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        EntityPos(nalgebra_glm::vec3(x, y, z))
    }

    pub fn new_from_vec(position: nalgebra_glm::Vec3) -> Self {
        EntityPos(position)
    }

    pub fn to_block_pos(&self) -> BlockPos {
        BlockPos::new(
            self.x.floor() as i32,
            self.y.floor() as i32,
            self.z.floor() as i32,
        )
    }
}

impl std::ops::Deref for EntityPos {
    type Target = nalgebra_glm::Vec3;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for EntityPos {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
