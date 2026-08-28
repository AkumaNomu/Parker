# Contributing

## Development requirements

- Windows 10/11, or a Linux distribution with the runtime tools listed in
  `docs/SETUP.md`.
- Stable Rust with `rustfmt` and `clippy`.
- PowerShell 5.1 or later (Windows workflows).
- FFmpeg available through `PARKER_FFMPEG`, beside the executable, or on
  `PATH`.
- Tesseract available through `PARKER_TESSERACT`, a standard installation, or
  `PATH`.

## Local workflow

Windows:

```powershell
.\scripts\windows\check.ps1
```

Linux:

```bash
./scripts/linux/check-linux.sh
```

Both wrap: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D
warnings`, `cargo test`, and a release build.

Run the application during development with:

```powershell
.\scripts\windows\run-dev.ps1
```

Global hotkeys can be registered by only one Parker process at a time. Stop an
installed instance before starting a development build.

AI agents should read `AGENTS.md` before changing shared modules and record
session notes in `memory.md`.

## Pull requests

Keep changes focused. Update documentation and `CHANGELOG.md` when behavior or
public configuration changes. Explain how QR, OCR, recording, post-processing,
clipboard behavior, and multi-monitor coordinates were tested when relevant.

Unsafe Win32 code should remain isolated and narrowly scoped. Do not introduce
capture uploads, analytics, automatic command execution, or broader URL schemes
without explicit security review and opt-in behavior.
