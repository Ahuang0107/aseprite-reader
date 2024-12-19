use crate::raw::{AsepriteBlendMode, AsepriteColor, AsepriteLayerType, RawAsepriteUserData};

use super::AsepriteCel;

#[derive(Debug, Clone)]
/// An aseprite layer
pub enum AsepriteLayer {
    /// A layer group
    Group {
        /// Name of the layer
        name: String,
        /// Index of the layer
        ///
        /// 所有 Layer Chunk 都是存储在一起的，这是按照 Layer Chunk 存储的顺序来决定，从 0 开始，对应了 aseprite 文件中从下到上的 layer 顺序
        index: usize,
        /// Visibility of the layer
        visible: bool,
        /// How deep it is nested in the layer hierarchy
        child_level: u16,
        /// Layer color
        color: AsepriteColor,
        /// Layer user data
        user_data: String,
    },
    /// A normal layer
    Normal {
        /// Name of the layer
        name: String,
        /// Index of the layer
        ///
        /// 所有 Layer Chunk 都是存储在一起的，这是按照 Layer Chunk 存储的顺序来决定，从 0 开始，对应了 aseprite 文件中从下到上的 layer 顺序
        index: usize,
        /// Blend mode of this layer
        blend_mode: AsepriteBlendMode,
        /// Opacity of this layer (if enabled)
        opacity: Option<u8>,
        /// Visibility of this layer
        visible: bool,
        /// How deep it is nested in the layer hierarchy
        child_level: u16,
        /// Cels
        cels: Vec<AsepriteCel>,
        /// Layer color
        color: AsepriteColor,
        /// Layer user data
        user_data: String,
    },
}

impl AsepriteLayer {
    pub(super) fn new(
        index: usize,
        name: String,
        layer_type: AsepriteLayerType,
        visible: bool,
        blend_mode: AsepriteBlendMode,
        opacity: Option<u8>,
        child_level: u16,
    ) -> Self {
        match layer_type {
            AsepriteLayerType::Normal => AsepriteLayer::Normal {
                name,
                index,
                blend_mode,
                opacity,
                visible,
                child_level,
                cels: vec![],
                color: AsepriteColor::default(),
                user_data: String::new(),
            },
            AsepriteLayerType::Group => AsepriteLayer::Group {
                name,
                index,
                visible,
                child_level,
                color: AsepriteColor::default(),
                user_data: String::new(),
            },
        }
    }

    /// Get the name of the layer
    pub fn name(&self) -> &str {
        match self {
            AsepriteLayer::Group { name, .. } | AsepriteLayer::Normal { name, .. } => &name,
        }
    }

    /// Get the index of the layer
    pub fn index(&self) -> usize {
        match self {
            AsepriteLayer::Group { index, .. } | AsepriteLayer::Normal { index, .. } => *index,
        }
    }

    /// Get the visibility of the layer
    pub fn is_visible(&self) -> bool {
        match self {
            AsepriteLayer::Group { visible, .. } | AsepriteLayer::Normal { visible, .. } => {
                *visible
            }
        }
    }

    /// Get child level of the layer
    pub fn child_level(&self) -> u16 {
        match self {
            AsepriteLayer::Group { child_level, .. }
            | AsepriteLayer::Normal { child_level, .. } => *child_level,
        }
    }

    /// Get blend mode of normal layer
    pub fn blend_mode(&self) -> AsepriteBlendMode {
        match self {
            AsepriteLayer::Group { .. } => AsepriteBlendMode::Normal,
            AsepriteLayer::Normal { blend_mode, .. } => *blend_mode,
        }
    }

    /// Get opacity of normal layer
    pub fn opacity(&self) -> Option<u8> {
        match self {
            AsepriteLayer::Group { .. } => None,
            AsepriteLayer::Normal { opacity, .. } => *opacity,
        }
    }

    pub(super) fn apply_raw_user_data(&mut self, value: RawAsepriteUserData) {
        match self {
            AsepriteLayer::Group {
                color, user_data, ..
            } => {
                *color = value.color;
                *user_data = value.text;
            }
            AsepriteLayer::Normal {
                color, user_data, ..
            } => {
                *color = value.color;
                *user_data = value.text;
            }
        }
    }

    /// Get user data of the layer
    pub fn user_data(&self) -> &str {
        return match self {
            AsepriteLayer::Group { user_data, .. } => user_data.as_str(),
            AsepriteLayer::Normal { user_data, .. } => user_data.as_str(),
        };
    }
}
