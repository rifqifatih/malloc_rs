use std::usize;
use std::mem::size_of;

pub struct Header {
    size: usize
}

pub struct Block (*mut Header);

pub struct Data (*mut usize); 

impl Block {
    #[inline(always)]
    pub fn from_ptr(ptr: usize) -> Block {
        Block(ptr as *mut Header)
    }

    #[inline(always)]
    pub fn header<'a>(&'a self) -> &'a mut Header {
        unsafe { &mut *self.0 }
    }

    pub fn next_block(&self) -> Block {
        let ptr = self as *const Block as usize;
        let next = ptr + size_of::<Header>() + self.header().size;
        Block::from_ptr(next)
    }
}

enum SearchStrategy {
    FirstFit
}

// align size to a multiple of machine word 
fn align(size: usize) -> usize {
    (size + (size_of::<usize>() - 1)) & !(size_of::<usize>() - 1)
}

fn search_free_spot(size: usize, search_strategy: SearchStrategy) -> Option<*mut Block> {
    match search_strategy {
        SearchStrategy::FirstFit => search_first_fit(size)
    }
}

fn search_first_fit(size: usize) -> Option<*mut Block> {
    None
}

pub fn malloc(size: usize) {
    let total_size = align(size);

    let spot: Option<*mut Block> = search_free_spot(total_size, SearchStrategy::FirstFit);
}

#[cfg(test)]
mod tests {
    use std::mem;
    use crate::malloc::Block;
    use crate::malloc::align;

    use super::Header;

    #[test]
    fn test_block_size() {
        assert_eq!(16, mem::size_of::<Block>());
    }

    #[test]
    fn test_align() {
        assert_eq!(0, align(0));
        assert_eq!(8, align(1));
        assert_eq!(8, align(7));
        assert_eq!(8, align(8));
        assert_eq!(16, align(9));
        assert_eq!(16, align(15));
        assert_eq!(16, align(16));
        assert_eq!(24, align(17));
    }

    #[test]
    fn test_block_from_ptr() {
        let header = Header {
            size: 16
        };

        let ptr = &header as *const Header as usize;
        let block = Block::from_ptr(ptr);

        assert_eq!(block.header().size, 16);
    }

    #[test]

    fn test_block_header() {
        let header = Header {
            size: 16
        };

        let block = Block(&header as *const Header as *mut Header);
        assert_eq!(16, block.header().size);
    }
}