use std::mem::size_of;

#[repr(C)]
pub struct Header {
    // Used to store size and free flag for optimization.
    internal: usize,
    prev: Block,
    next: Block,
}

pub struct Block(pub *mut Header);

pub struct Data(pub *mut usize);

impl Header {
    pub fn get_size(&self) -> usize {
        // & 1...1111000 on x86_64 system
        self.internal & !(size_of::<usize>() - 1)
    }

    pub fn set_size(&mut self, size: usize) {
        self.internal = size | self.get_free_bit();
    }

    pub fn get_free_bit(&self) -> usize {
        self.internal & 1
    }

    pub fn set_free_bit(&mut self, free_bit: usize) {
        self.internal = self.get_size() | free_bit;
    }

    pub fn set_next(&mut self, block: Block) {
        self.next = block;
    }

    pub fn set_prev(&mut self, block: Block) {
        self.prev = block;
    }

    // Creates new header setting the free state as 0 (occupied).
    pub fn from_usize(size: usize) -> Header {
        Header::new(size, Block::null(), Block::null())
    }

    fn new(internal: usize, prev: Block, next: Block) -> Header {
        Header {
            internal: internal,
            prev: prev,
            next: next,
        }
    }
}

impl Block {
    #[inline(always)]
    pub fn is_null(&self) -> bool {
        self.0 as usize == 0
    }

    pub fn null() -> Block {
        Block(0 as *mut Header)
    }

    #[inline(always)]
    pub fn from_usize(u: usize) -> Block {
        Block(u as *mut Header)
    }

    #[inline(always)]
    pub fn header<'a>(&'a self) -> &'a mut Header {
        unsafe { &mut *self.0 }
    }

    pub fn next_by_total_size(&self) -> Block {
        let addr = self.0 as usize;
        let next_addr = addr + self.get_total_size();
        Block::from_usize(next_addr)
    }

    pub fn has_next(&self) -> bool {
        !self.header().next.is_null()
    }

    pub fn next<'a>(&'a self) -> &'a mut Block {
        &mut self.header().next
    }

    pub fn has_prev(&self) -> bool {
        !self.header().prev.is_null()
    }

    pub fn prev<'a>(&'a self) -> &'a mut Block {
        &mut self.header().prev
    }

    pub fn get_data_size(&self) -> usize {
        self.header().get_size()
    }

    pub fn get_total_padding() -> usize {
        size_of::<Header>()
    }

    pub fn get_total_size(&self) -> usize {
        self.get_data_size() + Self::get_total_padding()
    }

    pub fn data(&self) -> Option<Data> {
        match self.header().get_size() {
            0 => None,
            _ => {
                let data_ptr = self.0 as usize + size_of::<Header>();
                Some(Data(data_ptr as *mut usize))
            }
        }
    }

    /// Because of alignment, total size must be at least 1 word which
    /// should have the last bit unused. This is used as free flag
    /// 1 = free
    /// 0 = occupied
    pub fn is_free(&self) -> bool {
        self.header().get_free_bit() == 1
    }

    pub fn set_free(&self, is_free: bool) {
        let free_bit = is_free as usize;
        self.header().set_free_bit(free_bit);
    }

    /// Split block if necessary. Occupy, and return the first of the two block.
    /// NOTE: `data_size` doesn't include the size of the header
    pub fn split<'a>(&'a mut self, data_size: usize) -> &'a mut Block {
        let old_total_size = self.get_total_size();
        let new_total_size = data_size + Block::get_total_padding();

        // don't split if it's unnecessary!!! (e.g. only creating header on the next block)
        if old_total_size == new_total_size
            || old_total_size - new_total_size <= Block::get_total_padding()
        {
            self.header().set_free_bit(0);
            return &mut *self;
        }

        let next_block = if self.has_next() {
            self.next_by_total_size()
        } else {
            Block::null()
        };

        let remaining_ptr = self.0 as usize + new_total_size;
        let remaining_block = Block::from_usize(remaining_ptr);
        let remaining_total_size = old_total_size - new_total_size;
        let remaining_data_size = remaining_total_size - Block::get_total_padding();
        remaining_block.header().set_size(remaining_data_size);
        remaining_block.header().set_free_bit(1);
        remaining_block.header().next = next_block;
        remaining_block.header().prev = Block::from_usize(self.0 as usize);

        self.header().set_size(data_size);
        self.header().set_free_bit(0);
        self.header().next = remaining_block;

        &mut *self
    }

    /// Attempts to join next and previous block if it's free.
    /// This function is called after every `free` in order to look forward / backward only once.
    pub fn coalesce(&self) {
        if self.has_next() && self.next().is_free() {
            let next = self.next();
            self.header()
                .set_size(self.header().get_size() + next.get_total_size());

            if next.has_next() {
                self.header().next = Block::from_usize(next.next().0 as usize);
                let nn = next.next();
                nn.header().prev = Block::from_usize(next.0 as usize);
            } else {
                self.header().next = Block::null();
            }
        }

        if self.has_prev() && self.prev().is_free() {
            let prev = self.prev();
            prev.header()
                .set_size(prev.get_data_size() + self.get_total_size());

            if self.has_next() {
                prev.header().next = Block::from_usize(self.next().0 as usize);
                let next = self.next();
                next.header().prev = Block::from_usize(prev.0 as usize);
            } else {
                prev.header().next = Block::null();
            }
        }
    }
}

impl Data {
    pub fn get_block(&self) -> Block {
        Block::from_usize(self.0 as usize - size_of::<Header>())
    }
}

#[cfg(test)]
mod tests {
    use std::mem;

    use crate::malloc::types::{Block, Header};

    #[test]
    fn test_block_size() {
        assert_eq!(8, mem::size_of::<Block>());
    }

    #[test]
    fn test_block_from_usize() {
        let header = Header::from_usize(16);

        let ptr = &header as *const Header as usize;
        let block = Block::from_usize(ptr);

        assert_eq!(block.header().get_size(), 16);
    }

    #[test]
    fn test_block_header() {
        let header = Header::from_usize(16);

        let block = Block(&header as *const Header as *mut Header);
        assert_eq!(16, block.header().get_size());
    }
}
