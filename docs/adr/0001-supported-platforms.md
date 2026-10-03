# Support Windows 10 and 11 on x64 only

The referenced planning conversation settled on Windows 10/11 x64 as the operating-system scope, dropping XP, Vista, Windows 7/8/8.1 and 32-bit Windows. WinRoll will target `x86_64-pc-windows-msvc`, avoiding separate x86 builds and legacy compatibility paths; both 32-bit and 64-bit target applications remain in scope, subject to feature compatibility testing.

The accepted baseline is Windows 10 22H2 and Windows 11 21H2 or newer. On 2026-10-01, the user lowered the Windows 11 baseline from 24H2 to 21H2. Intended support is distinct from verified support: record tested OS and application versions, and mark unavailable environments unverified.
