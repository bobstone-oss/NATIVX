use bytemuck::{Pod, Zeroable};
use std::fs;
use std::path::Path;

pub const NXPAK_MAGIC: [u8; 4] = *b"NXPK";
pub const NXPAK_VERSION: u32 = 1;
pub const NXPAK_ALIGNMENT: usize = 64;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct NxAssetHeader {
    pub magic: [u8; 4],
    pub version: u32,
    pub vertex_count: u32,
    pub index_count: u32,
    pub checksum: u64,
    pub _reserved: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct NxVertex3D {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

pub struct MappedMeshView<'a> {
    pub header: &'a NxAssetHeader,
    pub vertices: &'a [NxVertex3D],
    pub indices: &'a [u32],
}

impl<'a> MappedMeshView<'a> {
    pub fn from_raw_bytes(bytes: &'a [u8]) -> Result<Self, &'static str> {
        let h_size = core::mem::size_of::<NxAssetHeader>();
        if bytes.len() < h_size { return Err("Archivo .nxmesh truncado"); }
        let header: &NxAssetHeader = bytemuck::from_bytes(&bytes[0..h_size]);
        if &header.magic != b"NXM1" { return Err("Firma .nxmesh inválida"); }

        let v_bytes = (header.vertex_count as usize) * core::mem::size_of::<NxVertex3D>();
        let i_bytes = (header.index_count as usize) * core::mem::size_of::<u32>();
        let v_end = h_size + v_bytes;
        let i_end = v_end + i_bytes;
        if bytes.len() < i_end { return Err("Payload .nxmesh incompleto"); }

        Ok(Self {
            header,
            vertices: bytemuck::cast_slice(&bytes[h_size..v_end]),
            indices: bytemuck::cast_slice(&bytes[v_end..i_end]),
        })
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct NxPakHeader {
    pub magic: [u8; 4],
    pub version: u32,
    pub entry_count: u32,
    pub flags: u32,
    pub toc_offset: u64,
    pub data_offset: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct NxPakEntry {
    pub path_hash: u64,
    pub asset_kind: u32,
    pub flags: u32,
    pub offset: u64,
    pub size: u64,
    pub payload_checksum: u64,
    pub debug_name: [u8; 24],
}

#[inline(always)]
pub const fn fnv1a_hash_64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        i += 1;
    }
    hash
}

pub fn hash_asset_path(path: &str) -> u64 {
    let norm = path.trim_start_matches("./").replace('\\', "/");
    fnv1a_hash_64(norm.as_bytes())
}

pub struct NxPakArchive<'a> {
    raw_bytes: &'a [u8],
    header: &'a NxPakHeader,
    entries: &'a [NxPakEntry],
}

impl<'a> NxPakArchive<'a> {
    pub fn mount(raw_bytes: &'a [u8]) -> Result<Self, &'static str> {
        let h_size = core::mem::size_of::<NxPakHeader>();
        if raw_bytes.len() < h_size { return Err("Cabecera .nxpak inválida"); }
        let header: &NxPakHeader = bytemuck::from_bytes(&raw_bytes[0..h_size]);
        if header.magic != NXPAK_MAGIC { return Err("Magic NXPK inválido"); }

        let toc_start = header.toc_offset as usize;
        let toc_end = toc_start + (header.entry_count as usize) * core::mem::size_of::<NxPakEntry>();
        let entries: &[NxPakEntry] = bytemuck::cast_slice(&raw_bytes[toc_start..toc_end]);
        Ok(Self { raw_bytes, header, entries })
    }

    pub fn file_count(&self) -> usize { self.header.entry_count as usize }
    pub fn entries(&self) -> &'a [NxPakEntry] { self.entries }

    pub fn get_bytes(&self, path: &str) -> Result<&'a [u8], &'static str> {
        let target = hash_asset_path(path);
        let idx = self.entries.binary_search_by_key(&target, |e| e.path_hash).map_err(|_| "No encontrado")?;
        let e = &self.entries[idx];
        Ok(&self.raw_bytes[e.offset as usize..(e.offset + e.size) as usize])
    }

    pub fn load_mesh_zero_copy(&self, path: &str) -> Result<MappedMeshView<'a>, &'static str> {
        MappedMeshView::from_raw_bytes(self.get_bytes(path)?)
    }
}

pub fn write_struct_instant<T: Pod, P: AsRef<Path>>(path: P, state: &T) -> Result<(), String> {
    fs::write(path, bytemuck::bytes_of(state)).map_err(|e| e.to_string())
}
