use std::env;
use std::thread;
use std::time;
use uuid::Uuid;

use malloc_rs::queue::Queue;

#[repr(transparent)]
struct Work(Uuid);

fn main() {
    let args: Vec<String> = env::args().collect();

    let jps = *&args[1].parse::<u32>().unwrap();
    let num_workers = *&args[2].parse::<u32>().unwrap();

    let mut works = Queue::<Work>::new();
    // Use of unsafe to share pointers between threads
    let ptr = &mut works as *mut Queue<Work> as usize;

    let boss = thread::spawn(move || loop {
        let works = unsafe { &mut *(ptr as *mut Queue<Work>) };

        for _ in 0..jps {
            let new_uuid = Uuid::new_v4();
            let work = Work(new_uuid);
            works.push(work);
            println!("Boss push {:?}", new_uuid);
        }

        println!("Queue size is {:?}", works.get_size());
        let dur = time::Duration::from_millis(1000);
        thread::sleep(dur);
    });

    let mut threads = vec![];

    for i in 0..num_workers {
        threads.push(thread::spawn(move || loop {
            let works = unsafe { &mut *(ptr as *mut Queue<Work>) };
            let pop = works.pop();
            if pop.is_some() {
                println!("Worker {:?} pops {:?}", i, pop.unwrap().0)
            }
            let dur = time::Duration::from_millis(100);
            thread::sleep(dur);
        }));
    }

    let _ = boss.join();
    for thread in threads {
        let _ = thread.join();
    }

    println!("done");
}
