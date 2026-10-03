# Public package and installer validation

2026-10-01: the user requested a public package and installer after accepting ticket #6's compatibility report. This follow-up adds packaging; the WinRoll application code and its recorded compatibility limits are unchanged.

## Artifacts

`scripts/package-public.ps1` reuses the locked x64 portable build and compiles `installer/winroll.iss` with the already-installed Inno Setup 6.2.1 compiler. It writes these local release assets under `target/public-release/`:

- `winroll-0.1.0-x64.zip`
- `winroll-0.1.0-x64-setup.exe`
- `SHA256SUMS.txt`, covering both assets

The ZIP and installer carry the same executable, usage/recovery guide, original/dependency notices, Rust Standard Library notices and payload checksums. As requested on 2026-10-03, public packages exclude `compatibility-report.md`; the report remains in the repository and personal-testing package. The executable remains SHA256 `2e7960ff9eddd2cc8c94b56cfb5ec5643f9ffc498d5fb5aed88fc38bc794c0a9`. The separate x86 fixture is excluded.

The installer installs under the current user's Local AppData, creates Start menu launch/uninstall shortcuts and a Windows uninstall entry, and does not request administrator privileges. Startup, launch after installation and updates remain manual. Its architecture/version gates reject non-x64 Windows and versions before Windows 10 22H2. Windows 11 21H2 (build 22000) and newer meet that minimum. A stable AppId allows replacement installations to share uninstall information.

Setup and uninstall check the application's existing `Local\WinRoll-RS.Experiment` mutex. They refuse to proceed while WinRoll is running, so the user must Exit through the tray and complete any pending recovery first. Automatic application closing/restarting is disabled. See Inno's [AppMutex](https://jrsoftware.org/ishelp/topic_setup_appmutex.htm) and [per-user privileges](https://jrsoftware.org/ishelp/topic_setup_privilegesrequired.htm) documentation.

On 2026-10-01, the user lowered the intended Windows 11 baseline from 24H2 to 21H2. The additional Windows 11 build restriction was removed, packages rebuilt, and the installer smoke test repeated on the available Windows 11 25H2 environment. Testing on 21H2 remains unverified.

## Verified

On Windows 11 25H2 build 26200.9168 x64, running without administrator privileges:

- Public packaging compiled successfully with Inno Setup 6.2.1.
- Both release-asset hashes matched the release manifest. All five payload hashes inside the six-entry ZIP matched its payload manifest.
- The running-app mutex blocked setup before creating uninstall registration.
- A fresh install into a generated workspace test directory created the per-user uninstall entry and both Start menu shortcuts.
- Installed files matched the payload hashes, and the installed executable's native self-test returned exit code 0.
- Reinstalling succeeded.
- The running-app mutex blocked uninstall and preserved the executable.
- Normal uninstall succeeded and removed application files, shortcuts and uninstall registration. The test left no WinRoll installation behind.
- Read-only review found no actionable packaging issue; `git diff --check` passed.

The runnable check is `tests/installer-smoke.ps1`. It refuses to modify an existing WinRoll installation or Start menu folder and refuses to run elevated or while WinRoll's mutex exists. It tests installer behavior using a held mutex; it does not force-close or start the production controller.

## Repeat

```powershell
$env:CARGO_HOME = Join-Path $PWD '.cargo-cache' # when using the workspace dependency cache
./scripts/package-public.ps1 -IsccPath 'D:/Program Files/Inno Setup 6/ISCC.exe'
./tests/installer-smoke.ps1
```

Supply your compiler path, or omit `-IsccPath` when `ISCC.exe` is on PATH. Run the smoke test at normal privilege with WinRoll exited; it installs temporarily for the current user and uninstalls afterward.

Artifacts are unsigned. No GitHub release was published. Other Windows baselines, unsupported-platform rejection on real machines, the interactive wizard and a live production-controller mutex check remain unverified; the accepted application compatibility gaps remain in `compatibility-report.md`.
