//! Memory slab allocator.
//!
//! The slab manages a preallocated region of memory divided into fixed-size
//! blocks. Each block stores metadata and the key/value payload:
//!
//! ```text
//! [ TTL (8 bytes) ][ KeyLen (2 bytes) ][ ValLen (2 bytes) ][ Key ][ Value ]
//! ```
//!
//! All offsets are encoded manually using byte operations to keep the
//! allocator lock-free and allocation-free after initialisation.

use crate::Handle;
use core::convert::TryInto;

/// Memory slab allocator using a simple freelist.
pub struct Slab {
    /// Contiguous region backing the slab.
    region: Box<[u8]>,
    /// Total number of blocks in the region.
    total_blocks: usize,
    /// Indices of currently free blocks.
    free_list: Vec<usize>,
}

// Layout constants ---------------------------------------------------------
const TTL_OFFSET: usize = 0; // start of TTL field
const TTL_SIZE: usize = 8; // u64
const KEY_LEN_OFFSET: usize = TTL_OFFSET + TTL_SIZE; // 8
const VAL_LEN_OFFSET: usize = KEY_LEN_OFFSET + 2; // 10
const HEADER_SIZE: usize = VAL_LEN_OFFSET + 2; // 12
/// Fixed block size used by this allocator.
pub(crate) const BLOCK_SIZE: usize = 512; // bytes

/// Initialise a freelist containing `n` block indices in LIFO order.
fn init_freelist(n: usize) -> Vec<usize> {
    let mut list = Vec::with_capacity(n);
    for idx in (0..n).rev() {
        list.push(idx);
    }
    list
}

impl Slab {
    /// Create a new slab capable of storing at most `capacity_bytes` of data.
    ///
    /// Memory is divided into 512-byte blocks; the total capacity is truncated
    /// to a multiple of the block size.
    pub fn new(capacity_bytes: usize) -> Self {
        let total_blocks = capacity_bytes / BLOCK_SIZE;
        let region = vec![0u8; total_blocks * BLOCK_SIZE].into_boxed_slice();

        let free_list = init_freelist(total_blocks);

        Self {
            region,
            total_blocks,
            free_list,
        }
    }

    /// Allocate a block for the provided key/value pair and TTL.
    ///
    /// Returns a [`Handle`] to the allocated block or `None` if the slab is full
    /// or the data exceeds the block size.
    pub fn allocate(&mut self, key: &[u8], value: &[u8], ttl: u64) -> Option<Handle> {
        // Ensure lengths fit in our fixed block.
        if key.len() > u16::MAX as usize || value.len() > u16::MAX as usize {
            return None;
        }
        let required = HEADER_SIZE + key.len() + value.len();
        if required > BLOCK_SIZE {
            return None;
        }

        let index = self.free_list.pop()?;
        let offset = index * BLOCK_SIZE;
        let block = &mut self.region[offset..offset + BLOCK_SIZE];

        // Encode TTL (u64 LE).
        block[TTL_OFFSET..TTL_OFFSET + TTL_SIZE].copy_from_slice(&ttl.to_le_bytes());

        // Encode key and value lengths (u16 LE).
        let key_len = key.len() as u16;
        let val_len = value.len() as u16;
        block[KEY_LEN_OFFSET..KEY_LEN_OFFSET + 2].copy_from_slice(&key_len.to_le_bytes());
        block[VAL_LEN_OFFSET..VAL_LEN_OFFSET + 2].copy_from_slice(&val_len.to_le_bytes());

        // Copy key bytes directly after the header.
        let key_start = HEADER_SIZE;
        let key_end = key_start + key.len();
        block[key_start..key_end].copy_from_slice(key);

        // Copy value bytes immediately after key bytes.
        let val_start = key_end;
        let val_end = val_start + value.len();
        block[val_start..val_end].copy_from_slice(value);

        Some(Handle(index))
    }

    /// Retrieve the value slice stored for `handle`.
    ///
    /// The returned slice points directly into the slab's backing region. TTL
    /// and key can be recovered by interpreting the header and key bytes if
    /// needed.
    pub fn get_value(&self, handle: Handle) -> Option<&[u8]> {
        let index = handle.0;
        if index >= self.total_blocks {
            return None;
        }
        let offset = index * BLOCK_SIZE;
        let block = &self.region[offset..offset + BLOCK_SIZE];

        // Decode key and value lengths to determine the slice boundaries.
        let key_len_bytes = &block[KEY_LEN_OFFSET..KEY_LEN_OFFSET + 2];
        let val_len_bytes = &block[VAL_LEN_OFFSET..VAL_LEN_OFFSET + 2];
        let key_len = u16::from_le_bytes([key_len_bytes[0], key_len_bytes[1]]) as usize;
        let val_len = u16::from_le_bytes([val_len_bytes[0], val_len_bytes[1]]) as usize;

        let val_start = HEADER_SIZE + key_len;
        let val_end = val_start + val_len;
        if val_end > BLOCK_SIZE {
            return None;
        }

        Some(&block[val_start..val_end])
    }

    /// Retrieve all metadata and payload for `handle`.
    pub fn get_meta(&self, handle: Handle) -> Option<(u64, &[u8], &[u8])> {
        let index = handle.0;
        if index >= self.total_blocks {
            return None;
        }
        let offset = index * BLOCK_SIZE;
        let block = &self.region[offset..offset + BLOCK_SIZE];

        // Extract TTL.
        let ttl_bytes = &block[TTL_OFFSET..TTL_OFFSET + TTL_SIZE];
        let ttl = u64::from_le_bytes(ttl_bytes.try_into().ok()?);

        // Extract lengths.
        let key_len_bytes = &block[KEY_LEN_OFFSET..KEY_LEN_OFFSET + 2];
        let val_len_bytes = &block[VAL_LEN_OFFSET..VAL_LEN_OFFSET + 2];
        let key_len = u16::from_le_bytes([key_len_bytes[0], key_len_bytes[1]]) as usize;
        let val_len = u16::from_le_bytes([val_len_bytes[0], val_len_bytes[1]]) as usize;

        let key_start = HEADER_SIZE;
        let key_end = key_start + key_len;
        let val_start = key_end;
        let val_end = val_start + val_len;
        if val_end > BLOCK_SIZE {
            return None;
        }

        let key = &block[key_start..key_end];
        let value = &block[val_start..val_end];

        Some((ttl, key, value))
    }

    /// Deallocate the block referenced by `handle` and return it to the freelist.
    pub fn deallocate(&mut self, handle: Handle) {
        let index = handle.0;
        if index >= self.total_blocks {
            return;
        }
        assert!(
            !self.free_list.contains(&index),
            "block already freed"
        );
        let offset = index * BLOCK_SIZE;
        // Zero out the block for predictability; keeps hot path free of mallocs.
        self.region[offset..offset + BLOCK_SIZE].fill(0);
        self.free_list.push(index);
    }

    /// Dump the raw contents of the block for debugging purposes.
    pub fn debug_dump(&self, handle: Handle) -> Option<String> {
        let index = handle.0;
        if index >= self.total_blocks {
            return None;
        }
        let offset = index * BLOCK_SIZE;
        let block = &self.region[offset..offset + BLOCK_SIZE];
        let mut out = String::new();
        for (i, chunk) in block.chunks(16).enumerate() {
            let hex: String = chunk.iter().map(|b| format!("{:02x} ", b)).collect();
            let ascii: String = chunk
                .iter()
                .map(|&b| if b.is_ascii_graphic() || b == b' ' { b as char } else { '.' })
                .collect();
            out.push_str(&format!("{:04x}: {:<48} {}\n", i * 16, hex, ascii));
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_get_deallocate_round_trip() {
        let mut slab = Slab::new(1024); // 2 blocks
        let handle = slab.allocate(b"key", b"value", 1).expect("allocation");
        let val = slab.get_value(handle).expect("get");
        assert_eq!(val, b"value");

        let (ttl, key, value) = slab.get_meta(handle).expect("meta");
        assert_eq!(ttl, 1);
        assert_eq!(key, b"key");
        assert_eq!(value, b"value");

        slab.deallocate(handle);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            slab.deallocate(handle)
        }));
        assert!(result.is_err(), "double free should panic");
        assert_eq!(slab.free_list.len(), slab.total_blocks);
    }
}
