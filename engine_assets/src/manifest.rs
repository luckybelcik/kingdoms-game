use serde::Deserialize;

#[derive(Deserialize)]
pub struct BlockManifest {
    pub blocks: Vec<BlockDefinition>,
}

#[derive(Deserialize)]
pub struct BlockDefinition {
    pub id: String,
    pub texture_name: String,
}
