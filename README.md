# Game Boy Color Emulator

A Rust Game Boy / Game Boy Color emulator core.
<img width="801" height="741" alt="image" src="https://github.com/user-attachments/assets/a482737e-631b-4f72-bd3d-86c5f5656b22" />

Its a gameboy emulator in rust lol .
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
