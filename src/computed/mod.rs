use std::{
    collections::{BTreeMap, HashMap},
    ops::Range,
    path::Path,
};

use image::{Pixel, Rgba, RgbaImage};
use tracing::error;

use crate::raw::{RawAsepriteChunkType, RawAsepriteUserData};
use crate::{
    error::{AseResult, AsepriteError, AsepriteInvalidError},
    raw::{
        AsepriteAnimationDirection, AsepriteBlendMode, AsepriteColor, AsepriteColorDepth,
        AsepriteLayerType, AsepriteNinePatchInfo, AsepritePixel, RawAseprite, RawAsepriteCel,
        RawAsepriteChunk, RawAsepritePaletteEntry,
    },
};

#[derive(Debug, Clone)]
/// Data structure representing an Aseprite file
pub struct Aseprite {
    dimensions: (u16, u16),
    tags: BTreeMap<usize, AsepriteTag>,
    slices: HashMap<String, AsepriteSlice>,
    layers: BTreeMap<usize, AsepriteLayer>,
    cels: BTreeMap<usize, BTreeMap<usize, AsepriteCel>>,
    frame_count: usize,
    palette: Option<AsepritePalette>,
    transparent_palette: Option<u8>,
    frame_infos: Vec<AsepriteFrameInfo>,
}

impl Aseprite {
    /// Get the [`AsepriteTag`]s defined in this Aseprite
    pub fn tags(&self) -> impl Iterator<Item = &AsepriteTag> {
        self.tags.values()
    }

    /// Get the associated [`AsepriteLayer`]s defined in this Aseprite
    pub fn layers(&self) -> impl Iterator<Item = &AsepriteLayer> {
        self.layers.values()
    }

    /// Get the frames inside this aseprite
    pub fn get_frame(&self, frame_index: usize) -> Option<AsepriteFrame> {
        if frame_index >= self.frame_count {
            return None;
        }
        Some(AsepriteFrame {
            aseprite: self,
            frame_index,
        })
    }

    /// Get the slices inside this aseprite
    pub fn slices(&self) -> AsepriteSlices {
        AsepriteSlices { aseprite: self }
    }

    /// Get the cel of giving layer and frame
    pub fn get_cel(&self, layer_index: &usize, frame_index: &usize) -> AseResult<&AsepriteCel> {
        let layer_cels = self
            .cels
            .get(layer_index)
            .ok_or(AsepriteInvalidError::InvalidLayer(*layer_index))?;
        let cel = layer_cels
            .get(frame_index)
            .ok_or(AsepriteInvalidError::InvalidFrame(*frame_index))?;
        Ok(cel)
    }

    /// Get a layer by its name
    ///
    /// If you have its id, prefer fetching it using [`get_by_id`]
    pub fn get_layer_by_name<N: AsRef<str>>(&self, name: N) -> Option<&AsepriteLayer> {
        let name = name.as_ref();
        self.layers
            .iter()
            .find(|(_, layer)| layer.name() == name)
            .map(|(_, layer)| layer)
    }

    /// Get a layer by its index
    pub fn get_layer_by_index(&self, index: &usize) -> Option<&AsepriteLayer> {
        self.layers.get(index)
    }

    /// 找到提供的 index 的 layer 属于的所有 groups
    pub fn find_layer_belong_groups(&self, index: usize) -> Vec<usize> {
        let Some(layer) = self.layers.get(&index) else {
            return Vec::new();
        };
        let mut cur_child_level = layer.child_level();
        let mut cur_index = index;
        let mut result = Vec::new();
        'find_all_group: while cur_child_level > 0 {
            cur_child_level -= 1;
            loop {
                cur_index -= 1;
                if let Some(layer) = self.layers.get(&cur_index) {
                    match layer {
                        AsepriteLayer::Group {
                            index, child_level, ..
                        } => {
                            if *child_level == cur_child_level {
                                cur_index = *index;
                                result.push(*index);
                                continue 'find_all_group;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        result
    }

    /// 根据传入的像素数据生成对应图像，其中如果传入 cel 则生成完整尺寸的图像，否则生成当前 sprite trim 后的图像
    fn write_image(
        &self,
        cel: Option<&AsepriteCel>,
        width: u16,
        height: u16,
        pixels: &[AsepritePixel],
    ) -> AseResult<RgbaImage> {
        let mut image = RgbaImage::new(width as u32, height as u32);
        for x in 0..width {
            for y in 0..height {
                let mut pix_x = x as i16;
                let mut pix_y = y as i16;
                if let Some(cel) = &cel {
                    pix_x += cel.x as i16;
                    pix_y += cel.y as i16
                }

                if pix_x < 0 || pix_y < 0 {
                    continue;
                }
                let raw_pixel = &pixels[(x + y * width) as usize];
                let pixel =
                    Rgba(raw_pixel.get_rgba(self.palette.as_ref(), self.transparent_palette)?);

                image
                    .get_pixel_mut(pix_x as u32, pix_y as u32)
                    .blend(&pixel);
            }
        }
        Ok(image)
    }

    /// Get images of each layer in this frame
    ///
    /// The key of return map is layer id
    pub fn get_image_by_layer_frame(
        &self,
        layer_index: &usize,
        frame_index: &usize,
    ) -> AseResult<RgbaImage> {
        let cel = self.get_cel(layer_index, frame_index)?;
        match &cel.raw_cel {
            RawAsepriteCel::Raw {
                width,
                height,
                pixels,
            }
            | RawAsepriteCel::Compressed {
                width,
                height,
                pixels,
            } => Ok(self.write_image(None, *width, *height, pixels)?),
            RawAsepriteCel::Linked { frame_position } => {
                let frame_index = (*frame_position as usize) - 1;
                match &self.get_cel(layer_index, &frame_index)?.raw_cel {
                    RawAsepriteCel::Raw {
                        width,
                        height,
                        pixels,
                    }
                    | RawAsepriteCel::Compressed {
                        width,
                        height,
                        pixels,
                    } => Ok(self.write_image(None, *width, *height, pixels)?),
                    RawAsepriteCel::Linked { frame_position } => {
                        error!("Tried to draw a linked cel twice! This should not happen, linked cel should not link to a linked cel.");
                        Err(AsepriteError::InvalidConfiguration(
                            AsepriteInvalidError::InvalidFrame(*frame_position as usize),
                        ))
                    }
                }
            }
        }
    }
}

impl Aseprite {
    /// Construct a [`Aseprite`] from a [`RawAseprite`]
    pub fn from_raw(raw: RawAseprite) -> AseResult<Self> {
        let mut tags = BTreeMap::new();
        let mut layers = BTreeMap::new();
        let mut cels = BTreeMap::new();
        let mut palette = None;
        let mut frame_infos = vec![];
        let mut slices = HashMap::new();

        let frame_count = raw.frames.len();

        // 记录上一个处理过的 chunk 类型，处理 user data 时需要知道他跟随在哪个 chunk 后面
        let mut last_chunk_type = RawAsepriteChunkType::ColorProfile;

        for frame in raw.frames {
            frame_infos.push(AsepriteFrameInfo {
                delay_ms: frame.duration_ms as usize,
            });

            for chunk in frame.chunks {
                match chunk {
                    RawAsepriteChunk::Layer {
                        flags,
                        layer_type,
                        layer_child,
                        width: _,
                        height: _,
                        blend_mode,
                        opacity,
                        name,
                    } => {
                        let layer_index = layers.len();
                        let layer = AsepriteLayer::new(
                            layer_index,
                            name,
                            layer_type,
                            flags & 0x1 != 0,
                            blend_mode,
                            if raw.header.flags & 0x1 != 0 {
                                Some(opacity)
                            } else {
                                None
                            },
                            layer_child,
                        );
                        layers.insert(layer_index, layer);
                        last_chunk_type = RawAsepriteChunkType::Layer;
                    }
                    RawAsepriteChunk::Cel {
                        layer_index,
                        x,
                        y,
                        opacity,
                        cel,
                    } => {
                        let layer_cels =
                            cels.entry(layer_index as usize).or_insert(BTreeMap::new());

                        let frame_index = layer_cels.len();
                        layer_cels.insert(
                            frame_index,
                            AsepriteCel::new(x as f64, y as f64, opacity, cel),
                        );

                        last_chunk_type =
                            RawAsepriteChunkType::Cel(layer_index as usize, frame_index);
                    }
                    RawAsepriteChunk::Tags { tags: raw_tags } => {
                        let start_index = tags.len();
                        let mut cur_index = start_index;
                        for raw_tag in raw_tags {
                            tags.insert(
                                cur_index,
                                AsepriteTag {
                                    index: cur_index,
                                    frames: raw_tag.from..raw_tag.to,
                                    animation_direction: raw_tag.anim_direction,
                                    name: raw_tag.name,
                                    color: AsepriteColor::default(),
                                    user_data: String::new(),
                                },
                            );
                            cur_index += 1;
                        }
                        last_chunk_type = RawAsepriteChunkType::Tags(start_index);
                    }
                    RawAsepriteChunk::Palette {
                        palette_size,
                        from_color,
                        to_color: _,
                        entries,
                    } => {
                        palette =
                            Some(AsepritePalette::from_raw(palette_size, from_color, entries));
                        last_chunk_type = RawAsepriteChunkType::Palette;
                    }
                    RawAsepriteChunk::UserData { data } => {
                        match &mut last_chunk_type {
                            RawAsepriteChunkType::Layer => {
                                let id = layers.len() - 1;
                                let layer = layers.get_mut(&id).unwrap();
                                layer.apply_raw_user_data(data);
                            }
                            // [Aseprite File Specs](https://github.com/aseprite/aseprite/blob/main/docs/ase-file-specs.md)
                            // After a Tags chunk, there will be several user data chunks, one for each tag,
                            // you should associate the user data in the same order as the tags are in the Tags chunk.
                            RawAsepriteChunkType::Tags(cur_index) => {
                                let tag = tags.get_mut(&cur_index).unwrap();
                                tag.apply_raw_user_data(data);
                                *cur_index += 1;
                            }
                            RawAsepriteChunkType::Cel(layer_index, frame_index) => {
                                let layer_cels = cels
                                    .get_mut(layer_index)
                                    .ok_or(AsepriteInvalidError::InvalidLayer(*layer_index))?;
                                let cel = layer_cels
                                    .get_mut(frame_index)
                                    .ok_or(AsepriteInvalidError::InvalidFrame(*frame_index))?;
                                cel.color = data.color;
                                cel.user_data = data.text;
                            }
                            _ => {}
                        }
                    }
                    RawAsepriteChunk::Slice {
                        name,
                        slices: raw_slices,
                        ..
                    } => {
                        slices.extend(raw_slices.into_iter().map(
                            |crate::raw::RawAsepriteSlice {
                                 frame,
                                 x_origin,
                                 y_origin,
                                 width,
                                 height,
                                 nine_patch_info,
                                 ..
                             }| {
                                (
                                    name.clone(),
                                    AsepriteSlice {
                                        name: name.clone(),
                                        valid_frame: frame as u16,
                                        position_x: x_origin,
                                        position_y: y_origin,
                                        width,
                                        height,
                                        nine_patch_info,
                                    },
                                )
                            },
                        ));
                        last_chunk_type = RawAsepriteChunkType::Slice;
                    }
                    RawAsepriteChunk::CelExtra { .. } => {
                        todo!("Not yet implemented cel extra")
                    }
                    RawAsepriteChunk::ColorProfile { .. } => {
                        error!("Not yet implemented color profile")
                        // todo!("Not yet implemented color profile")
                    }
                }
            }
        }

        Ok(Aseprite {
            dimensions: (raw.header.width, raw.header.height),
            transparent_palette: if raw.header.color_depth == AsepriteColorDepth::Indexed {
                Some(raw.header.transparent_palette)
            } else {
                None
            },
            tags,
            layers,
            cels,
            frame_count,
            palette,
            frame_infos,
            slices,
        })
    }

    /// Construct a [`Aseprite`] from a [`Path`]
    pub fn from_path<S: AsRef<Path>>(path: S) -> AseResult<Self> {
        let buffer = std::fs::read(path)?;

        let raw_aseprite = crate::raw::read_aseprite(&buffer)?;

        Ok(Self::from_raw(raw_aseprite)?)
    }

    /// Construct a [`Aseprite`] from a `&[u8]`
    pub fn from_bytes<S: AsRef<[u8]>>(buffer: S) -> AseResult<Self> {
        let raw_aseprite = crate::raw::read_aseprite(buffer.as_ref())?;

        Ok(Self::from_raw(raw_aseprite)?)
    }
}

/// The palette entries in the aseprite file
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct AsepritePalette {
    pub entries: Vec<AsepriteColor>,
}

impl AsepritePalette {
    fn from_raw(
        palette_size: u32,
        from_color: u32,
        raw_entries: Vec<RawAsepritePaletteEntry>,
    ) -> Self {
        let mut entries = vec![
            AsepriteColor {
                red: 0,
                green: 0,
                blue: 0,
                alpha: 0
            };
            palette_size as usize
        ];

        for (raw_idx, idx) in ((from_color as usize)..entries.len()).enumerate() {
            entries[idx] = raw_entries[raw_idx].color;
        }

        AsepritePalette { entries }
    }
}

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
    fn apply_raw_user_data(&mut self, value: RawAsepriteUserData) {
        self.color = value.color;
        self.user_data = value.text;
    }
}

#[derive(Debug, Clone)]
/// A single Aseprite slice
pub struct AsepriteSlice {
    /// The slice name
    pub name: String,
    /// The frame from which it is valid
    pub valid_frame: u16,
    /// The slice's x position
    pub position_x: i32,
    /// The slice's y position
    pub position_y: i32,
    /// The slice's width
    pub width: u32,
    /// The slice's height
    pub height: u32,
    /// Nine-Patch Info if it exists
    pub nine_patch_info: Option<AsepriteNinePatchInfo>,
}

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
    fn new(
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

    fn apply_raw_user_data(&mut self, value: RawAsepriteUserData) {
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

#[derive(Debug, Clone)]
/// A single cel in a frame in a layer
pub struct AsepriteCel {
    x: f64,
    y: f64,
    #[allow(dead_code)]
    opacity: u8,
    raw_cel: RawAsepriteCel,
    color: AsepriteColor,
    user_data: String,
}

impl AsepriteCel {
    fn new(x: f64, y: f64, opacity: u8, raw_cel: RawAsepriteCel) -> Self {
        AsepriteCel {
            x,
            y,
            opacity,
            raw_cel,
            color: AsepriteColor::default(),
            user_data: String::new(),
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
/// The nine slices in a nine-patch image
#[allow(missing_docs)]
pub enum NineSlice {
    TopLeft,
    TopCenter,
    TopRight,
    RightCenter,
    BottomRight,
    BottomCenter,
    BottomLeft,
    LeftCenter,
    Center,
}

/// A single slice image
///
/// Only contains nine-patch info if the aseprite also contained one
#[allow(missing_docs)]
pub struct AsepriteSliceImage {
    pub image: RgbaImage,
    pub nine_slices: Option<HashMap<NineSlice, RgbaImage>>,
}

/// The slices contained in an aseprite
#[derive(Debug)]
pub struct AsepriteSlices<'a> {
    aseprite: &'a Aseprite,
}

impl<'a> AsepriteSlices<'a> {
    /// Get a slice by name
    pub fn get_by_name(&self, name: &str) -> Option<&AsepriteSlice> {
        self.aseprite.slices.get(name)
    }

    /// Get all slices in this aseprite
    pub fn get_all(&self) -> impl Iterator<Item = &AsepriteSlice> + '_ {
        self.aseprite.slices.values()
    }

    /// Get the images represented by the slices
    pub fn get_images<I: Iterator<Item = &'a AsepriteSlice>>(
        &self,
        wanted_slices: I,
    ) -> AseResult<Vec<AsepriteSliceImage>> {
        let mut slices = vec![];

        for slice in wanted_slices {
            let frame = image_for_frame(self.aseprite, slice.valid_frame)?;

            let image = image::imageops::crop_imm(
                &frame,
                slice.position_x.max(0) as u32,
                slice.position_y.max(0) as u32,
                slice.width,
                slice.height,
            )
            .to_image();

            let slice_image = AsepriteSliceImage {
                nine_slices: slice.nine_patch_info.as_ref().map(|info| {
                    let mut map: HashMap<_, RgbaImage> = HashMap::new();

                    let patch_x = info.x_center as u32;
                    let patch_y = info.y_center as u32;

                    let x = 0;
                    let y = 0;
                    let width = patch_x;
                    let height = patch_y;
                    map.insert(
                        NineSlice::TopLeft,
                        image::imageops::crop_imm(&image, x, y, width, height).to_image(),
                    );

                    let x = patch_x;
                    let y = 0;
                    let width = info.width;
                    let height = patch_y;
                    map.insert(
                        NineSlice::TopCenter,
                        image::imageops::crop_imm(&image, x, y, width, height).to_image(),
                    );

                    let x = patch_x + info.width;
                    let y = 0;
                    let width = slice.width - info.width - patch_x;
                    let height = patch_y;
                    map.insert(
                        NineSlice::TopRight,
                        image::imageops::crop_imm(&image, x, y, width, height).to_image(),
                    );

                    let x = patch_x + info.width;
                    let y = patch_y;
                    let width = slice.width - info.width - patch_x;
                    let height = info.height;
                    map.insert(
                        NineSlice::RightCenter,
                        image::imageops::crop_imm(&image, x, y, width, height).to_image(),
                    );

                    let x = patch_x + info.width;
                    let y = info.height + patch_y;
                    let width = slice.width - info.width - patch_x;
                    let height = slice.height - info.height - patch_y;
                    map.insert(
                        NineSlice::BottomRight,
                        image::imageops::crop_imm(&image, x, y, width, height).to_image(),
                    );

                    let x = patch_x;
                    let y = patch_y + info.height;
                    let width = info.width;
                    let height = slice.height - info.height - patch_y;
                    map.insert(
                        NineSlice::BottomCenter,
                        image::imageops::crop_imm(&image, x, y, width, height).to_image(),
                    );

                    let x = 0;
                    let y = patch_y + info.height;
                    let width = patch_x;
                    let height = slice.height - info.height - patch_y;
                    map.insert(
                        NineSlice::BottomLeft,
                        image::imageops::crop_imm(&image, x, y, width, height).to_image(),
                    );

                    let x = 0;
                    let y = patch_y;
                    let width = patch_x;
                    let height = info.height;
                    map.insert(
                        NineSlice::LeftCenter,
                        image::imageops::crop_imm(&image, x, y, width, height).to_image(),
                    );

                    let x = patch_x;
                    let y = patch_y;
                    let width = info.width;
                    let height = info.height;
                    map.insert(
                        NineSlice::Center,
                        image::imageops::crop_imm(&image, x, y, width, height).to_image(),
                    );

                    map
                }),
                image,
            };

            slices.push(slice_image);
        }

        Ok(slices)
    }
}

/// Information about a single animation frame
#[derive(Debug, Clone)]
pub struct AsepriteFrameInfo {
    /// The delay of this frame in milliseconds
    pub delay_ms: usize,
}

/// Single frame in an aseprite
/// TODO 目前看没必要存在这个结构，有空都统一到 Aseprite 上
pub struct AsepriteFrame<'a> {
    aseprite: &'a Aseprite,
    frame_index: usize,
}

impl<'a> AsepriteFrame<'a> {
    /// Get the timings
    pub fn get_infos(&self) -> AseResult<&AsepriteFrameInfo> {
        Ok(&self.aseprite.frame_infos[self.frame_index])
    }

    /// Get images of each layer in this frame
    ///
    /// The key of return map is layer id
    pub fn get_image_by_layer(&self, layer_index: &usize) -> AseResult<RgbaImage> {
        self.aseprite
            .get_image_by_layer_frame(layer_index, &self.frame_index)
    }
}

/// 这个方法是获取某一帧所有可见图层合并后的图片
///
/// TODO 没有处理透明度和混合模式的效果
fn image_for_frame(aseprite: &Aseprite, frame_index: u16) -> AseResult<RgbaImage> {
    let dim = aseprite.dimensions;
    let frame_index = frame_index as usize;
    let mut image = RgbaImage::new(dim.0 as u32, dim.1 as u32);
    for (layer_index, layer) in &aseprite.layers {
        if !layer.is_visible() {
            continue;
        }

        let Ok(cel) = aseprite.get_cel(layer_index, &frame_index) else {
            continue;
        };

        let mut write_to_image = |cel: &AsepriteCel,
                                  width: u16,
                                  height: u16,
                                  pixels: &[AsepritePixel]|
         -> AseResult<()> {
            for x in 0..width {
                for y in 0..height {
                    let pix_x = cel.x as i16 + x as i16;
                    let pix_y = cel.y as i16 + y as i16;

                    if pix_x < 0 || pix_y < 0 {
                        continue;
                    }
                    let raw_pixel = &pixels[(x + y * width) as usize];
                    let pixel = Rgba(
                        raw_pixel
                            .get_rgba(aseprite.palette.as_ref(), aseprite.transparent_palette)?,
                    );

                    image
                        .get_pixel_mut(pix_x as u32, pix_y as u32)
                        .blend(&pixel);
                }
            }
            Ok(())
        };

        match &cel.raw_cel {
            RawAsepriteCel::Raw {
                width,
                height,
                pixels,
            }
            | RawAsepriteCel::Compressed {
                width,
                height,
                pixels,
            } => {
                write_to_image(&cel, *width, *height, pixels)?;
            }
            RawAsepriteCel::Linked { frame_position } => {
                let frame_index = *frame_position as usize - 1;
                match &aseprite.get_cel(layer_index, &frame_index)?.raw_cel {
                    RawAsepriteCel::Raw {
                        width,
                        height,
                        pixels,
                    }
                    | RawAsepriteCel::Compressed {
                        width,
                        height,
                        pixels,
                    } => {
                        write_to_image(&cel, *width, *height, pixels)?;
                    }
                    RawAsepriteCel::Linked { frame_position } => {
                        error!("Tried to draw a linked cel twice!");
                        return Err(AsepriteError::InvalidConfiguration(
                            AsepriteInvalidError::InvalidFrame(*frame_position as usize),
                        ));
                    }
                }
            }
        }
    }

    Ok(image)
}

#[cfg(test)]
#[allow(deprecated)]
mod test {
    use crate::raw::AsepriteBlendMode;
    use crate::Aseprite;

    #[test]
    fn check_aseprite_reader_result() {
        let aseprite = Aseprite::from_path("./tests/test_cases/complex.aseprite").unwrap();
        // println!("{aseprite:#?}");

        let col2row1_layer = aseprite.get_layer_by_name("Col2Row1").unwrap();
        let col2row1_layer_group_ids = aseprite.find_layer_belong_groups(col2row1_layer.index());
        let col2row1_layer_group: Vec<&str> = col2row1_layer_group_ids
            .into_iter()
            .map(|index| aseprite.get_layer_by_index(&index).unwrap().name())
            .collect();
        assert_eq!(col2row1_layer_group, vec!["Col2", "Table"]);

        // 验证 layer BG1 的属性是否正确
        {
            let layer = aseprite.get_layer_by_name("BG1").unwrap();
            let frame_2_cel = aseprite.get_cel(&layer.index(), &1).unwrap();

            assert_eq!(frame_2_cel.opacity, 128);
        }

        // 验证 layer Col1 的属性是否正确
        {
            let layer = aseprite.get_layer_by_name("Col1").unwrap();

            assert_eq!(layer.user_data(), "LayerCol1UserData");
        }

        // 验证 layer BG1 frame 0 的图片和属性是否正确
        {
            let layer = aseprite.get_layer_by_name("BG1").unwrap();
            let layer_index = layer.index();
            let layer_image = aseprite.get_image_by_layer_frame(&layer_index, &0).unwrap();

            let export_image = image::open("./tests/test_cases/images/complex_BG1.png")
                .unwrap()
                .to_rgba8();
            assert_eq!(layer_image, export_image);
            assert_eq!(layer.user_data(), "LayerBG1UserData");
        }

        // 验证 layer Col1Row1 frame 0 的图片和属性是否正确
        {
            let layer = aseprite.get_layer_by_name("Col1Row1").unwrap();
            let layer_index = layer.index();
            let layer_image = aseprite.get_image_by_layer_frame(&layer_index, &0).unwrap();

            let export_image = image::open("./tests/test_cases/images/complex_Col1Row1.png")
                .unwrap()
                .to_rgba8();
            assert_eq!(layer_image, export_image);
        }

        // 验证 layer 的相关属性是否正确
        for layer in aseprite.layers() {
            let layer_index = layer.index();
            match layer.name() {
                "BG1" => {
                    let cel = aseprite.get_cel(&layer_index, &0).unwrap();
                    assert_eq!(cel.user_data, "CelBG1Frame1UserData");
                    let cel = aseprite.get_cel(&layer_index, &1).unwrap();
                    assert_eq!(cel.user_data, "CelBG1Frame2UserData");

                    assert_eq!(layer.blend_mode(), AsepriteBlendMode::Normal);
                    assert_eq!(layer.opacity(), Some(255));
                }
                "Col1BG" => {
                    let cel = aseprite.get_cel(&layer_index, &0).unwrap();
                    assert_eq!(cel.user_data, "CelCol1BGFrame1UserData");
                }
                "Day" => {
                    assert_eq!(layer.blend_mode(), AsepriteBlendMode::SoftLight);
                    assert_eq!(layer.opacity(), Some(128));
                }
                "Night" => {
                    assert!(!layer.is_visible());
                }
                _ => {}
            }
        }

        // 验证 tag 的相关属性是否正确
        for tag in aseprite.tags() {
            match tag.name.as_str() {
                "FrameAllTag" => {
                    assert_eq!(tag.frames, 0..1);
                    assert_eq!(tag.user_data, "FrameAllTagUserData");
                }
                "Frame1Tag" => {
                    assert_eq!(tag.frames, 0..0);
                }
                "Frame2Tag" => {
                    assert_eq!(tag.frames, 1..1);
                    assert_eq!(tag.user_data, "Frame2TagUserData");
                }
                _ => {}
            }
        }
    }
}
