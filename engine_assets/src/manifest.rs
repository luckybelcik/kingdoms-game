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

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum FaceValue {
    Simple(String),
    Detailed(FaceConfig),
}

#[derive(Debug, Deserialize, Clone)]
pub struct FaceConfig {
    pub path: String,
    pub colormap0: Option<ColormapConfig>,
    pub colormap1: Option<ColormapConfig>,
    pub colormap2: Option<ColormapConfig>,
    pub flip_x: Option<bool>,
    pub flip_y: Option<bool>,
}

impl FaceConfig {
    pub fn simple_from_path(path: &String) -> FaceConfig {
        FaceConfig {
            path: path.clone(),
            colormap0: None,
            colormap1: None,
            colormap2: None,
            flip_x: None,
            flip_y: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum SourceValue {
    Single(String),
    Dual(String, String),
}

#[derive(Debug, Deserialize, Clone)]
pub struct ColormapConfig {
    pub source: SourceValue,
    pub mask: String,
    pub map: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct TextureConfig {
    pub all: Option<FaceValue>,
    pub sides: Option<FaceValue>,
    pub ends: Option<FaceValue>,
    pub top: Option<FaceValue>,
    pub bottom: Option<FaceValue>,
    pub north: Option<FaceValue>,
    pub south: Option<FaceValue>,
    pub east: Option<FaceValue>,
    pub west: Option<FaceValue>,
}

impl TextureConfig {
    pub fn resolve_faces(&self) -> [FaceConfig; 6] {
        let resolve = |specific: &Option<FaceValue>, group: &Option<FaceValue>| {
            let value = specific
                .as_ref()
                .or(group.as_ref())
                .or(self.all.as_ref())
                .expect("Block texture missing a default!");

            match value {
                FaceValue::Simple(path) => FaceConfig {
                    path: path.clone(),
                    colormap0: None,
                    colormap1: None,
                    colormap2: None,
                    flip_x: None,
                    flip_y: None,
                },
                FaceValue::Detailed(config) => config.clone(),
            }
        };

        [
            resolve(&self.east, &self.sides),  // 0: +X
            resolve(&self.west, &self.sides),  // 1: -X
            resolve(&self.top, &self.ends),    // 2: +Y
            resolve(&self.bottom, &self.ends), // 3: -Y
            resolve(&self.north, &self.sides), // 4: +Z
            resolve(&self.south, &self.sides), // 5: -Z
        ]
    }
}
