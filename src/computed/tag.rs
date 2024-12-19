use std::ops::Range;

use crate::raw::{AsepriteAnimationDirection, AsepriteColor, RawAsepriteUserData};

#[derive(Debug, Clone)]
/// A single Aseprite tag
pub struct AsepriteTag {
    /// The tag index
    pub index: usize,
    /// The frames which this tag represents
    pub frames: Range<u16>,
    /// The direction of its animation
    pub animation_direction: AsepriteAnimationDirection,
    /// The tag name
    pub name: String,
    /// Tag color
    pub color: AsepriteColor,
    /// Tag user data
    pub user_data: String,
}

impl AsepriteTag {
    pub(super) fn apply_raw_user_data(&mut self, value: RawAsepriteUserData) {
        self.color = value.color;
        self.user_data = value.text;
    }
}
