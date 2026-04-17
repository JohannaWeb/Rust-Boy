use gbc_emulator::cartridge::CartridgeError;
use gbc_emulator::joypad::Button;
use gbc_emulator::ppu::{HEIGHT, WIDTH};
use gbc_emulator::{Cartridge, Emulator, EmulatorMode};
use minifb::{Key, Scale, Window, WindowOptions};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use std::thread;
use std::time::{Duration, Instant};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage(&args[0]);
        return Ok(());
    }

    match args[1].as_str() {
        "info" => {
            let path = required_path(&args)?;
            let cartridge = Cartridge::load(&path).map_err(format_cart_error)?;
            let header = cartridge.header();
            println!("title: {}", empty_title(&header.title));
            println!(
                "type: 0x{:02x} ({:?})",
                header.cartridge_type,
                cartridge.kind()
            );
            println!("rom banks: {}", header.rom_banks);
            println!("ram banks: {}", header.ram_banks);
            println!("cgb capable: {}", header.cgb_capable);
            println!("cgb only: {}", header.cgb_only);
            Ok(())
        }
        "run" => {
            let path = required_path(&args)?;
            let steps = arg_value(&args, "--steps")
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(1_000_000);
            let mode = mode_from_args(&args);
            let cartridge = Cartridge::load(&path).map_err(format_cart_error)?;
            let mut emulator = Emulator::new(cartridge, mode);
            let mut cycles = 0u64;
            for _ in 0..steps {
                cycles += u64::from(emulator.step().map_err(|err| err.to_string())?);
            }
            println!("executed {steps} CPU steps ({cycles} cycles)");
            println!("pc: 0x{:04x}", emulator.cpu.pc);
            println!("af: 0x{:02x}{:02x}", emulator.cpu.a, emulator.cpu.f);
            println!("bc: 0x{:02x}{:02x}", emulator.cpu.b, emulator.cpu.c);
            println!("de: 0x{:02x}{:02x}", emulator.cpu.d, emulator.cpu.e);
            println!("hl: 0x{:02x}{:02x}", emulator.cpu.h, emulator.cpu.l);
            println!("sp: 0x{:04x}", emulator.cpu.sp);
            println!("ime: {}", emulator.cpu.ime);
            println!("halted: {}", emulator.cpu.halted());
            println!("ie: 0x{:02x}", emulator.bus.interrupt_enable);
            println!("if: 0x{:02x}", emulator.bus.interrupt_flag);
            println!("lcdc: 0x{:02x}", emulator.bus.ppu.lcdc);
            println!("ly: {}", emulator.bus.ppu.ly);
            Ok(())
        }
        "dump-frame" => {
            let path = required_path(&args)?;
            let out = arg_value(&args, "--out")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("frame.ppm"));
            let frames = arg_value(&args, "--frames")
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(1);
            let max_steps = arg_value(&args, "--max-steps")
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(5_000_000);
            let mode = mode_from_args(&args);
            let cartridge = Cartridge::load(&path).map_err(format_cart_error)?;
            let mut emulator = Emulator::new(cartridge, mode);
            let mut completed_frames = 0;
            for _ in 0..max_steps {
                emulator.step().map_err(|err| err.to_string())?;
                if emulator.bus.ppu.take_frame_ready() {
                    completed_frames += 1;
                    if completed_frames >= frames {
                        break;
                    }
                }
            }
            write_ppm(&out, emulator.frame_buffer())?;
            println!("wrote {}", out.display());
            println!("frames observed: {completed_frames}");
            println!("pc: 0x{:04x}", emulator.cpu.pc);
            Ok(())
        }
        "play" => {
            let path = required_path(&args)?;
            let mode = mode_from_args(&args);
            let cartridge = Cartridge::load(&path).map_err(format_cart_error)?;
            let title = cartridge.header().title.clone();
            let mut emulator = Emulator::new(cartridge, mode);
            let mut window = Window::new(
                &format!("GBC Emulator - {}", empty_title(&title)),
                WIDTH,
                HEIGHT,
                WindowOptions {
                    scale: Scale::X4,
                    resize: false,
                    ..WindowOptions::default()
                },
            )
            .map_err(|err| format!("failed to open window: {err}"))?;

            while window.is_open() && !window.is_key_down(Key::Escape) {
                let frame_start = Instant::now();
                update_input(&window, &mut emulator);
                run_until_frame(&mut emulator)?;
                window
                    .update_with_buffer(emulator.frame_buffer(), WIDTH, HEIGHT)
                    .map_err(|err| format!("failed to update window: {err}"))?;

                let elapsed = frame_start.elapsed();
                let frame_time = Duration::from_micros(16_742);
                if elapsed < frame_time {
                    thread::sleep(frame_time - elapsed);
                }
            }
            Ok(())
        }
        "debug-video" => {
            let path = required_path(&args)?;
            let frames = arg_value(&args, "--frames")
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(60);
            let max_steps = arg_value(&args, "--max-steps")
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(10_000_000);
            let mode = mode_from_args(&args);
            let cartridge = Cartridge::load(&path).map_err(format_cart_error)?;
            let mut emulator = Emulator::new(cartridge, mode);
            let mut completed_frames = 0;
            for _ in 0..max_steps {
                emulator.step().map_err(|err| err.to_string())?;
                if emulator.bus.ppu.take_frame_ready() {
                    completed_frames += 1;
                    if completed_frames >= frames {
                        break;
                    }
                }
            }
            println!("frames observed: {completed_frames}");
            println!("pc: 0x{:04x}", emulator.cpu.pc);
            println!("lcdc: 0x{:02x}", emulator.bus.ppu.lcdc);
            println!("scx/scy: {}/{}", emulator.bus.ppu.scx, emulator.bus.ppu.scy);
            println!("wx/wy: {}/{}", emulator.bus.ppu.wx, emulator.bus.ppu.wy);
            for bank in 0..2 {
                let vram = emulator.bus.ppu.vram_bank(bank).unwrap();
                println!("vram{bank} nonzero: {}", count_nonzero(vram));
                println!(
                    "vram{bank} tilemap 9800 nonzero: {}",
                    count_nonzero(&vram[0x1800..0x1c00])
                );
                println!(
                    "vram{bank} tilemap 9c00 nonzero: {}",
                    count_nonzero(&vram[0x1c00..0x2000])
                );
                println!(
                    "vram{bank} tiles 8000-87ff/8800-8fff/9000-97ff nonzero: {}/{}/{}",
                    count_nonzero(&vram[0x0000..0x0800]),
                    count_nonzero(&vram[0x0800..0x1000]),
                    count_nonzero(&vram[0x1000..0x1800])
                );
                println!(
                    "vram{bank} tilemap 9800 ids: {}",
                    describe_bytes(&vram[0x1800..0x1c00])
                );
                if bank == 0 {
                    let tile = vram[0x1800];
                    let addr = signed_tile_addr(tile, 0);
                    println!(
                        "vram0 first 9800 tile 0x{tile:02x} bytes: {}",
                        format_bytes(&vram[addr..addr + 16])
                    );
                    let addr = signed_tile_addr(0x7f, 0);
                    println!(
                        "vram0 tile 0x7f signed bytes: {}",
                        format_bytes(&vram[addr..addr + 16])
                    );
                }
            }
            println!(
                "bg palette nonzero: {}",
                count_nonzero(emulator.bus.ppu.bg_palette_data())
            );
            println!(
                "obj palette nonzero: {}",
                count_nonzero(emulator.bus.ppu.obj_palette_data())
            );
            println!("bg color ids: {:?}", emulator.bus.ppu.bg_color_id_counts());
            println!(
                "first bg palette: {}",
                format_bytes(&emulator.bus.ppu.bg_palette_data()[0..8])
            );
            println!(
                "oam nonzero: {}",
                count_nonzero(emulator.bus.ppu.oam_data())
            );
            println!(
                "visible sprites: {}",
                count_visible_sprites(emulator.bus.ppu.oam_data())
            );
            println!(
                "frame colors: {}",
                count_frame_colors(emulator.frame_buffer())
            );
            Ok(())
        }
        _ => {
            print_usage(&args[0]);
            Ok(())
        }
    }
}

fn print_usage(binary: &str) {
    eprintln!("usage:");
    eprintln!("  {binary} info <rom.gb|rom.gbc>");
    eprintln!("  {binary} run <rom.gb|rom.gbc> [--steps N] [--dmg]");
    eprintln!("  {binary} play <rom.gb|rom.gbc> [--dmg]");
    eprintln!(
        "  {binary} dump-frame <rom.gb|rom.gbc> [--frames N] [--max-steps N] [--out frame.ppm] [--dmg]"
    );
    eprintln!("  {binary} debug-video <rom.gb|rom.gbc> [--frames N] [--max-steps N] [--dmg]");
}

fn required_path(args: &[String]) -> Result<PathBuf, String> {
    args.get(2)
        .map(PathBuf::from)
        .ok_or_else(|| "missing ROM path".to_string())
}

fn arg_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].as_str())
}

fn mode_from_args(args: &[String]) -> EmulatorMode {
    if args.iter().any(|arg| arg == "--dmg") {
        EmulatorMode::Dmg
    } else {
        EmulatorMode::Cgb
    }
}

fn write_ppm(path: &PathBuf, pixels: &[u32]) -> Result<(), String> {
    let mut bytes = format!("P6\n{WIDTH} {HEIGHT}\n255\n").into_bytes();
    for &pixel in pixels {
        bytes.push(((pixel >> 16) & 0xff) as u8);
        bytes.push(((pixel >> 8) & 0xff) as u8);
        bytes.push((pixel & 0xff) as u8);
    }
    fs::write(path, bytes).map_err(|err| format!("failed to write {}: {err}", path.display()))
}

fn run_until_frame(emulator: &mut Emulator) -> Result<(), String> {
    loop {
        emulator.step().map_err(|err| err.to_string())?;
        if emulator.bus.ppu.take_frame_ready() {
            return Ok(());
        }
    }
}

fn update_input(window: &Window, emulator: &mut Emulator) {
    let keys = [
        (Button::Right, Key::Right),
        (Button::Left, Key::Left),
        (Button::Up, Key::Up),
        (Button::Down, Key::Down),
        (Button::A, Key::Z),
        (Button::B, Key::X),
        (Button::Select, Key::RightShift),
        (Button::Start, Key::Enter),
    ];

    for (button, key) in keys {
        emulator
            .bus
            .joypad
            .set_button(button, window.is_key_down(key));
    }
}

fn count_nonzero(bytes: &[u8]) -> usize {
    bytes.iter().filter(|&&byte| byte != 0).count()
}

fn count_frame_colors(pixels: &[u32]) -> usize {
    let mut colors = Vec::new();
    for &pixel in pixels {
        if !colors.contains(&pixel) {
            colors.push(pixel);
        }
    }
    colors.len()
}

fn format_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn describe_bytes(bytes: &[u8]) -> String {
    let min = bytes.iter().copied().min().unwrap_or(0);
    let max = bytes.iter().copied().max().unwrap_or(0);
    let mut unique = Vec::new();
    for &byte in bytes {
        if !unique.contains(&byte) {
            unique.push(byte);
        }
    }
    format!("min=0x{min:02x} max=0x{max:02x} unique={}", unique.len())
}

fn signed_tile_addr(tile: u8, line: usize) -> usize {
    let signed = tile as i8 as i16;
    (0x1000i16 + signed * 16 + line as i16 * 2) as usize
}

fn count_visible_sprites(oam: &[u8]) -> usize {
    oam.chunks_exact(4)
        .filter(|sprite| {
            let y = sprite[0];
            let x = sprite[1];
            y > 0 && y < 160 && x > 0 && x < 168
        })
        .count()
}

fn empty_title(title: &str) -> &str {
    if title.is_empty() {
        "<untitled>"
    } else {
        title
    }
}

fn format_cart_error(err: CartridgeError) -> String {
    format!("failed to load cartridge: {err}")
}
