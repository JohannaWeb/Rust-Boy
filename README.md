# Game Boy Color Emulator

A Rust Game Boy / Game Boy Color emulator core.

This is a headless first milestone: it can load cartridges, run CPU steps, tick timers and the PPU, maintain a 160x144 framebuffer, and expose cartridge metadata from the command line. A windowed frontend can be added on top of the library crate.

## Commands

```powershell
cargo run -- info path\to\game.gbc
cargo run -- run path\to\game.gbc --steps 1000000
cargo run -- run path\to\game.gb --steps 1000000 --dmg
```

## Current Scope

- SM83 CPU core with most regular opcodes and all CB-prefixed bit operations.
- Interrupt dispatch and HALT behavior.
- Cartridge parsing plus ROM-only, MBC1, MBC3, and MBC5 banking.
- CGB VRAM and WRAM bank registers.
- Timers, joypad register, serial registers, DMA, and interrupt flags.
- Background tile renderer into an ARGB framebuffer.

## Next Milestones

- Add a `pixels`/`winit` frontend for video, input, and audio timing.
- Finish exact PPU mode timing, sprites, window layer, STAT edge behavior, HDMA, and CGB speed switching.
- Run public CPU and PPU conformance ROMs and fix compatibility gaps.
