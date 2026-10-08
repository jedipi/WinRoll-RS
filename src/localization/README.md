# Contributing translations

Each language lives in a Rust source file under `src/localization/`:

- `en.rs`: English and the complete list of message keys.
- `zh_hans.rs`: Simplified Chinese.
- `zh_hant.rs`: Traditional Chinese.

These files compile into the executable. No translation files need to be installed
alongside it. Selecting a language in Options applies immediately; rebuilding is
only necessary when adding or editing translations.

## Add a language

1. Copy `src/localization/en.rs` to a new file, for example `fr.rs`.
2. Give it an unused positive `id` and its native display `name`, such as
   `Français`. ID 0 means System default; existing IDs are saved preferences
   and must never be changed or reused.
3. Set `windows_ids` to the Windows UI language IDs this translation supports.
   Use an empty list if automatic Windows-language matching is not yet available.
4. In each message pair, keep the first string (the English key) unchanged and
   translate the second. Preserve placeholders such as `{version}`, `{percent}`,
   `{name}` and `{error}`, and newline escapes such as `\n`.
   An ampersand marks a keyboard shortcut: choose an appropriate letter or retain
   a suffix such as `(&L)`. Use `&&` for a literal ampersand.
5. In `src/localization.rs`, add `mod fr;` and add `fr::LANGUAGE` to
   `LANGUAGES`. The dropdown, preference validation and tests use this list
   automatically. No window-code changes are needed.

Use distinct terms for collapsing a window to its title bar (roll up), expanding
it again (unroll), and minimizing/restoring it. Leave product names, window titles
and Windows-provided error details unchanged.

## Check the result

Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and
`cargo test --locked`. Tests check duplicate IDs and keys, missing messages,
placeholders, Windows-language mappings, and live switching for every registered
language. Unknown languages or untranslated messages fall back to English.

Build with `cargo build --release --locked`, then check Options, About and tray
menus visually, including keyboard navigation and longer labels. Changes must
appear without restarting, and the selection must survive an application restart.
Chinese translations remain initial drafts awaiting language review.
