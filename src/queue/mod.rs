use std::mem::size_of;
use crate::malloc::{malloc, free};

const CAPACITY_INC: usize = 32;
const INITIAL_CAPACITY: usize = 32;

struct Segment {
    next: *mut Segment,
    origin: *mut usize,
    len: usize
}

struct Queue<T> {
    head: *mut T,
    tail: *mut T,
    head_segment: *mut Segment,
    tail_segment: *mut Segment,
    size: usize
}

impl Segment {
    pub fn has_next(&self) -> bool {
        self.next as usize != 0
    }
}

impl <T> Queue<T> {

    #[allow(dead_code)]
    pub fn new<'a>() -> Queue<T> {
        let head = malloc(INITIAL_CAPACITY * size_of::<T>()) as *mut T;
        let head_segment_ptr = malloc(size_of::<Segment>()) as *mut Segment;
        println!("new {:?} {:?}", head as usize, head_segment_ptr as usize);
        unsafe {
            *head_segment_ptr = Segment {
                next: 0 as *mut Segment,
                origin: head as *mut usize,
                len: INITIAL_CAPACITY * size_of::<T>()
            };
        }
        Queue::<T> {
            head,
            tail: head,
            head_segment: head_segment_ptr,
            tail_segment: head_segment_ptr,
            size: 0
        }
    }

    #[allow(dead_code)]
    pub fn pop<'a>(&'a mut self) -> &'a mut T {
        assert!(!self.is_empty());
        // println!("pop size {:?} at {:?} {:?}", self.size, self.head as usize, self.head_segment as usize);

        let res = unsafe { &mut *self.head };
        let head_segment_ptr = self.head_segment;
        let head_segment = unsafe { &*self.head_segment };
        let next = unsafe { &*head_segment.next };

        let is_last_block = self.head as usize + size_of::<T>() >= head_segment.origin as usize + head_segment.len;
        if is_last_block {
            println!("is_last_block origin {:?}", head_segment.origin as usize);
            self.head_segment = head_segment.next;
            self.head = next.origin as *mut T;
            free(head_segment.origin);
            free(head_segment_ptr as *mut usize);
        } else {
            self.head = (self.head as usize + size_of::<T>()) as *mut T;
        }

        self.size -= 1;
        res
    }

    fn allocate_next(&mut self) {
        println!("start allocate");
        let origin = malloc(CAPACITY_INC * size_of::<T>()) as *mut T;
        println!("done origin");
        let segment = malloc(size_of::<Segment>()) as *mut Segment;
        println!("done segment");
        println!("allocation at {:?}", origin as usize);
        unsafe {
            *segment = Segment {
                next: 0 as *mut Segment,
                origin: origin as *mut usize,
                len: CAPACITY_INC * size_of::<T>()
            };
        }

        let mut tail_segment = unsafe { &mut *self.tail_segment };
        tail_segment.next = segment;
    }

    #[allow(dead_code)]
    pub fn push(&mut self, item: T) {
        let tail_segment = unsafe { &*self.tail_segment };
        if !tail_segment.has_next() {
            println!("no next");
            self.allocate_next();
        }

        // println!("push at {:?} {:?}", self.tail as usize, self.tail_segment as usize);

        self.size += 1;
        unsafe { *self.tail = item; }

        let is_last_block = self.tail as usize + size_of::<T>() >= tail_segment.origin as usize + tail_segment.len;
        if is_last_block {
            println!("push last block {:?}", tail_segment.origin as usize);
            self.tail_segment = tail_segment.next;
            let next = unsafe { &*self.tail_segment };
            println!("next origin is {:?}", next.origin as usize);
            self.tail = next.origin as *mut T;
        } else {
            self.tail = (self.tail as usize + size_of::<T>()) as *mut T;
        }
    }

    pub fn get_size(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }
}

mod tests {
    use super::Queue;

    #[test]
    fn test_queue_0() {
        let mut q = Queue::<i32>::new();

        q.push(1);
        q.push(13);
        q.push(14);
        q.push(15);
        q.push(11);
        q.push(21);
        q.push(51);

        assert_eq!(*q.pop(), 1);
    }

    #[test]
    fn test_queue_1() {
        let mut q = Queue::<i32>::new();
        
        for i in 0..100 {
            q.push(i);
        }

        for i in 0..100 {
            assert_eq!(*q.pop(), i);
        }
    }

    #[test]
    fn test_queue_2() {
        let mut q = Queue::<i32>::new();
        
        for i in 0..100 {
            q.push(i);
        }

        for i in 0..100 {
            assert_eq!(*q.pop(), i);
        }
    }
}