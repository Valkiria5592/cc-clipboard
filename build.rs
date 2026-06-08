fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    println!("cargo:rerun-if-changed=assets/logo.jpeg");

    generate_icon();

    let mut res = winres::WindowsResource::new();
    res.set_icon("assets/icon.ico");
    res.compile().expect("winres compile failed");
}

fn generate_icon() {
    use image::imageops::FilterType;
    use image::GenericImageView;

    let img = image::open("assets/logo.jpeg").expect("assets/logo.jpeg not found");
    let (w, h) = img.dimensions();

    // Center-crop to square — crab is centered in the 2752×1536 source
    let size = w.min(h);
    let x = (w - size) / 2;
    let y = (h - size) / 2;
    let cropped = img.crop_imm(x, y, size, size);

    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);

    // Standard ICO sizes: PNG-encoded for 256, BMP-encoded for smaller
    for &px in &[256u32, 128, 64, 48, 32, 16] {
        let resized = cropped.resize(px, px, FilterType::Lanczos3).to_rgba8();
        let ico_img = ico::IconImage::from_rgba_data(px, px, resized.into_raw());
        let entry = if px >= 256 {
            ico::IconDirEntry::encode_as_png(&ico_img)
        } else {
            ico::IconDirEntry::encode(&ico_img)
        };
        icon_dir.add_entry(entry.expect("ico encode"));
    }

    let path = "assets/icon.ico";
    let file = std::fs::File::create(path).expect("create icon.ico");
    icon_dir.write(file).expect("write icon.ico");
}
