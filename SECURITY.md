# Security Policy

## Supported versions

Only the latest released version is supported with security fixes.

## Security boundaries

Parker processes captures locally. QR auto-opening accepts only HTTP and HTTPS
values without whitespace or control characters. This limits protocol-handler
abuse but does not make the destination trustworthy. Users can set
`PARKER_QR_AUTO_OPEN=0` to copy QR content without opening it.

Parker must not execute OCR output, QR payloads, captured commands, or file paths
as shell commands. Runtime paths for FFmpeg and Tesseract should point only to
trusted executables.

## Reporting a vulnerability

Do not publish exploitable details in a public issue. Use GitHub's private
security-advisory feature for the repository, or contact the repository owner
privately through GitHub.

Include the affected version, reproduction steps, expected impact, and any
suggested mitigation. Reports will be assessed before public disclosure.
