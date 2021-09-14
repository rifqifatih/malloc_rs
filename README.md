Memory allocation using Rust. The brk syscall 12 is used for memory allocation specifically for x86_64 machine.

## Running
Use nightly to enable feature `feature(llvm_asm)`
```
rustup override set nightly
```

Run unit test:
```
cargo test -- --test-threads=1
```
Some tests uses brk to keep track of memory offset that requires test to run in sequence.

Run endless multithreaded producer-consumer queue test:
```
cargo run $JOB_PER_SECOND $NUM_WORKERS
```
where `$JOB_PER_SECOND` is the number of push to the queue per second, and `$NUM_WORKERS` is the number of worker which consumes the queue.

Example:
```
cargo run 30 3
```

### TODO
- Explicit free list optimization
- How to run unit test in parallel
- Safely use Queue concurrently without unsafe dereferencing
- Implement for other architecture