use crate::raw::{AsepriteBlendMode, AsepriteColor, AsepriteLayerType, RawAsepriteUserData};

#[derive(Debug, Clone)]
/// An aseprite layer
pub enum AsepriteLayer {
    /// A layer group
    Group(GroupLayer),
    /// A normal layer
    Normal(NormalLayer),
}

/// 表示图层组
#[derive(Debug, Clone)]
pub struct GroupLayer {
    /// Name of the layer
    pub name: String,
    /// Index of the layer
    ///
    /// 所有 Layer Chunk 都是存储在一起的，这是按照 Layer Chunk 存储的顺序来决定，从 0 开始，对应了 aseprite 文件中从下到上的 layer 顺序
    pub index: usize,
    /// Visibility of the layer
    pub visible: bool,
    /// How deep it is nested in the layer hierarchy
    pub child_level: u16,
    /// Layer color
    pub color: AsepriteColor,
    /// Layer user data
    pub user_data: String,
}

/// 表示普通图层
#[derive(Debug, Clone)]
pub struct NormalLayer {
    /// Name of the layer
    pub name: String,
    /// Index of the layer
    ///
    /// 所有 Layer Chunk 都是存储在一起的，这是按照 Layer Chunk 存储的顺序来决定，从 0 开始，对应了 aseprite 文件中从下到上的 layer 顺序
    pub index: usize,
    /// Blend mode of this layer
    pub blend_mode: AsepriteBlendMode,
    /// Opacity of this layer (if enabled)
    pub opacity: Option<u8>,
    /// Visibility of this layer
    pub visible: bool,
    /// How deep it is nested in the layer hierarchy
    pub child_level: u16,
    /// Layer color
    pub color: AsepriteColor,
    /// Layer user data
    pub user_data: String,
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
            AsepriteLayerType::Normal => AsepriteLayer::Normal(NormalLayer {
                name,
                index,
                blend_mode,
                opacity,
                visible,
                child_level,
                color: AsepriteColor::default(),
                user_data: String::new(),
            }),
            AsepriteLayerType::Group => AsepriteLayer::Group(GroupLayer {
                name,
                index,
                visible,
                child_level,
                color: AsepriteColor::default(),
                user_data: String::new(),
            }),
        }
    }

    /// Get the name of the layer
    pub fn name(&self) -> &str {
        match self {
            AsepriteLayer::Group(GroupLayer { name, .. })
            | AsepriteLayer::Normal(NormalLayer { name, .. }) => &name,
        }
    }

    /// Get the index of the layer
    pub fn index(&self) -> usize {
        match self {
            AsepriteLayer::Group(GroupLayer { index, .. })
            | AsepriteLayer::Normal(NormalLayer { index, .. }) => *index,
        }
    }

    /// Get the visibility of the layer
    pub fn is_visible(&self) -> bool {
        match self {
            AsepriteLayer::Group(GroupLayer { visible, .. })
            | AsepriteLayer::Normal(NormalLayer { visible, .. }) => *visible,
        }
    }

    /// Get child level of the layer
    pub fn child_level(&self) -> u16 {
        match self {
            AsepriteLayer::Group(GroupLayer { child_level, .. })
            | AsepriteLayer::Normal(NormalLayer { child_level, .. }) => *child_level,
        }
    }

    /// Get blend mode of normal layer
    pub fn blend_mode(&self) -> AsepriteBlendMode {
        match self {
            AsepriteLayer::Group(..) => AsepriteBlendMode::Normal,
            AsepriteLayer::Normal(NormalLayer { blend_mode, .. }) => *blend_mode,
        }
    }

    /// Get opacity of normal layer
    pub fn opacity(&self) -> Option<u8> {
        match self {
            AsepriteLayer::Group(..) => None,
            AsepriteLayer::Normal(NormalLayer { opacity, .. }) => *opacity,
        }
    }

    pub(super) fn apply_raw_user_data(&mut self, value: RawAsepriteUserData) {
        match self {
            AsepriteLayer::Group(GroupLayer {
                color, user_data, ..
                                 }) => {
                *color = value.color;
                *user_data = value.text;
            }
            AsepriteLayer::Normal(NormalLayer {
                color, user_data, ..
                                  }) => {
                *color = value.color;
                *user_data = value.text;
            }
        }
    }

    /// Get user data of the layer
    pub fn user_data(&self) -> &str {
        return match self {
            AsepriteLayer::Group(GroupLayer { user_data, .. }) => user_data.as_str(),
            AsepriteLayer::Normal(NormalLayer { user_data, .. }) => user_data.as_str(),
        };
    }
}
