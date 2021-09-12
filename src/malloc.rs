use std::usize;
use std::mem::size_of;

use crate::syscalls::{BRK, syscall1};

pub struct Header {
    // used to store size and free flag for optimization
    internal: usize
}

pub struct Block (*mut Header);

pub struct Data (*mut usize); 

impl Header {
    pub fn get_size(&self) -> usize {
        self.internal & !(size_of::<usize>() - 1)
    }

    pub fn get_free_bit(&self) -> usize {
        self.internal & 1
    }

    pub fn set_free_bit(&mut self, free_bit: usize) {
        self.internal = self.get_size() | free_bit;
    }

    pub fn new(size: usize) -> Header {
        Header { internal: size }
    }
}

impl Block {
    #[inline(always)]
    pub fn is_null(&self) -> bool {
        self.0 as usize == 0
    }

    #[inline(always)]
    pub fn from_ptr(ptr: usize) -> Block {
        Block(ptr as *mut Header)
    }

    #[inline(always)]
    pub fn header<'a>(&'a self) -> &'a mut Header {
        unsafe { &mut *self.0 }
    }

    #[inline(always)]
    pub fn next_block_ptr(&self) -> usize {
        let ptr = self as *const Block as usize;
        ptr + size_of::<Header>() + self.header().get_size()
    }

    pub fn next_block(&self) -> Block {
        Block::from_ptr(self.next_block_ptr())
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

    pub fn is_free(&self) -> bool {
        // because of alignment, total size must be at least 1 word which
        // should have the last bit unused. This is used as free flag
        // 1 = free
        // 0 = occupied
        self.header().get_free_bit() == 1
    }

    pub fn set_free(&self, is_free: bool) {
        let free_bit = is_free as usize;
        self.header().set_free_bit(free_bit);
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

static mut root: Block = Block(0 as *mut Header);
static mut current_brk: *mut usize = 0 as *mut usize;

unsafe fn brk(end_data_segment: *mut usize) -> *mut isize {
    let new = syscall1(BRK, end_data_segment as usize);
    println!("eds {:?} new {:?} newisize {:?} asas {:?}", end_data_segment as usize, new, new as isize, new as isize as *mut isize as usize);
    current_brk = new as *mut usize;
    // allow brk to set over end, also handles 0
    (if new < (end_data_segment as usize) {-1} else {new as isize}) as *mut isize
}

unsafe fn sbrk(increment: usize) -> *mut isize {
    if current_brk as usize == 0 {
        println!("if {:?}", brk(0 as *mut usize) as usize);
    }
    let new = current_brk as usize + increment;
    let res = brk(new as *mut usize);
    println!("res {:?}", res as usize);
    res
}

pub fn init_malloc() {
    unsafe {
        println!("C1");
        let current = brk(0 as *mut usize);
        println!("current {:?}", current as usize);
        root = Block::from_ptr(current as usize);
        let after = sbrk(size_of::<Header>());
        println!("after {:?}", after as usize);
        *root.header() = Header::new(0);
        println!("C4");
    }
}

pub fn malloc(size: usize) -> *mut usize {
    let total_size = align(size_of::<Header>() + size);

    let current_root = unsafe { &root };
    if (current_root.is_null()) {
        init_malloc();
    }

    let spot: Option<*mut Block> = search_free_spot(total_size, SearchStrategy::FirstFit);

    match spot {
        None => {
            // allocate new memory
            let current = unsafe { sbrk(0) };
            let after = unsafe { sbrk(total_size) };


            0 as *mut usize
        },
        _ => {
            // reuse block
            0 as *mut usize
        }
    }
}

#[cfg(test)]
mod tests {
    use std::mem;
    use crate::malloc::Block;
    use crate::malloc::align;

    use super::Header;
    use super::malloc;

    #[test]
    fn test_block_size() {
        assert_eq!(8, mem::size_of::<Block>());
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
        let header = Header::new(16);

        let ptr = &header as *const Header as usize;
        let block = Block::from_ptr(ptr);

        assert_eq!(block.header().get_size(), 16);
    }

    #[test]
    fn test_block_header() {
        let header = Header::new(16);

        let block = Block(&header as *const Header as *mut Header);
        assert_eq!(16, block.header().get_size());
    }

    #[test]
    fn test_block_next_block() {
        let header = Header::new(8);

        let block = Block(&header as *const Header as *mut Header);
        let next_block_ptr = block.next_block_ptr() as *mut Header;
        unsafe {
            *next_block_ptr = Header::new(16);
        }
        let next_block = Block(next_block_ptr);
        assert_eq!(block.next_block().header().get_size(), next_block.header().get_size())
    }

    #[test]
    fn test_malloc() {
        malloc(16);
    }
}