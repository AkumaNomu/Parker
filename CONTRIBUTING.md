# Contributing

## Development requirements

- Windows 10 or Windows 11.
- Stable Rust with `rustfmt` and `clippy`.
- PowerShell 5.1 or later.
- FFmpeg available through `PARKER_FFMPEG`, beside the executable, or on
  `PATH`.
- Tesseract available through `PARKER_TESSERACT`, a standard installation, or
  `PATH`.

## Local workflow

```powershell
rustup component add rustfmt clippy
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

Run the application during development with:

```powershell
.\run-dev.ps1
```

Global hotkeys can be registered by only one Parker process at a time. Stop an
installed instance before starting a development build.

## Pull requests

Keep changes focused. Update documentation and `CHANGELOG.md` when behavior or
public configuration changes. Explain how QR, OCR, recording, post-processing,
clipboard behavior, and multi-monitor coordinates were tested when relevant.

Unsafe Win32 code should remain isolated and narrowly scoped. Do not introduce
capture uploads, analytics, automatic command execution, or broader URL schemes
without explicit security review and opt-in behavior.
