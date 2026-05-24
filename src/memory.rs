#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DomainId {
    pub high: u64,
    pub low: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MemoryHandle {
    pub abi_version: u32,
    pub flags: u32,
    pub domain_id: DomainId,
    pub offset: u64,
    pub byte_size: u64,
    pub schema_hash: u64,
    pub generation: u64,
    pub type_fqn: [u8; 8],
}
