use serde::Deserialize;

#[derive(Deserialize)]
pub struct BlockManifest {
    pub blocks: Vec<BlockDefinition>,
}

#[derive(Deserialize)]
pub struct BlockDefinition {
    pub id: String,
    pub faces: FacesOptions,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum FacesOptions {
    Unified(String),
    Unique(FaceConfigs),
}

#[derive(Debug, Deserialize, Default)]
pub struct FaceConfigs {
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

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum FaceValue {
    Simple(String),
    Detailed(FaceConfig),
    Variable(FaceConfigWithVariants),
}

#[derive(Debug, Deserialize, Clone)]
pub struct FaceConfig {
    pub face: FaceDefinition,
    pub colormap0: Option<ColormapConfig>,
    pub colormap1: Option<ColormapConfig>,
    pub colormap2: Option<ColormapConfig>,
    pub flip_x: Option<bool>,
    pub flip_y: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FaceConfigWithVariants {
    pub faces: Vec<FaceDefinition>,
    pub colormap0: Option<ColormapConfig>,
    pub colormap1: Option<ColormapConfig>,
    pub colormap2: Option<ColormapConfig>,
    pub flip_x: Option<bool>,
    pub flip_y: Option<bool>,
    pub fully_random_faces: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FaceDefinition {
    pub texture: String,
    pub colormap0_mask: Option<String>,
    pub colormap1_mask: Option<String>,
    pub colormap2_mask: Option<String>,
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
    pub map: String,
}

impl FaceConfigs {
    pub fn resolve_faces(&self) -> [FaceConfigWithVariants; 6] {
        let resolve = |specific: &Option<FaceValue>, group: &Option<FaceValue>| {
            let value = specific
                .as_ref()
                .or(group.as_ref())
                .or(self.all.as_ref())
                .expect("Block texture missing a default!");

            match value {
                FaceValue::Simple(path) => FaceConfigWithVariants {
                    faces: vec![FaceDefinition::new_default(path)],
                    colormap0: None,
                    colormap1: None,
                    colormap2: None,
                    flip_x: None,
                    flip_y: None,
                    fully_random_faces: None,
                },
                FaceValue::Detailed(config) => FaceConfigWithVariants {
                    faces: vec![config.face.clone()],
                    colormap0: config.colormap0.clone(),
                    colormap1: config.colormap1.clone(),
                    colormap2: config.colormap2.clone(),
                    flip_x: config.flip_x,
                    flip_y: config.flip_y,
                    fully_random_faces: None,
                },
                FaceValue::Variable(config) => config.clone(),
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

impl FaceDefinition {
    pub fn new_default(path: &String) -> FaceDefinition {
        FaceDefinition {
            texture: path.clone(),
            colormap0_mask: None,
            colormap1_mask: None,
            colormap2_mask: None,
        }
    }
}

impl FaceConfigWithVariants {
    pub fn simple_from_path(path: &String) -> FaceConfigWithVariants {
        FaceConfigWithVariants {
            faces: vec![FaceDefinition::new_default(path)],
            colormap0: None,
            colormap1: None,
            colormap2: None,
            flip_x: None,
            flip_y: None,
            fully_random_faces: None,
        }
    }
}
