use crate::raw::{AsepriteColor, RawAsepritePaletteEntry};

/// The palette entries in the aseprite file
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct AsepritePalette {
    pub entries: Vec<AsepriteColor>,
}

impl AsepritePalette {
    pub(super) fn from_raw(
        palette_size: u32,
        from_color: u32,
        raw_entries: Vec<RawAsepritePaletteEntry>,
    ) -> Self {
        let mut entries = vec![
            AsepriteColor {
                red: 0,
                green: 0,
                blue: 0,
                alpha: 0,
            };
            palette_size as usize
        ];

        for (raw_idx, idx) in ((from_color as usize)..entries.len()).enumerate() {
            entries[idx] = raw_entries[raw_idx].color;
        }

        AsepritePalette { entries }
    }
}