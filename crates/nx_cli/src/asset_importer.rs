// Archivo: crates/nx_cli/src/asset_importer.rs
use bytemuck::{Pod, Zeroable};
use nx_hal_fs::{fnv1a_hash_64, NxAssetHeader, NxVertex3D};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Cabecera binaria alineada a 32 bytes para texturas universales `.nxtex`
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct NxTexHeader {
    pub magic: [u8; 4],      // Siempre b"NXT1"
    pub version: u32,        // 1
    pub width: u32,          // Ancho base en píxeles
    pub height: u32,         // Alto base en píxeles
    pub mip_levels: u32,     // Cantidad de niveles Mipmap incluidos en el archivo
    pub format: u32,         // 1 = RGBA8_SRGB, 2 = ASTC_4x4, 3 = BC7
    pub payload_checksum: u64,
}

/// Clave binaria exacta para deduplicar vértices idénticos en O(1)
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct VertexKey {
    pos_bits: [u32; 3],
    norm_bits: [u32; 3],
    uv_bits: [u32; 2],
}

impl VertexKey {
    fn from_vertex(v: &NxVertex3D) -> Self {
        Self {
            pos_bits: [v.position[0].to_bits(), v.position[1].to_bits(), v.position[2].to_bits()],
            norm_bits: [v.normal[0].to_bits(), v.normal[1].to_bits(), v.normal[2].to_bits()],
            uv_bits: [v.uv[0].to_bits(), v.uv[1].to_bits()],
        }
    }
}

/// Calcula la normal unitaria de un triángulo 3D usando el producto cruz $(B - A) \times (C - A)$
#[inline(always)]
fn compute_face_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let ux = b[0] - a[0];
    let uy = b[1] - a[1];
    let uz = b[2] - a[2];

    let vx = c[0] - a[0];
    let vy = c[1] - a[1];
    let vz = c[2] - a[2];

    let nx = uy * vz - uz * vy;
    let ny = uz * vx - ux * vz;
    let nz = ux * vy - uy * vx;

    let len = (nx * nx + ny * ny + nz * nz).sqrt();
    if len > 1e-8 {
        [nx / len, ny / len, nz / len]
    } else {
        [0.0, 1.0, 0.0]
    }
}

/// Empaqueta una lista de vértices e índices ya optimizados al formato binario `.nxmesh`
pub fn write_nxmesh_binary(
    output_path: &Path,
    vertices: &[NxVertex3D],
    indices: &[u32],
) -> Result<(), String> {
    if vertices.is_empty() || indices.is_empty() {
        return Err("La malla 3D no contiene geometría válida".to_string());
    }

    let vert_bytes: &[u8] = bytemuck::cast_slice(vertices);
    let idx_bytes: &[u8] = bytemuck::cast_slice(indices);

    // Checksum combinado sobre la geometría para validación en caliente
    let mut payload = Vec::with_capacity(vert_bytes.len() + idx_bytes.len());
    payload.extend_from_slice(vert_bytes);
    payload.extend_from_slice(idx_bytes);
    let checksum = fnv1a_hash_64(&payload);

    let header = NxAssetHeader {
        magic: *b"NXM1",
        version: 1,
        vertex_count: vertices.len() as u32,
        index_count: indices.len() as u32,
        checksum,
        _reserved: 0,
    };

    let mut final_file = Vec::with_capacity(std::mem::size_of::<NxAssetHeader>() + payload.len());
    final_file.extend_from_slice(bytemuck::bytes_of(&header));
    final_file.extend_from_slice(&payload);

    fs::write(output_path, &final_file)
        .map_err(|e| format!("Error al guardar '{}': {}", output_path.display(), e))?;

    println!(
        "📐 [Mesh Importer] '{}' -> {} vértices únicos, {} triángulos ({} bytes).",
        output_path.display(),
        vertices.len(),
        indices.len() / 3,
        final_file.len()
    );
    Ok(())
}

// ============================================================================
// 1. IMPORTADOR Y OPTIMIZADOR DE MODELOS WAVEFRONT (.OBJ -> .NXMESH)
// ============================================================================
pub fn import_obj_to_nxmesh(input_obj: &Path, output_nxmesh: &Path) -> Result<(), String> {
    let content = fs::read_to_string(input_obj)
        .map_err(|e| format!("No se pudo leer '{}': {}", input_obj.display(), e))?;

    let mut raw_positions: Vec<[f32; 3]> = Vec::new();
    let mut raw_normals: Vec<[f32; 3]> = Vec::new();
    let mut raw_uvs: Vec<[f32; 2]> = Vec::new();

    let mut unique_vertices: Vec<NxVertex3D> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut dedup_map: HashMap<VertexKey, u32> = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("v") => {
                let x = parts.next().and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
                let y = parts.next().and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
                let z = parts.next().and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
                raw_positions.push([x, y, z]);
            }
            Some("vn") => {
                let nx = parts.next().and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
                let ny = parts.next().and_then(|s| s.parse::<f32>().ok()).unwrap_or(1.0);
                let nz = parts.next().and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
                raw_normals.push([nx, ny, nz]);
            }
            Some("vt") => {
                let u = parts.next().and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
                let v = parts.next().and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
                // Invertir eje V para coincidir con el origen superior-izquierdo de Vulkan/Metal/WebGPU
                raw_uvs.push([u, 1.0 - v]);
            }
            Some("f") => {
                // Soporta triángulos, quads y polígonos N-gonos mediante triangulación en abanico
                let face_tokens: Vec<&str> = parts.collect();
                if face_tokens.len() < 3 {
                    continue;
                }

                let mut face_verts = Vec::with_capacity(face_tokens.len());
                let mut missing_normal = false;

                for token in &face_tokens {
                    let mut comps = token.split('/');
                    let p_idx = comps
                        .next()
                        .and_then(|s| s.parse::<usize>().ok())
                        .unwrap_or(1)
                        .saturating_sub(1);
                    let uv_idx = comps
                        .next()
                        .and_then(|s| if s.is_empty() { None } else { s.parse::<usize>().ok() })
                        .map(|i| i.saturating_sub(1));
                    let n_idx = comps
                        .next()
                        .and_then(|s| if s.is_empty() { None } else { s.parse::<usize>().ok() })
                        .map(|i| i.saturating_sub(1));

                    let position = raw_positions.get(p_idx).copied().unwrap_or([0.0; 3]);
                    let uv = uv_idx
                        .and_then(|i| raw_uvs.get(i).copied())
                        .unwrap_or([0.0, 0.0]);
                    let normal = match n_idx.and_then(|i| raw_normals.get(i).copied()) {
                        Some(n) => n,
                        None => {
                            missing_normal = true;
                            [0.0, 1.0, 0.0]
                        }
                    };

                    face_verts.push(NxVertex3D {
                        position,
                        normal,
                        uv,
                    });
                }

                // Si el archivo .obj no tenía `vn`, calculamos la normal geométrica de la cara
                if missing_normal && face_verts.len() >= 3 {
                    let auto_n = compute_face_normal(
                        face_verts[0].position,
                        face_verts[1].position,
                        face_verts[2].position,
                    );
                    for fv in &mut face_verts {
                        fv.normal = auto_n;
                    }
                }

                // Triangulación Fan: (0, i, i+1)
                for i in 1..(face_verts.len() - 1) {
                    let tri = [face_verts[0], face_verts[i], face_verts[i + 1]];
                    for v in tri {
                        let key = VertexKey::from_vertex(&v);
                        let idx = *dedup_map.entry(key).or_insert_with(|| {
                            let new_idx = unique_vertices.len() as u32;
                            unique_vertices.push(v);
                            new_idx
                        });
                        indices.push(idx);
                    }
                }
            }
            _ => {}
        }
    }

    write_nxmesh_binary(output_nxmesh, &unique_vertices, &indices)
}

// ============================================================================
// 2. IMPORTADOR DE BLENDER GLTF 2.0 (.GLB / .GLTF -> .NXMESH)
// ============================================================================
pub fn import_gltf_to_nxmesh(input_gltf: &Path, output_nxmesh: &Path) -> Result<(), String> {
    let (document, buffers, _images) = gltf::import(input_gltf)
        .map_err(|e| format!("Error al abrir glTF '{}': {}", input_gltf.display(), e))?;

    let mut unique_vertices: Vec<NxVertex3D> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut dedup_map: HashMap<VertexKey, u32> = HashMap::new();

    for mesh in document.meshes() {
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

            let positions: Vec<[f32; 3]> = match reader.read_positions() {
                Some(iter) => iter.collect(),
                None => continue,
            };

            let normals: Vec<[f32; 3]> = reader
                .read_normals()
                .map(|iter| iter.collect())
                .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]);

            let uvs: Vec<[f32; 2]> = reader
                .read_tex_coords(0)
                .map(|iter| iter.into_f32().collect())
                .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);

            let prim_indices: Vec<u32> = match reader.read_indices() {
                Some(gltf::mesh::util::ReadIndices::U8(iter)) => iter.map(|i| i as u32).collect(),
                Some(gltf::mesh::util::ReadIndices::U16(iter)) => iter.map(|i| i as u32).collect(),
                Some(gltf::mesh::util::ReadIndices::U32(iter)) => iter.collect(),
                None => (0..positions.len() as u32).collect(),
            };

            for raw_idx in prim_indices {
                let i = raw_idx as usize;
                if i >= positions.len() {
                    continue;
                }
                let v = NxVertex3D {
                    position: positions[i],
                    normal: *normals.get(i).unwrap_or(&[0.0, 1.0, 0.0]),
                    uv: *uvs.get(i).unwrap_or(&[0.0, 0.0]),
                };

                let key = VertexKey::from_vertex(&v);
                let deduplicated_idx = *dedup_map.entry(key).or_insert_with(|| {
                    let next = unique_vertices.len() as u32;
                    unique_vertices.push(v);
                    next
                });
                indices.push(deduplicated_idx);
            }
        }
    }

    write_nxmesh_binary(output_nxmesh, &unique_vertices, &indices)
}

// ============================================================================
// 3. IMPORTADOR DE TEXTURAS CON GENERADOR DE MIPMAPS (.PNG / .JPG -> .NXTEX)
// ============================================================================
pub fn import_image_to_nxtex(input_img: &Path, output_nxtex: &Path) -> Result<(), String> {
    let img = image::open(input_img)
        .map_err(|e| format!("Error al decodificar imagen '{}': {}", input_img.display(), e))?;

    let rgba_base = img.to_rgba8();
    let mut current_w = rgba_base.width();
    let mut current_h = rgba_base.height();
    let mut current_pixels: Vec<u8> = rgba_base.into_raw();

    let mut all_mips_payload = Vec::new();
    let mut mip_levels = 0u32;

    // Generar toda la cadena de Mipmaps (Nivel 0 -> Nivel N hasta 1x1) usando Box Filter 2x2
    loop {
        all_mips_payload.extend_from_slice(&current_pixels);
        mip_levels += 1;

        if current_w <= 1 && current_h <= 1 {
            break;
        }

        let next_w = (current_w / 2).max(1);
        let next_h = (current_h / 2).max(1);
        let mut next_pixels = vec![0u8; (next_w * next_h * 4) as usize];

        for y in 0..next_h {
            for x in 0..next_w {
                let sx0 = (x * 2).min(current_w - 1);
                let sy0 = (y * 2).min(current_h - 1);
                let sx1 = (x * 2 + 1).min(current_w - 1);
                let sy1 = (y * 2 + 1).min(current_h - 1);

                let idx00 = ((sy0 * current_w + sx0) * 4) as usize;
                let idx10 = ((sy0 * current_w + sx1) * 4) as usize;
                let idx01 = ((sy1 * current_w + sx0) * 4) as usize;
                let idx11 = ((sy1 * current_w + sx1) * 4) as usize;

                let dst_idx = ((y * next_w + x) * 4) as usize;
                for c in 0..4 {
                    let sum = current_pixels[idx00 + c] as u32
                        + current_pixels[idx10 + c] as u32
                        + current_pixels[idx01 + c] as u32
                        + current_pixels[idx11 + c] as u32;
                    next_pixels[dst_idx + c] = (sum / 4) as u8;
                }
            }
        }

        current_w = next_w;
        current_h = next_h;
        current_pixels = next_pixels;
    }

    let header = NxTexHeader {
        magic: *b"NXT1",
        version: 1,
        width: img.width(),
        height: img.height(),
        mip_levels,
        format: 1, // RGBA8_SRGB con cadena de Mipmaps continua
        payload_checksum: fnv1a_hash_64(&all_mips_payload),
    };

    let mut out_bytes = Vec::with_capacity(std::mem::size_of::<NxTexHeader>() + all_mips_payload.len());
    out_bytes.extend_from_slice(bytemuck::bytes_of(&header));
    out_bytes.extend_from_slice(&all_mips_payload);

    fs::write(output_nxtex, &out_bytes)
        .map_err(|e| format!("Error al escribir '{}': {}", output_nxtex.display(), e))?;

    println!(
        "🖼️  [Texture Importer] '{}' -> {}x{} px ({} niveles Mipmap, {} bytes).",
        output_nxtex.display(),
        header.width,
        header.height,
        header.mip_levels,
        out_bytes.len()
    );

    Ok(())
}

// ============================================================================
// 4. PIPELINE DE IMPORTACIÓN DE CARPETA COMPLETA (RAW_ASSETS -> ASSETS)
// ============================================================================
pub fn import_directory_pipeline(raw_dir: &str, cooked_dir: &str) -> Result<usize, String> {
    let raw_base = Path::new(raw_dir);
    let cooked_base = Path::new(cooked_dir);

    if !raw_base.exists() {
        return Err(format!("La carpeta de arte original '{}' no existe", raw_dir));
    }

    let mut converted_count = 0;
    convert_folder_recursive(raw_base, raw_base, cooked_base, &mut converted_count)?;

    println!(
        "✅ [NX-Asset Pipeline] {} archivos de diseño convertidos a formatos nativos en '{}'.",
        converted_count, cooked_dir
    );
    Ok(converted_count)
}

fn convert_folder_recursive(
    raw_root: &Path,
    current_dir: &Path,
    cooked_root: &Path,
    count: &mut usize,
) -> Result<(), String> {
    for entry in fs::read_dir(current_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path.is_dir() {
            convert_folder_recursive(raw_root, &path, cooked_root, count)?;
            continue;
        }

        let rel = path.strip_prefix(raw_root).map_err(|e| e.to_string())?;
        let dest_parent = cooked_root.join(rel.parent().unwrap_or_else(|| Path::new("")));
        fs::create_dir_all(&dest_parent).map_err(|e| e.to_string())?;

        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("asset");
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "obj" => {
                let out_file = dest_parent.join(format!("{}.nxmesh", stem));
                import_obj_to_nxmesh(&path, &out_file)?;
                *count += 1;
            }
            "gltf" | "glb" => {
                let out_file = dest_parent.join(format!("{}.nxmesh", stem));
                import_gltf_to_nxmesh(&path, &out_file)?;
                *count += 1;
            }
            "png" | "jpg" | "jpeg" => {
                let out_file = dest_parent.join(format!("{}.nxtex", stem));
                import_image_to_nxtex(&path, &out_file)?;
                *count += 1;
            }
            _ => {}
        }
    }
    Ok(())
}
