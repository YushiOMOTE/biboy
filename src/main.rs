#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

#[macro_use]
extern crate alloc;

use alloc::{boxed::Box, vec::Vec};
use bootloader::BootInfo;
use core::{env, panic::PanicInfo};
use log::*;
use rand::{rngs::JitterRng, Rng};
use rgy::{
    self,
    hardware::{Key, Stream},
};
use x86_64::instructions::port::Port;

mod allocator;

use crate::allocator::init_heap;

struct Serial {
    ports: [Port<u8>; 8],
}

impl Serial {
    fn new() -> Self {
        let mut ports = [
            Port::new(0x3f8),
            Port::new(0x3f9),
            Port::new(0x3fa),
            Port::new(0x3fb),
            Port::new(0x3fc),
            Port::new(0x3fd),
            Port::new(0x3fe),
            Port::new(0x3ff),
        ];

        unsafe {
            ports[1].write(0x00);
            ports[3].write(0x80);
            ports[0].write(0x03);
            ports[1].write(0x00);
            ports[3].write(0x03);
            ports[2].write(0xc7);
            ports[4].write(0x0b);
        }

        Self { ports }
    }

    fn read(&mut self) -> Option<u8> {
        unsafe {
            if self.ports[5].read() & 0x01 != 0 {
                Some(self.ports[0].read())
            } else {
                None
            }
        }
    }

    fn write(&mut self, d: u8) {
        unsafe {
            while self.ports[5].read() & 0x20 == 0 {}
            self.ports[0].write(d);
        }
    }
}

struct Keyboard {
    port: Port<u8>,
}

impl Keyboard {
    fn new() -> Self {
        Self {
            port: Port::new(0x60),
        }
    }

    fn read(&mut self) -> Option<u8> {
        Some(unsafe { self.port.read() })
    }
}

fn tsc() -> u64 {
    #[cfg(target_arch = "x86")]
    use core::arch::x86::_rdtsc;
    #[cfg(target_arch = "x86_64")]
    use core::arch::x86_64::_rdtsc;

    unsafe { _rdtsc() as u64 }
}

struct Hardware {
    kbd: Keyboard,
    display: Display,
    rng: JitterRng,
    vramsz: (usize, usize),
}

impl Hardware {
    fn new(display: Display, kbd: Keyboard) -> Self {
        Self {
            display,
            kbd,
            rng: JitterRng::new_with_timer(tsc),
            vramsz: (0, 0),
        }
    }
}

impl rgy::Hardware for Hardware {
    fn vram_update(&mut self, line: usize, buffer: &[u32]) {
        for (i, b) in buffer.iter().enumerate() {
            let col = match b & 0xff {
                0..=63 => 0,
                64..=127 => 8,
                128..=191 => 7,
                192..=255 => 15,
                _ => 0,
            };
            self.display.set(i, line, col)
        }
    }

    fn joypad_pressed(&mut self, key: Key) -> bool {
        let scancode = match key {
            Key::Right => 0x4d,
            Key::Left => 0x4b,
            Key::Up => 0x48,
            Key::Down => 0x50,
            Key::A => 0x2c,
            Key::B => 0x2d,
            Key::Select => 0x39,
            Key::Start => 0x1c,
        };

        self.kbd.read().map(|s| s == scancode).unwrap_or(false)
    }

    fn sound_play(&mut self, _stream: Box<dyn Stream>) {
        // Unimplemented
    }

    fn clock(&mut self) -> u64 {
        tsc() / 1000
    }

    fn send_byte(&mut self, _b: u8) {
        // Unimplemented
    }

    fn recv_byte(&mut self) -> Option<u8> {
        None
    }

    fn sched(&mut self) -> bool {
        true
    }

    fn load_ram(&mut self, size: usize) -> Vec<u8> {
        vec![0; size]
    }

    fn save_ram(&mut self, _ram: &[u8]) {
        // Unimplemented
    }
}

struct Display {
    vram: *mut u8,
}

impl Display {
    const WIDTH: usize = 320;
    const SCALE: usize = 1;

    fn new() -> Self {
        Self {
            vram: 0xa0000 as *mut u8,
        }
    }

    fn set(&self, x: usize, y: usize, col: u8) {
        let s = Display::SCALE;

        for xo in 0..s {
            for yo in 0..s {
                let i = (x * s + xo) + (y * s + yo) * Display::WIDTH;

                unsafe {
                    *self.vram.offset(i as isize) = col;
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
    init_heap(boot_info).unwrap();

    comlog::init_with_filter(LevelFilter::Info);

    info!("Starting");

    let d = Display::new();
    let k = Keyboard::new();
    let hw = Hardware::new(d, k);

    let cfg = rgy::Config::new()
        .freq(env!("BIBOY_FREQ").parse().unwrap_or(4194300))
        .sample(env!("BIBOY_SAMPLE").parse().unwrap_or(4194))
        .delay_unit(env!("BIBOY_DELAY_UNIT").parse().unwrap_or(10))
        .native_speed(env!("BIBOY_NATIVE").parse().unwrap_or(false));

    rgy::run(cfg, include_bytes!(env!("BIBOY_ROM")).to_vec(), hw);

    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("panic: {}", info);
    loop {}
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout);
}
