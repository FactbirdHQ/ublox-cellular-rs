#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FromHexError {
    /// An invalid character was found. Valid ones are: `0...9`, `a...f`
    /// or `A...F`.
    InvalidHexCharacter,

    /// A hex string's length needs to be even, as two digits correspond to
    /// one byte.
    OddLength,
}

fn val(c: u8) -> Result<u8, FromHexError> {
    match c {
        b'A'..=b'F' => Ok(c - b'A' + 10),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'0'..=b'9' => Ok(c - b'0'),
        _ => Err(FromHexError::InvalidHexCharacter),
    }
}

pub fn from_hex<'a>(hex: &'a mut [u8]) -> Result<&'a [u8], FromHexError> {
    if hex.len() % 2 != 0 {
        return Err(FromHexError::OddLength);
    }

    let len = hex.len() / 2;
    for i in 0..len {
        hex[i] = val(hex[i * 2])? << 4 | val(hex[i * 2 + 1])?
    }
    Ok(&hex[..len])
}
