use uuid::Uuid;
use std::thread;
use std::time;

use malloc_rs::queue::Queue;

#[repr(transparent)]
struct Work(Uuid);

fn main() {
    let mut works = Queue::<Work>::new();

    let boss = thread::spawn(move || {
        loop {
            let new_uuid = Uuid::new_v4();
            let work = Work(new_uuid);
            works.push(work);
            println!("Boss push {:?}", new_uuid);

            let dur = time::Duration::from_millis(100);
            thread::sleep(dur);
        }
    });

    let worker1 = thread::spawn(move || {
        loop {
            let pop = works.pop();
            if pop.is_some() {
                println!("Worker 1 pops {:?}", pop.unwrap().0)
            }
            let dur = time::Duration::from_millis(250);
            thread::sleep(dur);
        }
    });

    let worker2 = thread::spawn(move || {
        loop {
            let pop = works.pop();
            if pop.is_some() {
                println!("Worker 2 pops {:?}", pop.unwrap().0)
            }
            let dur = time::Duration::from_millis(280);
            thread::sleep(dur);
        }
    });

    boss.join();
    worker1.join();
    worker2.join();
    println!("done");
}
