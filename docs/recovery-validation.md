# Ticket #4 recovery validation

2026-09-25: The release self-test passed with two native fixtures, a controlled F8 expansion refusal, mixed unroll outcomes, and Pause/Exit retry. `cargo fmt --check`, `cargo clippy --all-targets --locked --offline -- -D warnings`, and `cargo test --locked --offline` passed.

The user confirmed both live tray recovery paths with the updated fixture. Failed Pause and Exit kept the refusing window rolled and offered **Retry unroll all**. After F8 allowed expansion, retry expanded the fixture; Pause remained available for Enable, while Exit removed WinRoll from the tray. The client area painted normally after the background-brush fix.

The earlier attempt used a stale fixture executable without the F8 toggle and did not produce a recovery failure. The live results are user-reported; this run did not record per-window geometry or environment versions. The automated self-test does not exercise notification-area activation or user selection of menu actions.
