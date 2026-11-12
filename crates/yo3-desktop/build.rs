use std::{env, fs, io::BufReader, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=assets/branding/launcher-window.png");
    println!("cargo:rerun-if-changed=assets/branding/launcher.ico");
    let mut reader = png::Decoder::new(BufReader::new(fs::File::open("assets/branding/launcher-window.png")?)).read_info()?;
    let mut pixels = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut pixels)?;
    assert_eq!((frame.width, frame.height, frame.color_type, frame.bit_depth), (256, 256, png::ColorType::Rgba, png::BitDepth::Eight), "The window icon must be a 256x256 RGBA8 PNG");
    fs::write(PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory")).join("launcher-icon.rgba"), &pixels[..frame.buffer_size()])?;

    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("assets/branding/launcher.ico")
            .set("ProductName", "YO3 Revival")
            .set("FileDescription", "YO3 Revival launcher")
            .set("OriginalFilename", "yo3-revival.exe")
            .compile_for("yo3-revival")?;
    }
    Ok(())
}
