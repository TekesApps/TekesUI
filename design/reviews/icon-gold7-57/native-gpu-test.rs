
    #[test]
    #[ignore="Explicit native vector shape evidence"]
    fn icon_gold7_native_all_sizes_gpu() {
        let out=std::path::PathBuf::from(std::env::var_os("ICON_NATIVE_OUT").unwrap());assert!(!out.exists());std::fs::create_dir_all(&out).unwrap();
        let mut renderer=Renderer::new_headless().unwrap();
        for(ti,theme)in Theme::ALL.into_iter().enumerate(){for backing in[1.,2.]{for size in[12.,16.,24.]{for(i,(icon,_))in Icon::ALL.into_iter().enumerate(){
            let frame=Rect::new((48.-size)*0.5,(48.-size)*0.5,size,size);
            let rgba=render_icon_pixels(&mut renderer,icon,frame,backing,theme,Rgba::rgb8(25,25,25));let side=(48.*backing)as u32;
            let mut png=png::Encoder::new(std::fs::File::create(out.join(format!("native-{i}-t{ti}-b{backing}-s{size}.png"))).unwrap(),side,side);png.set_color(png::ColorType::Rgba);png.set_depth(png::BitDepth::Eight);png.write_header().unwrap().write_image_data(&rgba).unwrap();
        }}}}
    }
