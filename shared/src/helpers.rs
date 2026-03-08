use pinocchio::error::ProgramError;

pub fn serialize_borsh_string(dest: &mut [u8], s: &str) -> usize {
    let len = s.len() as u32;
    dest[0..4].copy_from_slice(&len.to_le_bytes());
    dest[4..4 + s.len()].copy_from_slice(s.as_bytes());
    4 + s.len()
}

pub fn from_fixed_bytes(bytes: &[u8]) -> Result<&str, ProgramError> {
    let len = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());

    core::str::from_utf8(&bytes[..len]).map_err(|_| ProgramError::Custom(1))
}

pub fn to_fixed_bytes<const N: usize>(s: &str) -> [u8; N] {
    let bytes = s.as_bytes();
    assert!(bytes.len() <= N, "string too long");

    let mut out = [0u8; N];
    out[..bytes.len()].copy_from_slice(bytes);
    out
}
