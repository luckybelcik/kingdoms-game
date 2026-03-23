use crate::manifest::BlockDefinition;

#[derive(Debug)]
pub struct BlockProperties {
    pub display_name: String,
}

impl From<BlockDefinition> for BlockProperties {
    fn from(definition: BlockDefinition) -> Self {
        BlockProperties {
            display_name: definition.id,
        }
    }
}
