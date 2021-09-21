use std::env;
use std::{sync::Mutex, usize};
use std::mem::size_of;
use lazy_static::lazy_static;

use self::{syscalls::{BRK, syscall1}, types::{Block, Header, Data}};

mod types;
mod syscalls;

static mut ROOT: Block = Block(0 as *mut Header);
static mut CURRENT_BRK: *mut usize = 0 as *mut usize;
lazy_static! {
    static ref MUTEX: Mutex<i32> = Mutex::new(0);
}

#[allow(dead_code)]
enum SearchStrategy {
    FirstFit,
    BestFit
}

/// Align size to a multiple of machine word.
///
/// E.g. on x86_64:
///
/// - 5 -> 8
/// - 8 -> 8
/// - 9 -> 16
fn align(size: usize) -> usize {
    (size + (size_of::<usize>() - 1)) & !(size_of::<usize>() - 1)
}

/// Search blocks for free spot or return the last block.
/// Caller must check the `bool == true` flag if it found spot, otherwise `Block` is the last block.
fn search_free_spot_or_last(size: usize) -> (Block, bool) {
    let args: Vec<String> = env::args().collect();

    let search_strategy = match args.len() {
        0..=3 => SearchStrategy::FirstFit,
        _ => match args[3].as_str() {
            "BEST_FIT" => SearchStrategy::BestFit,
            "FIRST_FIT" => SearchStrategy::FirstFit,
            _ => SearchStrategy::FirstFit 
        }
    };

    match search_strategy {
        SearchStrategy::FirstFit => search_first_fit(size),
        SearchStrategy::BestFit => search_best_fit(size)
    }
}

/// Search blocks consecutively and returns the first one fits.
/// In case no block fits, `bool` is false, and `Block` is the last block.
fn search_first_fit(size: usize) -> (Block, bool) {
    let mut current = unsafe { &ROOT };
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

/// Search blocks consecutively and returns the minimum-sized block that still fits `size`.
fn search_best_fit(size: usize) -> (Block, bool) {
    let mut current = unsafe { &ROOT };
    let mut found = false;
    let mut min = usize::MAX;
    let mut min_block = current;

    while current.has_next() {
        current = current.next();
        let current_total_size = current.get_total_size();
        if current.is_free() && current_total_size >= size && current_total_size < min {
            found = true;
            min = current_total_size;
            min_block = current;
        }
    }

    if found { 
        (Block::from_usize(min_block.0 as usize), found)
    } else {
        (Block::from_usize(current.0 as usize), found)
    }
}

unsafe fn brk(end_data_segment: *mut usize) -> *mut isize {
    let new = syscall1(BRK, end_data_segment as usize);
    CURRENT_BRK = new as *mut usize;
    // allow brk to set over end, also handles 0
    (if new < (end_data_segment as usize) {-1} else {new as isize}) as *mut isize
}

unsafe fn sbrk(increment: usize) -> *mut isize {
    if CURRENT_BRK as usize == 0 {
        brk(0 as *mut usize);
    }
    let new = CURRENT_BRK as usize + increment;
    let res = brk(new as *mut usize);
    res
}

fn init_malloc() {
    unsafe {
        let current = brk(0 as *mut usize);
        ROOT = Block::from_usize(current as usize);
        sbrk(size_of::<Header>());
        *ROOT.header() = Header::from_usize(0);
    }
}

pub fn malloc(size: usize) -> *mut usize {
    assert!(size > 0);
    let lock = MUTEX.lock().unwrap();

    let current_root = unsafe { &ROOT };
    if current_root.is_null() {
        init_malloc();
    }

    let aligned_size = align(size);
    let total_size = aligned_size + Block::get_total_padding();

    let (mut block, found) = search_free_spot_or_last(total_size);

    let res = 
    if found {
        // `block` can be reuse
        // println!("split total_size {:?}", total_size);
        let new = block.split(aligned_size);

        new.data().unwrap().0
    } else {
        // `block` is the last Block, allocate new memory
        // println!("allocate total_size {:?}", total_size);
        let current = unsafe { sbrk(0) as *mut usize };
        unsafe { sbrk(total_size) };

        let new = Block::from_usize(current as usize);
        unsafe {
            *new.0 = Header::from_usize(aligned_size);
            new.header().set_prev(Block::from_usize(block.0 as usize));
        }
        block.header().set_next(Block::from_usize(current as usize));

        new.data().unwrap().0
    };

    Mutex::unlock(lock);
    res
}

pub fn free(ptr: *mut usize) {
    assert!(ptr as usize != 0);
    let lock = MUTEX.lock().unwrap();

    let block = Data(ptr).get_block();
    block.set_free(true);
    block.coalesce();
    Mutex::unlock(lock);
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use lazy_static::lazy_static;
    use crate::malloc::align;
    use crate::malloc::init_malloc;

    use super::brk;
    use super::malloc;
    use super::free;

    // TODO find a better way.
    // Currently malloc tests requires to run in sequence (w.r.t ALL test) to keep track of memory using brk
    lazy_static! {
        static ref MUTEX: Mutex<i32> = Mutex::new(0);
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
    fn test_malloc() {
        let lock = MUTEX.lock().unwrap();
        init_malloc();
        let total_size = |data| -> usize {
            crate::malloc::Block::get_total_padding() + align(data)
        };

        let initial_brk = unsafe { brk(0 as *mut usize) as usize };
        println!("initial_brk {:?}", initial_brk);

        let requests = [3, 4, 8, 13, 28, 321];
        let mut counter_size = 0;
        for request in requests {
            malloc(request);
            counter_size += total_size(request);
        }

        let final_brk = unsafe { brk(0 as *mut usize) as usize };
        println!("final_brk {:?}", final_brk);

        assert_eq!(initial_brk + counter_size, final_brk);
        Mutex::unlock(lock);
    }

    #[test]
    fn test_free() {
        let lock = MUTEX.lock().unwrap();
        init_malloc();
        let total_size = |data| -> usize {
            crate::malloc::Block::get_total_padding() + align(data)
        };

        init_malloc();

        let initial_brk = unsafe { brk(0 as *mut usize) as usize };
        println!("initial_brk {:?}", initial_brk);

        let mut tmp = malloc(18);
        free(tmp);
        tmp = malloc(17);
        free(tmp);
        tmp = malloc(18);
        free(tmp);
        tmp = malloc(24);
        free(tmp);
        let tmp1 = malloc(1048576);
        let tmp2 = malloc(1048576);
        let tmp3 = malloc(1048576);
        free(tmp1);
        free(tmp2);
        free(tmp3);
        tmp = malloc(3145728);
        free(tmp);
        
        let final_brk = unsafe { brk(0 as *mut usize) as usize };
        println!("final_brk {:?}", final_brk);

        let max = 3 * total_size(1048576) + total_size(24);
        assert_eq!(initial_brk + max, final_brk);
        Mutex::unlock(lock);
    }
}