pub mod asset_importer;
use nx_hal_fs::*;
use std::env;
use std::fs;

fn align_up(val: usize, align: usize) -> usize { (val + align - 1) & !(align - 1) }

pub fn generate_sample_assets(folder: &str) {
    fs::create_dir_all(format!("{}/meshes", folder)).unwrap();
    let vertices = [
        NxVertex3D { position: [0.0, 1.0, 0.0], normal: [0.0, 1.0, 0.0], uv: [0.5, 0.0] },
        NxVertex3D { position: [-1.0, -1.0, 1.0], normal: [-0.7, 0.0, 0.7], uv: [0.0, 1.0] },
        NxVertex3D { position: [1.0, -1.0, 1.0], normal: [0.7, 0.0, 0.7], uv: [1.0, 1.0] },
    ];
    let indices: [u32; 3] = [0, 1, 2];
    let header = NxAssetHeader {
        magic: *b"NXM1", version: 1, vertex_count: 3, index_count: 3, checksum: 12345, _reserved: 0,
    };
    let mut bytes = Vec::new();
    bytes.extend_from_slice(bytemuck::bytes_of(&header));
    bytes.extend_from_slice(bytemuck::cast_slice(&vertices));
    bytes.extend_from_slice(bytemuck::cast_slice(&indices));
    fs::write(format!("{}/meshes/nave.nxmesh", folder), &bytes).unwrap();
}

pub fn pack_directory(input_dir: &str, output_pak: &str) {
    let mesh_path = format!("{}/meshes/nave.nxmesh", input_dir);
    let data = fs::read(&mesh_path).expect("Ejecuta primero 'init-assets'");
    let h_size = std::mem::size_of::<NxPakHeader>();
    let t_size = std::mem::size_of::<NxPakEntry>();
    let data_offset = align_up(h_size + t_size, NXPAK_ALIGNMENT);
    let total_size = align_up(data_offset + data.len(), NXPAK_ALIGNMENT);

    let mut pak = vec![0u8; total_size];
    let header = NxPakHeader {
        magic: NXPAK_MAGIC, version: NXPAK_VERSION, entry_count: 1, flags: 0,
        toc_offset: h_size as u64, data_offset: data_offset as u64,
    };
    let mut debug_name = [0u8; 24];
    debug_name[..18].copy_from_slice(b"meshes/nave.nxmesh");
    let entry = NxPakEntry {
        path_hash: hash_asset_path("meshes/nave.nxmesh"),
        asset_kind: 1, flags: 0, offset: data_offset as u64, size: data.len() as u64,
        payload_checksum: fnv1a_hash_64(&data), debug_name,
    };
    pak[0..h_size].copy_from_slice(bytemuck::bytes_of(&header));
    pak[h_size..h_size + t_size].copy_from_slice(bytemuck::bytes_of(&entry));
    pak[data_offset..data_offset + data.len()].copy_from_slice(&data);

    if let Some(p) = std::path::Path::new(output_pak).parent() { let _ = fs::create_dir_all(p); }
    fs::write(output_pak, &pak).unwrap();
    println!("📦 [NX-Pak] '{}' creado ({} bytes alineados a 64B).", output_pak, pak.len());
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("init-assets") => generate_sample_assets(args.get(2).map(|s| s.as_str()).unwrap_or("assets")),
        Some("pack") => pack_directory(
            args.get(2).map(|s| s.as_str()).unwrap_or("assets"),
            args.get(3).map(|s| s.as_str()).unwrap_or("dist/game_data.nxpak"),
        ),
        _ => println!("Uso: cargo run -p nx_cli -- <init-assets|pack>"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_pak_roundtrip() {
        let dir = std::env::temp_dir().join("nx_cli_auto_test");
        let pak_file = std::env::temp_dir().join("auto.nxpak");
        generate_sample_assets(dir.to_str().unwrap());
        pack_directory(dir.to_str().unwrap(), pak_file.to_str().unwrap());
        let raw = fs::read(&pak_file).unwrap();
        let archive = NxPakArchive::mount(&raw).unwrap();
        let mesh = archive.load_mesh_zero_copy("meshes/nave.nxmesh").unwrap();
        assert_eq!(mesh.vertices.len(), 3);
    }
}
