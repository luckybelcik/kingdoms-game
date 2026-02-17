use serde::Deserialize;

#[derive(Deserialize)]
pub struct BlockManifest {
    pub blocks: Vec<BlockDefinition>,
}

#[derive(Deserialize)]
pub struct BlockDefinition {
    pub id: String,
    pub texture: TextureValue,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TextureValue {
    Simple(String),
    Complex(TextureConfig),
}

#[derive(Debug, Deserialize, Default)]
pub struct TextureConfig {
    pub all: Option<String>,
    pub sides: Option<String>,
    pub ends: Option<String>,
    pub top: Option<String>,
    pub bottom: Option<String>,
    pub north: Option<String>,
    pub south: Option<String>,
    pub east: Option<String>,
    pub west: Option<String>,
}

impl TextureConfig {
    pub fn resolve_faces(&self) -> [String; 6] {
        // Priority 1: Specific face
        // Priority 2: .sides and .ends
        // Priority 3: .all or the simple string
        let get = |face: &Option<String>, group: &Option<String>| {
            face.clone()
                .or_else(|| group.clone()) // Check "ends" or "sides"
                .or_else(|| self.all.clone()) // Check "all"
                .expect("Block texture missing a default!")
        };

        [
            get(&self.east, &self.sides),  // 0: +X
            get(&self.west, &self.sides),  // 1: -X
            get(&self.top, &self.ends),    // 2: +Y
            get(&self.bottom, &self.ends), // 3: -Y
            get(&self.north, &self.sides), // 4: +Z
            get(&self.south, &self.sides), // 5: -Z
        ]
    }
}
