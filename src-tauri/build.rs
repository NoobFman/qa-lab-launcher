fn main() {
    let icon_dir = std::path::Path::new("icons");
    let icon_path = icon_dir.join("icon.ico");
    std::fs::create_dir_all(icon_dir).expect("create icons directory");
    // Minimal valid 1x1 BGRA/DIB icon. Replace with branded assets for production.
    let mut dib = Vec::new();
    dib.extend_from_slice(&40u32.to_le_bytes()); // BITMAPINFOHEADER
    dib.extend_from_slice(&1i32.to_le_bytes()); dib.extend_from_slice(&2i32.to_le_bytes());
    dib.extend_from_slice(&1u16.to_le_bytes()); dib.extend_from_slice(&32u16.to_le_bytes());
    dib.extend_from_slice(&0u32.to_le_bytes()); dib.extend_from_slice(&4u32.to_le_bytes());
    dib.extend_from_slice(&[0; 16]);
    dib.extend_from_slice(&[0xED, 0x61, 0x5B, 0xFF]); // pixel
    dib.extend_from_slice(&[0; 4]); // AND mask
    let mut ico = vec![0,0,1,0,1,0,1,1,0,0,1,0,32,0];
    ico.extend_from_slice(&(dib.len() as u32).to_le_bytes()); ico.extend_from_slice(&22u32.to_le_bytes()); ico.extend_from_slice(&dib);
    std::fs::write(icon_path, ico).expect("write icon");
    tauri_build::build()
}
