// https://github.com/kmcallister/syscall.rs/blob/master/src/platform/linux-x86_64/mod.rs
pub const BRK: usize = 12;

#[inline(always)]
pub unsafe fn syscall1(n: usize, a1: usize) -> usize {
    let ret : usize;
    llvm_asm!("syscall" : "={rax}"(ret)
                   : "{rax}"(n), "{rdi}"(a1)
                   : "rcx", "r11", "memory"
                   : "volatile");
    ret
}