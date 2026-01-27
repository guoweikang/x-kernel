use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    ax_println!("{}", info);
    ax_println!("{}", backtrace::Backtrace::capture());
    axhal::power::system_off()
}
