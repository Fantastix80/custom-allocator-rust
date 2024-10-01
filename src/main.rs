#![no_std]
#![no_main]

use core::panic::PanicInfo;

// Build le programme avec: RUSTFLAGS="-C link-arg=-nostartfiles" cargo build

#[panic_handler]
fn panic(_panic: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() {
    loop {}
}
