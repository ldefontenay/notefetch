# NoteFetch tasks

Work one milestone at a time. Each milestone ends with a keyboard test script for Laurent to run with JAWS, and the results are saved as `test-milestone-N.txt`. Do not start the next milestone until those results are in and any failures are fixed or recorded in PLAN.md.

## Milestone 0: repository

- [ ] `git init` in the existing folder, add an MIT `LICENSE`, and create the private repository from it with GitHub CLI: `gh repo create notefetch --private --source . --push`.
- [ ] Add SPEC.md, PLAN.md, TASKS.md, CLAUDE.md, log.md and .gitignore.
- [ ] Turn on secret scanning and push protection if the account's plan offers them for private repositories. If not, add a `gitleaks` pre-commit check.
- [ ] Create a `test-data/` folder, which .gitignore excludes, for any real downloads used during development.
- [ ] First commit.

## Milestone 1: spike (`spike/plaud-probe`)

A throwaway command-line program. Plain text output only, with no colour codes or decorative characters.

Much of this is already shown by plaud-cli's code (see PLAN.md), so the spike confirms it on Laurent's account and records what nobody has published. Its sign-in and MCP client code, adapted from plaud-cli, is the first draft of the real `auth` and `plaud` modules.

- [ ] Fetch the discovery documents and print the key fields.
- [ ] Bind a free loopback port and register a client for it. Record exactly what `/register` accepts and returns (with the client ID shortened in anything printed).
- [ ] Complete sign-in in the browser, with PKCE and the `resource` value, and record the token lifetimes and any scope in the response.
- [ ] First gate: the first tool call shows whether the free Starter plan has access. If refused, stop and report to Laurent.
- [ ] `initialize`, then `tools/list`, and save the full tool schemas to `test-data/`.
- [ ] Call `list_files` with several page sizes. Record the minimum and maximum page size, the paging behaviour, and whether paging goes past 500 recordings if the account has that many.
- [ ] Record any folder or tag fields in `list_files` and `get_file` replies.
- [ ] Check that times are UTC, by comparing `start_at` with a recording whose Plaud name is still its default date and time.
- [ ] Call `get_file` on one recording and confirm `presigned_url` is present.
- [ ] Download the audio. Record the content type, extension and size.
- [ ] Call `get_transcript` and `get_note`, and save the raw JSON to `test-data/`.
- [ ] Wait for the access token to expire and confirm that refresh works.
- [ ] Write findings to `test-milestone-1.txt`, and update SPEC.md "Open questions" and PLAN.md.
- [ ] Decision gate: if loopback redirects are refused, stop and follow the fallback in PLAN.md before going further.
- [ ] Design the organisation feature from what the server provides, add it to SPEC.md, and add a milestone for it here.

## Milestone 2: core engine, no interface

- [ ] Cargo project at the repository root, with `build.rs` using `embed-resource` as StableSound does.
- [ ] `config` module: read and write `notefetch.conf`, beside the exe or in `%APPDATA%` as a fallback.
- [ ] `auth` module:
  - [ ] the sign-in flow, from the spike's code adapted from plaud-cli (MIT notice kept), with a fresh registration at each sign-in;
  - [ ] the loopback callback listener;
  - [ ] the accessible success page returned to the browser;
  - [ ] DPAPI storage in `auth.bin`.
- [ ] `plaud` module: the small MCP client (plain JSON or SSE replies, re-`initialize` after a token refresh), and typed wrappers for the four tools with response types based on the milestone 1 JSON. Times converted from UTC to local, with unit tests around the clock changes.
- [ ] `store` module: rusqlite database of recordings, with migrations. It holds:
  - the state of each part;
  - the local name alongside the original Plaud name;
  - the Deleted and Skipped states, which sync must never override.
- [ ] `naming` module: base name building (name first, then a comma and the date and time; a trailing Plaud date and time removed from the name, and "Untitled" if nothing is left), setting modified dates to the recording start time, cleaning, shortening and clash handling, applied to the folder and every file in it. Unit tests, including reserved names, trailing dots, very long names and non-Latin characters.
- [ ] `download` module:
  - [ ] streaming to `.part`, with a size check and rename on completion;
  - [ ] refetching an expired link;
  - [ ] retries with increasing delay;
  - [ ] at most two downloads at once;
  - [ ] cancellation.
- [ ] `library` module: local operations on downloaded recordings.
  - [ ] Rename folder and files together, putting everything back if any step fails.
  - [ ] Move a folder to the Recycle Bin with `IFileOperation`.
  - [ ] On refresh, reconcile the database with the disk: folders missing count as deleted, and folders that reappear count as downloaded again.
  - [ ] Unit tests for the rename rollback and the reconciliation rules.
- [ ] `sync` module: the logic for what to fetch (new, incomplete, still waiting for a transcript or summary), including the first-sync cutoff.
- [ ] A hidden `--sync` developer flag that runs a sync and prints plain progress lines, so the engine can be tested before the interface exists.
- [ ] `cargo test` and `cargo clippy -- -D warnings` both clean.

## Milestone 3: main window

- [ ] `.rc` dialog template with the controls, tab order and Alt mnemonics from SPEC.md.
- [ ] Menu bar listing every command with its shortcut.
- [ ] Accelerator table for the window-wide keys only, handled before `IsDialogMessage` in the message loop. The list's own keys are handled in the list view's subclass, so they do not fire in the filter box.
- [ ] List view: report view, full-row select, keeps its selection visible, newest first. Selection is kept by Plaud ID across refreshes.
- [ ] Filter box and "Show" combo box.
- [ ] Log edit box: read-only, multi-line, capped at a sensible length, newest entry at the top, caret at the start after every write.
- [ ] A small test of `UiaRaiseNotificationEvent` from the dialog, to learn early whether JAWS speaks it.
- [ ] Worker thread running the tokio runtime. Commands go in through a channel. Events come out through a queue, with `PostMessageW` sent to the dialog as a signal that events are waiting. The interface thread is the only one that touches windows.
- [ ] Escape cancels the running operation and does not close the window.
- [ ] Test script and `test-milestone-3.txt`. Check in particular how JAWS reads list rows and columns, what Tab to the Log says, and whether the test announcement is spoken.

## Milestone 4: local recordings

- [ ] Default action on Enter in the list: play, download or download again, depending on status. Dialogs normally treat Enter as the default button, so the list view needs subclassing to answer `WM_GETDLGCODE` with `DLGC_WANTALLKEYS` for Enter.
- [ ] Play, Open transcript and Open summary through `ShellExecuteW`, with any failure reported in the Log.
- [ ] Context menu on the Applications key and Shift+F10. `WM_CONTEXTMENU` with coordinates of -1 means the keyboard opened it, so position the menu at the focused row. Items depend on status, with a default item set by `SetMenuDefaultItem`.
- [ ] Recording menu in the menu bar with the same items.
- [ ] F2 in-place rename (`LVS_EDITLABELS`), editing the name part only. If JAWS handles the in-place edit badly, replace it with a small rename dialog and record why in PLAN.md.
- [ ] Delete key: a confirmation question (with a setting to turn it off), the move to the Recycle Bin, and the status set to Deleted.
- [ ] "Don't download" for recordings never downloaded.
- [ ] The Deleted view in the Show combo box, with Download again.
- [ ] Copy audio file path.
- [ ] After each of these actions, focus stays on or near the same row, and the Log says what happened.
- [ ] Test script and `test-milestone-4.txt`. Check in particular:
  - [ ] how JAWS reads the context menu and its default item;
  - [ ] in-place rename;
  - [ ] focus after delete.

## Milestone 5: sign-in, first sync and settings

- [ ] Sign in and sign out from the menu and the settings dialog, with Cancel and timeout while waiting.
- [ ] The first-sync question (whole history, last N days, or start from now).
- [ ] Settings dialog as described in SPEC.md, with the Windows folder picker.
- [ ] Sync on start.
- [ ] Test script and `test-milestone-5.txt`.

## Milestone 6: announcements, help and polish

- [ ] UI Automation notification events for start, progress (throttled), completion and errors, with the three announcement levels.
- [ ] Optional completion tone.
- [ ] `help.html` compiled into the exe and written out on F1. Semantic headings, a keyboard reference as a list, and no tables unless they have proper headers.
- [ ] Error messages reviewed so that every one says what happened and what to do next.
- [ ] Test script and `test-milestone-6.txt`, run with JAWS and NVDA.

## Milestone 7: release

- [ ] GitHub Actions workflow on a Windows runner: build, test, clippy, and on a version tag attach `notefetch.exe` and `notefetch.zip` to the release.
- [ ] Version resource and application manifest in the `.rc` file.
- [ ] README in StableSound's style: the download links first, then the problem, how to use it, accessibility, antivirus and SmartScreen, what has actually been tested, files written, building, known issues, and the licence.
- [ ] Write to Plaud through their developer contact page: describe the app, check it is within their terms, and ask about use of the name.
- [ ] Check SignPath's eligibility for free open-source code signing.
- [ ] Scan the whole git history for secrets and test data before making the repository public.
- [ ] Make the repository public.
