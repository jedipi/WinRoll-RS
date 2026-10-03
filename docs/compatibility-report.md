# Personal-testing compatibility report — ticket #6

Run date: 2026-10-01 (Australia/Sydney). This is a local personal-testing build, not a public release or a claim of universal compatibility.

Acceptance: on 2026-10-01 the user reviewed this report and confirmed "compatibility report checked. all good". Ticket #6 is accepted for personal testing with the unverified checks below retained; that acceptance does not turn those checks into passes.

## Build and environment

- Application source: `f570eac0662ac97af115635c75c946619868aa47`, WinRoll RS 0.1.0. Recovery, tray controls and monitor placement already share the same executable. No application behavior changed for this ticket.
- Packaging change: `.cargo/config.toml` selects static CRT linking for `x86_64-pc-windows-msvc`. The original build imported `VCRUNTIME140.dll`; the packaged build imports only Windows system DLLs/API sets, as checked with MSVC `dumpbin /DEPENDENTS`.
- Packaged executable SHA256: `2e7960ff9eddd2cc8c94b56cfb5ec5643f9ffc498d5fb5aed88fc38bc794c0a9`.
- The 60 real-application cycles and user tray checks ran before the CRT linking change, on executable SHA256 `25598d0519e4ac39a61c7e745fa9c8698e15124e420d1d33a957c0898d4a0976`. The final static-CRT executable passed the native self-test again; the full live matrix was not repeated on that binary. These are separate observations, not byte-identical artifacts.
- OS tested: Windows 11 25H2 x64, build 26200.9168. The user revised the intended baselines on 2026-10-01 to Windows 10 22H2 and Windows 11 21H2 or newer, x64.
- Rust 1.98.1 (`48a229cea`), Cargo 1.98.1, MSVC tools 14.51.36231. Locked dependencies: windows-sys 0.61.2, windows-link 0.2.1.
- Primary display: bounds `(0,0)-(2560,1440)`, work area `(0,0)-(2560,1392)`, 96 DPI / 100%.
- Right portrait display: bounds `(2560,0)-(3640,1920)`, work area `(2560,0)-(3640,1872)`, 96 DPI / 100%.
- Controller and targets ran at normal user privilege on the interactive desktop. An initial sandbox launch could not create a tray icon; the normal-desktop launch succeeded. That sandbox limitation is not counted as an application compatibility failure.

## Passed checks

Real-application cycles used right-click caption input through the installed global mouse hook. Each completed unroll logged five matching observed geometry samples, rather than merely an API success. Screenshots were inspected between inputs. Dimensions below are outer-window pixels; screenshot bounds omit invisible window borders and can clip at monitor edges.

| Target | Version / architecture | Completed cycles | Measured expanded geometry and outcome |
| --- | --- | --- | --- |
| Microsoft Edge | 153.0.4234.48 / x64 | 20 | 1335×1114 → 1335×40 → 1335×1114. Initial position (106,298); after intervening user movement, (508,133). Each group restored at its current position. |
| Google Chrome | 153.0.8010.53 / x64 | 20 | 1265×956 → 1265×40 → 1265×956 at (1026,597). |
| File Explorer | 10.0.26100.8117 / x64 | 20 | 1080×641 → 1080×40 → 1080×641 at (2560,0). One non-caption/pass-through click occurred while another Explorer window was in front; activation and retry completed that cycle. |
| Separate native fixture | MSVC build of `tests/native-x86.c` / x86 PE32 | 1 measured caption cycle | x64 controller rolled 800×600 to 800×40 and unrolled to 800×600 at (100,100). Fixture compiled with `/W4 /WX /O2 /MT`; PE machine 0x014c. The fixture is not included in the release ZIP. |

| Check | Evidence and boundary |
| --- | --- |
| Unrelated right-click input | Edge tab and address-bar menus appeared normally; Chrome tab, address-bar and page menus appeared normally; Explorer tab and ordinary folder-background menus appeared normally. No roll was triggered by those samples. |
| Maximized exclusion and caption button | Edge's maximize button worked. Caption right-click while maximized left it unchanged and opened its normal window menu; diagnostic confirmed `maximized=1`. Native self-test independently checks maximized exclusion. |
| Moved window and reachability | Rolled Explorer moved from the portrait display to (2060,0), then (1595,0), with caption-height geometry after each drag. Unroll restored 1080×641 at (1480,0), clamped to fit the primary work area. This is a same-DPI crossing, not a mixed-DPI result or frame-by-frame assertion. |
| Live Pause / Enable / Unroll all / Exit | User confirmed all four requested tray actions passed with two disposable windows. Pause preserves rolled windows; Enable resumes gestures; Unroll all expands them; Exit expands before quitting. Logs corroborate successful Unroll all and Exit restoration. User-reported visual evidence is distinguished from automated menu selection, which the available desktop tool could not perform. |
| Controlled restoration failure and retry | Final binary's native self-test passes mixed successful/refused unroll, affected-window identification, retained recovery state, retry, and pending-Exit completion. These checks use the native fixture and Manager boundary, not visible tray action selection. |
| Pause preservation, Enable and paused movement | Final native self-test passes multiple-window geometry preservation and managed drag while paused, followed by restoration. |
| Closed-window cleanup and stale identity | Final native self-test passes closed rolled-window cleanup, closed pending-recovery Exit completion, and invalidated ownership-marker handling without resizing the unowned window. This does not establish actual OS HWND reuse. |
| Placement regression | Tests pass moved/unchanged positions, negative monitor coordinates, oversized width and a tall caption near the work-area bottom. Native self-test passes moved restoration and synthetic DPI/height correction. |
| Tray modal-loop shutdown | Final native self-test opens the menu and verifies Exit completion dismisses it and completes the outer message loop. This does not test notification-area activation. |
| Build and package | Formatting, Clippy with warnings denied, all 3 unit tests, locked offline x64 release build and final native self-test pass. PE machine is AMD64; static CRT build has no VC++ runtime DLL import. ZIP contains one product executable, usage/report/notices and SHA256 manifest. |

The test controller exited normally after the user's tray checks. Disposable test windows were closed afterward. Local raw evidence is retained under ignored `target/`: `acceptance.log`, `acceptance-capture-bounds.json`, `self-test.log`, and `package-self-test.log`. Capture metadata starts at Edge cycle 11 and includes subsequent Chrome/Explorer cycles; the controller log contains all 60 completed cycles. Do not infer full-cycle metadata from the shorter capture file.

## Failed checks

No required application failed the completed roll/unroll or sampled input checks. The initial dynamic-runtime dependency failed the portable-build audit and was corrected by static CRT linking. Desktop-tool access/targeting interruptions are recorded above and are not silently counted as successful gestures.

Any reproducible required-application failure in the remaining checks requires architecture review before adding features or changing to injection; do not silently drop that application.

## Unverified checks and limits

- Windows 10 22H2 and Windows 11 21H2, 22H2, 23H2 and 24H2 were not tested in this run. Windows 11 25H2 testing does not establish those versions.
- Both available displays were at 100%. The final self-test explicitly reports mixed-DPI movement unverified. Earlier ticket #5 records a user-reported Explorer 100%/125% boundary pass, but that is historical evidence on an earlier build, not this run's mixed-DPI matrix.
- Snapped-window exclusion was not completed in this run. Earlier feasibility evidence records a native snapped-window pass; it is not a new pass for the packaged binary.
- The final static-CRT binary has native regression coverage; the 20-cycle-per-app and live tray matrix remain to be repeated on its exact hash.
- Broader mouse-button/modifier combinations, every caption button, page controls and browser themes, elevated targets, hung-process recovery, actual reused HWNDs, monitor removal and long-lived rolled windows were not exhaustively exercised.
- Controlled failed recovery/retry passed the native self-test; a new visible tray refusal/retry test with F8 was not performed in this run. Historical recovery documents describe earlier Pause behavior; they must not be used to claim that current Pause expands windows.
- There is no automatic crash, forced-termination or logoff recovery. Native caption double-click behavior is unavailable while rolled. Public packaging, installers, updates, startup integration and deferred window-management features remain out of scope.

## Repeat the remaining checks

Use the packaged `winroll.exe`, recording its SHA256, Windows/application versions and display arrangement/scaling. Run `--inventory` and `--self-test` from a terminal with `Start-Process -Wait -PassThru` to capture diagnostic completion. Then repeat 20 caption cycles per required application, compare expanded geometry, and test non-caption input, maximized/snapped exclusions, moved placement and mixed-scaling crossings where available.

For live failure/retry, launch a second copy with `--fixture`. Focus it and press F8 until its title says it refuses expansion, roll it, then choose Unroll all or Exit from the controller's tray. Confirm it stays available and identifies the window. F8 allows expansion again; Retry unroll all should restore it and complete a pending Exit. Repeat with a second responsive managed window, and close a refusing window to check cleanup. Keep unavailable environments and unperformed checks explicitly unverified.

Build the separate x86 target from an x86 Native Tools Command Prompt using the command at the top of `tests/native-x86.c`. Test it against the x64 controller; never ship a second WinRoll executable for cross-bitness support.
