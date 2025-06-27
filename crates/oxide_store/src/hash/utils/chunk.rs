use anyhow::Result;
use oxide_core::store::HASH_PART_LEN;
use tokio::{fs::File, io::AsyncReadExt};

// TODO: make this value dynamic based on the available memory
pub const BUFF_SIZE: usize = 128 * 1024;

pub struct ChunkReader {
    file: File,
    buff: Box<[u8; BUFF_SIZE + HASH_PART_LEN - 1]>,
    len: usize,
    offset: u64,
}

impl ChunkReader {
    pub fn new(file: File) -> Self {
        ChunkReader {
            file,
            buff: Box::new([0; BUFF_SIZE + HASH_PART_LEN - 1]),
            len: 0,
            offset: 0,
        }
    }

    pub async fn next(&mut self) -> Result<Option<Chunk>> {
        // copy the tail from the previous iteration
        let tail_start = self.len.saturating_sub(HASH_PART_LEN - 1);
        let tail_len = self.len - tail_start;
        self.buff.copy_within(tail_start..self.len, 0);
        // read the new data
        let n = self
            .file
            .read(&mut self.buff[tail_len..tail_len + 1])
            .await?;
        if n == 0 {
            return Ok(None);
        }
        let offset = self.offset - tail_len as u64;
        self.offset += n as u64;

        self.len = tail_len + n;
        Ok(Some(Chunk {
            chunk: &mut self.buff[..self.len],
            offset,
        }))
    }
}

pub struct Chunk<'a> {
    chunk: &'a mut [u8],
    offset: u64,
}

impl<'a> Chunk<'a> {
    pub fn chunk(&mut self) -> &mut [u8] {
        self.chunk
    }

    pub fn chunk_offset(&self) -> u64 {
        self.offset
    }

    pub fn split_at_overlap(&mut self) -> (&mut [u8], &mut [u8]) {
        let tail_start = self.chunk.len().saturating_sub(HASH_PART_LEN - 1);
        self.chunk.split_at_mut(tail_start)
    }
}
