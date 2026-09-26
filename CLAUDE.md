# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## The project

NoteFetch is a Windows desktop app in Rust that downloads recordings, transcripts and AI summaries from a Plaud account through Plaud's hosted MCP server. It is built for blind users first. The developer, Laurent, is blind and uses JAWS on Windows 11. He tests every milestone by keyboard with JAWS.

Read SPEC.md for what to build, PLAN.md for why, and TASKS.md for the order. Read PLAN.md before changing any design decision, and record any change there with the reason.

The sister project StableSound (github.com/ldefontenay/stablesound) sets the conventions: raw Win32 dialogs from `.rc` templates, the `windows` crate, `embed-resource` in `build.rs`, a reviewable message window, and help compiled into the exe. Follow its patterns where they apply.

## Reporting to Laurent: log.md, not the terminal

**Anything you would otherwise report in the terminal goes into `log.md`, newest entry first.** Laurent cannot reliably read the terminal with JAWS, especially long messages. This is the same arrangement as in TweakSound.

- Each entry starts with a level 2 heading holding the date and a short title, for example `## 2026-09-26: Milestone 1 spike results`, so the heading keys jump between entries. Use level 3 headings inside an entry, starting with "The short version".
- Plain lists. No tables, no box drawing.
- Put every question just before its own `ANSWER:` line, so he can answer in the file.
- The terminal reply is one line pointing at the log.
- At the start of a session, read the top of `log.md` as well as PLAN.md, and check for answers he has written under `ANSWER:` lines.

## Accessibility knowledge from earlier projects

**Load the `accessible-windows-apps` skill before any interface work** (dialogs, menus, the list view, announcements, help, test rounds). It lives in `~/.claude/skills/accessible-windows-apps/` and holds what StableSound and TweakSound taught through many rounds of testing with Laurent, marked `[proven]`, `[reasoned]` or `[untested]`. Never upgrade a marker without new evidence. Its `references/recipes.md` has working Rust code for dialogs and the message loop.

Before asking Laurent to test any window, run the skill's probe against it and fix what it finds. It settles names, labels, roles, mnemonics, focus and tab order, so his time goes on what only listening can judge:

```
powershell -NoProfile -ExecutionPolicy Bypass -File "$env:USERPROFILE\.claude\skills\accessible-windows-apps\scripts\a11y-probe.ps1" -Mode selftest
powershell -NoProfile -ExecutionPolicy Bypass -File "$env:USERPROFILE\.claude\skills\accessible-windows-apps\scripts\a11y-probe.ps1" -Mode dialog -Window 'NoteFetch'
```

Proven findings that bear directly on NoteFetch:

- A label must sit immediately before its control in the `.rc` template, or the control has no name.
- JAWS does not reliably announce group box headings, so no label may depend on one.
- A single-line read-only edit is read out in full when it gets focus. A multi-line one is not, and its caret lands at the end after text is set. NoteFetch's Log is multi-line. Laurent chose newest entry at the top (PLAN.md), so insert new lines at the start and set the caret to the start with `EM_SETSEL` 0, 0 and `EM_SCROLLCARET` after every write (see TweakSound PLAN.md section 2.12). Check what Tab to the Log says.
- Tones only for what the user did, never for what the app does by itself.

The skill has nothing yet on list views, context menus in a dialog, in-place label editing or UI Automation notification events. Those are NoteFetch's new ground: mark what is learned `[proven]` or `[untested]` in PLAN.md, and add proven findings back to the skill.

## Current state

Check TASKS.md for the first unticked milestone before starting work. Milestone 1 is a spike in `spike/plaud-probe` that must answer the "Open questions" in SPEC.md, including whether the free Plaud plan has access and whether loopback redirects are accepted, before the real app is written. If loopback redirects are refused, follow the fallback in PLAN.md.

## Hard rules

- **No drawn-UI toolkits.** No egui, iced, Slint, Tauri, webviews or similar. Real Win32 controls from resource templates only.
- **Only the interface thread touches windows.** Network and disk work run on the worker thread's tokio runtime, and results reach the interface through the event queue plus a `PostMessageW` signal. Never block the interface thread.
- **Never log or print secrets.** That means access tokens, refresh tokens, client registrations, authorisation codes and temporary audio links.
- **Never commit** tokens, `auth.bin`, `notefetch.conf`, databases, real audio, transcripts or summaries. Real data used in development goes in `test-data/`, which is ignored by git. The repository will become public, and its whole history with it.
- **No rmcp.** NoteFetch has its own small MCP client, adapted from plaud-cli (MIT). Keep plaud-cli's copyright notice in any file that takes code from it. Check library APIs on docs.rs for the version pinned in Cargo.lock rather than working from memory.
- **Ask Laurent before adding a dependency**, and say why it is needed. Keep the dependency list short.

## Accessibility rules

- Every control has a static label just before it in tab order, and an Alt mnemonic.
- Tab order follows SPEC.md.
- Focus always returns to a sensible place after an operation, usually the list, and is never left on a hidden or disabled control.
- Nothing is conveyed by colour, icon or position alone. List status is in words.
- Everything announced is also written to the Log.
- Routine information never goes in a message box. Message boxes are only for questions the user must answer.
- Error text says what happened and what to do next, in plain words.
- Flag any accessibility problem you notice, even outside the current task.

## Language and output

- UK English everywhere: interface text, help, comments, commit messages, and identifiers where natural (for example `colour`, `authorise`, `licence` as a noun).
- No em dashes in interface text, help, README or docs. Use commas, full stops or parentheses.
- Terminal output from any program, script or test in this repository is plain text only. No ANSI colour codes, emoji, box-drawing characters or decorative ASCII.
- When reporting to Laurent, lead with the result. Keep summaries short and screen-reader friendly: headings and bullet lists, not tables unless the data is genuinely tabular.

## Commands

```
cargo build --release
cargo test
cargo clippy -- -D warnings
cargo test -- --ignored
cargo test naming::            # one module's tests
cargo test some_test_name      # a single test, by name or substring
cargo run -- --sync            # hidden developer flag: run a sync with plain progress lines, no interface
cargo run --manifest-path spike/plaud-probe/Cargo.toml   # milestone 1 spike, a separate crate
```

- Clippy must stay clean.
- The `--ignored` tests are those that touch the real machine or the network, such as DPAPI round trips and live Plaud calls.
- The `--sync` flag and the spike do not exist until milestones 2 and 1 respectively.

## Architecture (planned, see TASKS.md milestones 2 and 3)

One binary crate at the repository root. `build.rs` compiles the `.rc` file (dialogs, menu, accelerators, version resource, manifest, help) with `embed-resource`.

Two threads:

- The interface thread owns the modeless main dialog and every window. Its message loop runs the accelerator table (window-wide keys only) before `IsDialogMessage`. The list's own keys (Enter, Delete, F2, Ctrl+A and so on) are handled in the list view's subclass so they never fire in the filter box.
- The worker thread runs a tokio runtime. Commands reach it through a channel. Events come back through a queue, and the worker posts a message to the dialog to say events are waiting. The dialog drains the queue, updates controls, writes to the Log and raises any UI Automation announcement.

Engine modules, none of which touch windows:

- `config`: `notefetch.conf`, beside the exe or in `%APPDATA%\NoteFetch`.
- `auth`: OAuth with a fresh client registration at each sign-in, the loopback callback listener and its accessible success page, and DPAPI storage in `auth.bin` (client ID and tokens).
- `plaud`: the MCP client (`initialize`, `tools/call`, plain JSON or SSE replies, and a new session after each token refresh) and typed wrappers for `list_files`, `get_file`, `get_transcript` and `get_note`. Response types come from the raw JSON saved in `test-data/` during milestone 1. Plaud times are UTC and are converted to local.
- `store`: rusqlite database in `%LOCALAPPDATA%\NoteFetch`, with migrations.
- `naming`: base names (recording name, comma, local date and time), cleaning, shortening and clash handling, and setting modified dates to the recording start time.
- `download`: streaming to `.part`, refetching expired links, retries and cancellation.
- `library`: local rename, Recycle Bin and reconciliation of the database with the disk.
- `sync`: decides what to fetch.

Rules that span several modules:

- Recordings are identified by Plaud ID everywhere: in the database, in `metadata.json`, and when restoring list selection after a refresh.
- Sync must never override the Deleted or Skipped states, and never overwrite a local name.
- A rename covers the folder and every file in it, and either completes fully or is fully undone.
- `metadata.json` keeps its fixed name so a renamed folder can still be recognised. Every other file uses the folder's base name.
- Presigned audio links count as secrets. They must not appear in the Log, `notefetch.log` or error text.
- Transcript and summary text must not appear in logs.

## Working method

- Work one milestone from TASKS.md at a time. Tick tasks off as they are done.
- At the end of each milestone, write a keyboard test script for Laurent as `test-milestone-N.txt` (with `-round-2` and so on for repeat rounds): plain text, numbered steps giving the exact keystrokes and what JAWS should say or what should happen, each step immediately before its `ANSWER:` line, and an `ANYTHING ODD:` line per section. Never put test steps in the terminal. Wait for his results in the file, and record his own words in PLAN.md for anything he decides.
- Treat hardware and screen reader results as the truth over what the code appears to do. If a result contradicts the design, record it in PLAN.md.
- Before any commit, check the staged files for secrets and personal data.
- This project does not handle client data, so Auto mode is appropriate.
