use crate::raw::AsepriteBlendMode;

use super::Aseprite;

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
        let layer_image = aseprite.get_image_by_layer_frame(&layer_index, &0).unwrap().unwrap();

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
        let layer_image = aseprite.get_image_by_layer_frame(&layer_index, &0).unwrap().unwrap();

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