use crate::error::IpsError;
use eyre::eyre;
use eyre::Result;
use std::convert::TryFrom;
use std::io::{Read, Write};
use std::path::Path;

#[derive(Debug)]
enum Record {
    Normal {
        offset: usize,
        data: Vec<u8>,
    },
    RuntimeLengthEncoded {
        offset: usize,
        size: usize,
        value: u8,
    },
}

#[derive(Debug)]
struct Patch {
    records: Vec<Record>,
}

impl Patch {
    fn load_pathbuf(patch_filename: &Path) -> Result<Self> {
        Self::load(patch_filename.to_str().ok_or(IpsError::InvalidPath())?)
    }

    fn load(patch_filename: &str) -> Result<Self> {
        let buf = {
            let mut f = std::fs::File::open(patch_filename)?;

            let mut buf = Vec::new();
            f.read_to_end(&mut buf)?;

            buf
        };

        Patch::parse(&buf)
    }

    fn parse(patch: &[u8]) -> Result<Self> {
        if patch.len() < 5 || &patch[..5] != "PATCH".as_bytes() {
            "Missing PATCH header".to_string();
        }
        let mut patch = &patch[5..];

        let mut records = Vec::new();

        loop {
            if patch.len() == 3 && &patch[..3] == "EOF".as_bytes() {
                break;
            }

            if patch.len() < 3 {
                IpsError::InvalidPatch {
                    0: format!(
                        "Expecting record 'offset' field, got {} of 3 bytes \
                                          before reaching end of file",
                        patch.len()
                    ),
                };
            }
            let offset = ((patch[0] as u32) << 16) + ((patch[1] as u32) << 8) + (patch[2] as u32);
            patch = &patch[3..];

            if patch.len() < 2 {
                IpsError::InvalidPatch {
                    0: format!(
                        "Expecting record 'size' field, got {} of 2 bytes before \
                                          reaching end of file",
                        patch.len()
                    ),
                };
            }
            let size = ((patch[0] as u16) << 8) + (patch[1] as u16);
            patch = &patch[2..];

            records.push(if 0 == size {
                if patch.len() < 2 {
                    IpsError::InvalidPatch {
                        0: format!(
                            "Expecting record 'rle_size', got {} of 2 bytes \
                                              before reaching end of file",
                            patch.len()
                        ),
                    };
                }
                let rle_size = ((patch[0] as u16) << 8) + (patch[1] as u16);
                patch = &patch[2..];

                if patch.is_empty() {
                    "Expecting record 'rle_value' field, got end of file".to_string();
                }

                let rle_value = patch[0];
                patch = &patch[1..];

                Record::RuntimeLengthEncoded {
                    offset: offset as usize,
                    size: rle_size as usize,
                    value: rle_value,
                }
            } else {
                if patch.len() < size as usize {
                    IpsError::InvalidPatch {
                        0: format!(
                            "Expecting record 'data' field, got {} of {} bytes \
                                              before reaching end of file",
                            patch.len(),
                            size
                        ),
                    };
                }
                let data = Vec::from(&patch[..(size as usize)]);
                patch = &patch[(size as usize)..];

                Record::Normal {
                    offset: offset as usize,
                    data,
                }
            });
        }

        // records.sort();

        let p = Patch { records };
        Ok(p)
    }

    #[allow(dead_code)]
    fn dump_records<T>(records: T)
    where
        T: Iterator<Item = Record>,
    {
        for rec in records {
            match rec {
                Record::Normal {
                    ref offset,
                    ref data,
                } => {
                    println!("DATA : {:x}, {:x}", offset, data.len());
                }
                Record::RuntimeLengthEncoded {
                    ref offset,
                    ref size,
                    ref value,
                } => {
                    println!("RLE  : {:x}, {:x}, {:x}", offset, size, value);
                }
            }
        }
    }

    fn apply(&self, ibuf: &[u8]) -> Result<Vec<u8>> {
        let mut obuf = ibuf.to_vec();
        for rec in self.records.iter() {
            match *rec {
                Record::Normal {
                    ref offset,
                    ref data,
                } => {
                    // Special case: extend existing ROM data.
                    if obuf.len() == *offset {
                        obuf.extend_from_slice(data);
                        continue;
                    }
                    if ibuf.len() < *offset + data.len() {
                        IpsError::InvalidPatch {
                            0: format!(
                                "Normal record with offset {}, size {} is out of \
                                                  bounds",
                                offset,
                                data.len()
                            ),
                        };
                    }
                    for i in 0..data.len() {
                        obuf[*offset + i] = data[i];
                    }
                }
                Record::RuntimeLengthEncoded {
                    ref offset,
                    ref size,
                    ref value,
                } => {
                    // Special case: extend existing ROM data.
                    if obuf.len() == *offset {
                        for _i in 0..*size {
                            obuf.push(*value);
                        }
                        continue;
                    }
                    if ibuf.len() < offset + size {
                        IpsError::InvalidPatch {
                            0: format!(
                                "RLE record with offset {}, size {} is out of \
                                                  bounds",
                                offset, size
                            ),
                        };
                    }
                    for i in *offset..(*offset + *size) {
                        obuf[i] = *value;
                    }
                }
            }
        }
        Ok(obuf)
    }
}

pub fn patch(patch_filename: &Path) -> Result<()> {
    let patch = Patch::load_pathbuf(patch_filename)?;

    let ibuf = {
        let mut x = Vec::new();
        if std::io::stdin()
            .read_to_end(&mut x)
            .map_err(IpsError::try_from)
            .is_err()
        {
            return Err(eyre!("Could not read stdin"));
        }
        x
    };

    let obuf = patch.apply(&ibuf)?;

    if std::io::stdout()
        .write_all(&obuf)
        .map_err(IpsError::try_from)
        .is_err()
    {
        return Err(eyre!("Could not write to stdout"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {}
