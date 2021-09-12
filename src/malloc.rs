use std::usize;
use std::mem::size_of;

use crate::syscalls::{BRK, syscall1};

pub struct Header {
    // Used to store size and free flag for optimization.
    internal: usize,
    prev: Block,
    next: Block
}

pub struct Block (*mut Header);

pub struct Data (*mut usize); 

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

    // Creates new header setting the free state as 0 (occupied).
    pub fn new(size: usize) -> Header {
        Header { 
            internal: size,
            prev: Block::null(),
            next: Block::null()
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
        let addr = self as *const Block as usize;
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

    pub fn get_total_size(&self) -> usize {
        self.get_data_size() + 2*size_of::<Header>()
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

    // Because of alignment, total size must be at least 1 word which
    // should have the last bit unused. This is used as free flag
    // 1 = free
    // 0 = occupied
    pub fn is_free(&self) -> bool {
        self.header().get_free_bit() == 1
    }

    pub fn set_free(&self, is_free: bool) {
        let free_bit = is_free as usize;
        self.header().set_free_bit(free_bit);
    }

    // Split block. Occupy, and return the first of the two block.
    // No splitting occurs if `data_size` + (2 * size_of::<Header>()) == size of this block.
    // NOTE: `data_size` doesn't include the size of the header
    pub fn split<'a>(&'a mut self, data_size: usize) -> &'a mut Block {
        let old_total_size = self.get_total_size();
        let new_total_size = data_size + (2 * size_of::<Header>());

        if old_total_size == new_total_size {
            return &mut *self
        }

        let next_block = if self.has_next() { self.next_by_total_size() } else { Block::null() };

        let remaining_ptr = self.0 as usize + new_total_size;
        let remaining_block = Block::from_usize(remaining_ptr);
        let remaining_total_size = old_total_size - new_total_size;
        let remaining_data_size = remaining_total_size - (2 * size_of::<Header>());
        remaining_block.header().set_size(remaining_data_size);
        remaining_block.header().set_free_bit(1);
        remaining_block.header().next = next_block;
        remaining_block.header().prev = Block::from_usize(self.0 as usize);

        self.header().set_size(data_size);
        self.header().set_free_bit(0);
        self.header().next = remaining_block;

        &mut *self
    }
}

enum SearchStrategy {
    FirstFit
}

// Align size to a multiple of machine word.
fn align(size: usize) -> usize {
    (size + (size_of::<usize>() - 1)) & !(size_of::<usize>() - 1)
}

// Search blocks for free spot or return the last block.
// Caller must check the `bool == true` flag if it found spot, otherwise `Block` is the last block.
fn search_free_spot_or_last(size: usize, search_strategy: SearchStrategy) -> (Block, bool) {
    match search_strategy {
        SearchStrategy::FirstFit => search_first_fit(size)
    }
}

fn search_first_fit(size: usize) -> (Block, bool) {
    let mut current = unsafe { &root };
    let mut found = false;

    while current.has_next() {
        current = current.next();
        if current.is_free() && current.get_total_size() >= size {
            found = true;
            break;
        }
    }

    (Block::from_usize(current.0 as usize), found)
}

pub static mut root: Block = Block(0 as *mut Header);
static mut current_brk: *mut usize = 0 as *mut usize;

unsafe fn brk(end_data_segment: *mut usize) -> *mut isize {
    let new = syscall1(BRK, end_data_segment as usize);
    // println!("eds {:?} new {:?} newisize {:?} asas {:?}", end_data_segment as usize, new, new as isize, new as isize as *mut isize as usize);
    current_brk = new as *mut usize;
    // allow brk to set over end, also handles 0
    (if new < (end_data_segment as usize) {-1} else {new as isize}) as *mut isize
}

unsafe fn sbrk(increment: usize) -> *mut isize {
    if current_brk as usize == 0 {
        let temp = brk(0 as *mut usize) as usize;
        // println!("if {:?}", temp);
    }
    let new = current_brk as usize + increment;
    let res = brk(new as *mut usize);
    // println!("res {:?}", res as usize);
    res
}

pub fn init_malloc() {
    unsafe {
        // println!("C1");
        let current = brk(0 as *mut usize);
        // println!("current {:?}", current as usize);
        root = Block::from_usize(current as usize);
        let after = sbrk(size_of::<Header>());
        // println!("after {:?}", after as usize);
        *root.header() = Header::new(0);
        // println!("C4");
    }
}

pub fn malloc(size: usize) -> *mut usize {
    assert!(size > 0);

    let current_root = unsafe { &root };
    if current_root.is_null() {
        init_malloc();
    }

    let aligned_size = align(size);
    let total_size = aligned_size + (2 * size_of::<Header>());

    let (mut block, found) = search_free_spot_or_last(total_size, SearchStrategy::FirstFit);

    if found {
        // `block` can be reuse
        let new = block.split(total_size);

        new.data().unwrap().0
    } else {
        // `block` is the last Block, allocate new memory
        let current = unsafe { sbrk(0) as *mut usize };
        // println!("total_size {:?}", total_size);
        let after = unsafe { sbrk(total_size) as *mut usize };
        // println!("after {:?}", after as usize);

        let new = Block::from_usize(current as usize);
        unsafe {
            *new.0 = Header {
                internal: aligned_size,
                prev: Block::from_usize(block.0 as usize),
                next: Block::null()
            };
        }
        
        block.header().next = Block::from_usize(current as usize);

        new.data().unwrap().0
    }
}

#[cfg(test)]
mod tests {
    use std::mem;
    use crate::malloc::Block;
    use crate::malloc::align;
    use crate::malloc::init_malloc;

    use super::Header;
    use super::brk;
    use super::malloc;

    #[test]
    fn test_block_size() {
        assert_eq!(8, mem::size_of::<Block>());
    }

    #[test]
    fn test_align() {
        // test only applies on x86_64
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
    fn test_block_from_usize() {
        let header = Header::new(16);

        let ptr = &header as *const Header as usize;
        let block = Block::from_usize(ptr);

        assert_eq!(block.header().get_size(), 16);
    }

    #[test]
    fn test_block_header() {
        let header = Header::new(16);

        let block = Block(&header as *const Header as *mut Header);
        assert_eq!(16, block.header().get_size());
    }

    #[test]
    fn test_malloc() {
        use std::mem::size_of;
        let total_size = |data| -> usize {
            (2 * size_of::<Header>()) + align(data)
        };

        init_malloc();

        let initial_brk = unsafe { brk(0 as *mut usize) as usize };
        println!("current_brk {:?}", initial_brk);

        let requests = [3, 4, 8, 13, 28, 321];
        let mut counter_size = 0;
        for request in requests {
            malloc(request);
            counter_size += total_size(request);
        }

        let final_brk = unsafe { brk(0 as *mut usize) as usize };
        println!("final_brk {:?}", final_brk);

        assert_eq!(initial_brk + counter_size, final_brk);
    }
}