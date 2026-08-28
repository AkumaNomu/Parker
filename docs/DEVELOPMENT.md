# Development

## Prerequisites

- Windows 10/11 or any Linux distro listed in `docs/SETUP.md`.
- Stable Rust with `rustfmt` and `clippy`.
- FFmpeg on `PATH`, beside the executable, or configured with `PARKER_FFMPEG`.
- Tesseract on `PATH`, in a standard installation directory, beside the
  executable, or configured with `PARKER_TESSERACT`.
- On Linux, `curl` for self-update and a clipboard tool (`wl-copy` or `xclip`).

Cargo downloads crates on first build; commit `Cargo.lock` after a successful
build on either platform.

## Checks

Windows:

```powershell
.\scripts\windows\check.ps1
```

Linux:

```bash
./scripts/linux/check-linux.sh
# or the raw commands:
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked --release
```

Shared modules (`src/smart.rs`, `src/translate.rs`, `src/updater.rs`) are compiled
on both platforms — keep `#[cfg]` gates correct and verify by reading when you
cannot compile the other target locally.

## Manual test matrix

1. Select a QR URL and verify it is copied and opened once. On Linux confirm `parker capture` finds it via `rqrr`.
2. Set `PARKER_QR_AUTO_OPEN=0` and verify the URL is only copied.
3. Select multiple QR codes and verify all payloads are copied line-by-line.
4. Select source code in light and dark themes; verify code classification and
   indentation retention.
5. Select a two-column and multi-column table; paste into Excel/Sheets and verify TSV.
6. Force `PARKER_OCR_MODE` to each value and verify behavior; on Linux also check
   `PARKER_KEEP_OCR_CAPTURE=1` cleans up correctly.
7. Run `parker shot` / `Ctrl+Shift+F9` and paste as image (primary screenshot workflow).
8. Cancel both selectors with `Esc` and right-click; cancellation should return
   quietly to the GUI without an error dialog.
9. Verify toast/notification feedback without stealing focus.
10. Remove FFmpeg and verify smart capture still works.
11. Remove Tesseract and verify QR detection and recording still work.
12. Record regions on the primary monitor and monitors arranged left or above it (`Ctrl+Shift+F11`).
13. Move the pointer through the region and confirm the cursor never appears.
14. Stop a short and a long recording; verify the temporary MKV is removed and
   the MP4 is playable and copied (`CF_HDROP` / `text/uri-list`).
15. Set different `PARKER_COMPRESSION` / `PARKER_POST_CRF` / `PARKER_MAX_WIDTH` values and inspect output + `ffmpeg.log`.
16. Force a post-process failure and verify `.capture.mkv` is preserved + `parker batch` recovers it.
17. Exit while recording and verify the capture is finalized and copied.
18. Test `--self-update` against a stubbed GitHub API; verify the matching
    `.sha256` asset is required, the checksum is validated, unsafe tar members
    are rejected, and a failed replacement restores the old binary.

## Versioning

Update `Cargo.toml` and `CHANGELOG.md`, regenerate and commit `Cargo.lock`, then
push a `vX.Y.Z` tag to create a GitHub release package.
