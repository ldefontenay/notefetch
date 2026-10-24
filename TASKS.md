# NoteFetch tasks

Work one milestone at a time. Each milestone ends with a keyboard test script for Laurent to run with JAWS, and the results are saved as `test-milestone-N.txt`. Do not start the next milestone until those results are in and any failures are fixed or recorded in PLAN.md.

## Milestone 0: repository

- [x] `git init` in the existing folder, add an MIT `LICENSE`, and create the private repository from it with GitHub CLI: `gh repo create notefetch --private --source . --push`.
- [x] Add SPEC.md, PLAN.md, TASKS.md, CLAUDE.md, log.md and .gitignore.
- [x] Turn on secret scanning and push protection if the account's plan offers them for private repositories. If not, add a `gitleaks` pre-commit check. (Not offered: GitHub replied "Secret scanning is not available for this repository". gitleaks 8.30.1 installed with winget; the hook is `hooks/pre-commit`, enabled with `git config core.hooksPath hooks`.)
- [x] Create a `test-data/` folder, which .gitignore excludes, for any real downloads used during development.
- [x] First commit.

## Milestone 1: spike (`spike/plaud-probe`)

A throwaway command-line program. Plain text output only, with no colour codes or decorative characters.

Much of this is already shown by plaud-cli's code (see PLAN.md), so the spike confirms it on Laurent's account and records what nobody has published. Its sign-in and MCP client code, adapted from plaud-cli, is the first draft of the real `auth` and `plaud` modules.

- [x] Fetch the discovery documents and print the key fields.
- [x] Bind a free loopback port and register a client for it. Record exactly what `/register` accepts and returns (with the client ID shortened in anything printed).
- [x] Complete sign-in in the browser, with PKCE and the `resource` value, and record the token lifetimes and any scope in the response.
- [x] First gate: the first tool call shows whether the free Starter plan has access. If refused, stop and report to Laurent.
- [x] `initialize`, then `tools/list`, and save the full tool schemas to `test-data/`.
- [x] Call `list_files` with several page sizes. Record the minimum and maximum page size, the paging behaviour, and whether paging goes past 500 recordings if the account has that many. (10 to 100; the account has 71, so the 500 question stays open.)
- [x] Record any folder or tag fields in `list_files` and `get_file` replies.
- [x] Check that times are UTC, by comparing `start_at` with a recording whose Plaud name is still its default date and time.
- [x] Call `get_file` on one recording and confirm `presigned_url` is present.
- [x] Download the audio. Record the content type, extension and size.
- [x] Call `get_transcript` and `get_note`, and save the raw JSON to `test-data/`.
- [x] Wait for the access token to expire and confirm that refresh works.
- [x] Write findings to `test-milestone-1.txt`, and update SPEC.md "Open questions" and PLAN.md.
- [x] Decision gate: if loopback redirects are refused, stop and follow the fallback in PLAN.md before going further. (Accepted first time.)
- [x] Design the organisation feature from what the server provides, add it to SPEC.md, and add a milestone for it here. (Folders mirroring File Explorer; engine work in milestone 2, interface in milestones 3 and 4.)

## Milestone 2: core engine, no interface

- [x] Cargo project at the repository root, with `build.rs` using `embed-resource` as StableSound does.
- [x] `config` module: read and write `notefetch.conf`, beside the exe or in `%APPDATA%` as a fallback.
- [x] `auth` module:
  - [x] the sign-in flow, from the spike's code adapted from plaud-cli (MIT notice kept), with a fresh registration at each sign-in;
  - [x] the loopback callback listener;
  - [x] the accessible success page returned to the browser;
  - [x] DPAPI storage in `auth.bin`.
- [x] `plaud` module: the small MCP client (plain JSON or SSE replies, the safety envelope unwrapped, re-`initialize` after a token refresh, one refresh at a time with the rotated refresh token saved at once), and typed wrappers for `list_files` and `get_file` (with `get_transcript` and `get_note` as fallbacks) with response types based on the milestone 1 JSON. Times converted from UTC to local, with unit tests around the clock changes.
- [x] `store` module: rusqlite database of recordings, with migrations. It holds:
  - the state of each part;
  - the local name alongside the original Plaud name;
  - the Deleted and Skipped states, which sync must never override.
- [x] Writing parts: audio as `.ogg`; transcript, cleaned transcript, outline and highlights as text; each note tab as Markdown named after its tab; no file for a part Plaud does not have.
- [x] `naming` module: base name building (name first, then a comma and the date and time; a trailing Plaud date and time removed from the name, and "Untitled" if nothing is left), setting modified dates to the recording start time, cleaning, shortening and clash handling, applied to the folder and every file in it. Unit tests, including reserved names, trailing dots, very long names and non-Latin characters.
- [x] `download` module:
  - [x] streaming to `.part`, with a size check and rename on completion;
  - [x] refetching an expired link;
  - [x] retries with increasing delay;
  - [x] at most two downloads at once;
  - [x] cancellation.
- [x] `library` module: local operations on downloaded recordings.
  - [x] Rename folder and files together, putting everything back if any step fails.
  - [x] Move a folder to the Recycle Bin with `IFileOperation`.
  - [x] On refresh, reconcile the database with the disk: folders missing count as deleted, and folders that reappear count as downloaded again.
  - [x] Folders: scan the output folder one level deep, recognise recordings by `metadata.json` wherever they are, create, rename and delete folders, and move recordings between them with the same all-or-nothing rule as rename.
  - [x] Unit tests for the rename rollback, the reconciliation rules and folder moves.
- [x] `sync` module: the logic for what to fetch (new, incomplete, still waiting for a transcript or summary), including the first-sync cutoff.
- [x] A hidden `--sync` developer flag that runs a sync and prints plain progress lines, so the engine can be tested before the interface exists.
- [x] `cargo test` and `cargo clippy -- -D warnings` both clean.
- [x] Also done: developer flags for rename, move, delete, skip and folders (`src/main.rs`), and `NOTEFETCH_DATA` to keep development data in `test-data`.
- [x] Test script `test-milestone-2.txt`: names and files in File Explorer with JAWS.
- [x] After the test: dates as "24 Sep 2026, 0900", files labelled first ("Transcript, name, date.txt"), and the `notes_as_txt` setting.

## Milestone 3: main window

- [x] `.rc` dialog template with the controls, tab order and Alt mnemonics from SPEC.md.
- [x] Menu bar listing every command with its shortcut.
- [x] Accelerator table for the window-wide keys only, handled before `IsDialogMessage` in the message loop. The list's own keys are handled in the list view's subclass, so they do not fire in the filter box.
- [x] List view: report view, full-row select, keeps its selection visible, newest first. Selection is kept by Plaud ID across refreshes.
- [x] Filter box, "Show" combo box, "Folder" combo box, and the Folder column.
- [x] Log edit box: read-only, multi-line, capped at a sensible length, newest entry at the top, caret at the start after every write.
- [x] A small test of `UiaRaiseNotificationEvent` from the dialog, to learn early whether JAWS speaks it.
- [x] Worker thread running the tokio runtime. Commands go in through a channel. Events come out through a queue, with `PostMessageW` sent to the dialog as a signal that events are waiting. The interface thread is the only one that touches windows.
- [x] Escape cancels the running operation and does not close the window.
- [x] Accessibility probe run on the live window: names, labels, Alt keys and roles correct (see PLAN.md).
- [x] Laurent's results in `test-milestone-3.txt`. Rows, the Log and the test announcement all read well; steps 13 to 15 were blocked by an expired sign-in.
- [x] After the test: Sign in and Sign out in the File menu (brought forward from milestone 5 at Laurent's request), the offer to sign in when the sign-in has expired, and the Ctrl+A announcement delayed so JAWS's own speech does not swallow it.
- [x] Laurent's results in `test-milestone-3-round-2.txt`: signing in from the window, and the download steps from round 1. All worked, except Escape in the Log, which closed NoteFetch.
- [x] After round 2: Escape in the Log cancels instead of closing; a sync's Log says what it will fetch and why, and folds the "checked again" lines into one; messages use the folder's own name, clash number included; `--tools` developer flag. Plaud's tools checked again: still read-only.
- [x] Laurent's results in `test-milestone-3-round-3.txt`: Escape in the Log, and the clearer sync Log. All as expected.
- [x] After round 3, from his answers: the `sync_downloads_new` setting (on by default; in the Settings dialog in milestone 5), rows kept in the list until the running command finishes, "(2)" in the Name column, and "Fetched 1 of 6" for a sync's progress.

## Milestone 4: local recordings

- [x] Default action on Enter in the list: play, download or download again, depending on status. Dialogs normally treat Enter as the default button, so the list view needs subclassing to answer `WM_GETDLGCODE` with `DLGC_WANTALLKEYS` for Enter.
- [x] Play, Open transcript and Open summary through `ShellExecuteW`, with any failure reported in the Log.
- [x] Context menu on the Applications key and Shift+F10. `WM_CONTEXTMENU` with coordinates of -1 means the keyboard opened it, so position the menu at the focused row. Items depend on status, with a default item set by `SetMenuDefaultItem`.
- [x] Recording menu in the menu bar with the same items.
- [x] F2 in-place rename (`LVS_EDITLABELS`), editing the name part only. If JAWS handles the in-place edit badly, replace it with a small rename dialog and record why in PLAN.md.
- [x] Delete key: a confirmation question (with a setting to turn it off), the move to the Recycle Bin, and the status set to Deleted.
- [x] "Don't download" for recordings never downloaded.
- [x] The Deleted view in the Show combo box, with Download again.
- [x] Copy audio file path.
- [x] Move to folder (Ctrl+Shift+M) with its small dialog, and the Folder menu (new, rename, delete).
- [x] After each of these actions, focus stays on or near the same row, and the Log says what happened.
- [x] Test script `test-milestone-4.txt` written. Built and checked on the machine first (see PLAN.md).
- [x] Laurent's results in `test-milestone-4.txt`. All 29 steps worked. Checked in particular:
  - [x] how JAWS reads the context menu and its default item ("Play Enter", read as perfect);
  - [x] in-place rename (kept; JAWS also reads the rest of the row, "not a biggy");
  - [x] focus after delete;
  - [x] rows kept in the list during a download (from milestone 3, round 3).
- [x] After the test, from his answers: renaming, moving and deleting during a sync, except on what it is fetching; Open transcript and Open summary unavailable when there is none; the `delete_marks_dont_download` setting (in the Settings dialog in milestone 5).
- [x] Laurent's results in `test-milestone-4-round-2.txt`: changes during a sync, and the menu items. All he could test worked; downloads were too quick to catch a change to a recording waiting its turn, and he is happy to leave that as it is.
- [x] His answer on telling recording folders from category folders in File Explorer: an "Uncategorised" folder (log.md, 19 October 2026), built in milestone 5.

## Milestone 5: sign-in, first sync and settings

- [x] Sign in and sign out from the settings dialog. (From the File menu, with Escape and the five-minute timeout, done in milestone 3.)
- [x] The first-sync question (whole history, last N days, or start from now).
- [x] Settings dialog as described in SPEC.md, with the Windows folder picker. Include "Download new recordings when syncing" (`sync_downloads_new`) and "Delete marks a recording never downloaded as Don't download" (`delete_marks_dont_download`).
- [x] Changing the output folder moves the recordings, after asking.
- [x] Sync on start (in place since milestone 3, now in Settings).
- [x] The "Uncategorised" folder (PLAN.md, 23 October 2026): new downloads go into it, "Not in a folder" becomes "Uncategorised" everywhere, the name is reserved, and existing recordings move into it once, safely, one at a time.
- [x] Accessibility probe run on Settings and the first-sync question: no defects.
- [x] Test script `test-milestone-5.txt` written.
- [x] Laurent's results in `test-milestone-5.txt`, and his changes (PLAN.md, 24 October 2026).
- [x] Laurent's results in `test-milestone-5-round-2.txt`, and his last three changes. Signed off (PLAN.md, 24 October 2026).

## Milestone 6: announcements, help and polish

- [x] UI Automation notification events for start, progress (throttled), completion and errors, with the three announcement levels. (Built in milestones 3 to 5; the levels are heard in `test-milestone-6.txt`.)
- [x] A sound palette in StableSound's style (PLAN.md, 23 October 2026): a first draft Laurent can hear through a developer flag, then wired in for what he did only, with a setting for sounds and volume.
- [x] Laurent's verdict on the sounds in `test-milestone-6-sounds.txt`: the sounds pass; four changes to when they play.
- [x] Laurent's results in `test-milestone-6-sounds-round-2.txt`: all seven steps as expected. The sounds are signed off.
- [x] `help.html` compiled into the exe and written out on F1: a humanised user guide with little tutorials, along the lines of Laurent's Clipman and Photo Scribe guides (PLAN.md, 23 October 2026). Semantic headings, a keyboard reference as a list, and no tables unless they have proper headers.
- [x] Error messages reviewed so that every one says what happened and what to do next.
- [x] Test script `test-milestone-6.txt` written.
- [x] Laurent's results in `test-milestone-6.txt`, with JAWS and NVDA: the guide and error messages signed off; About to become a window; NVDA heard no announcements and re-read the list row.
- [x] NVDA fixes: a live region for NVDA's announcements, the list updated in place, the list filled before the window shows, Tab confirming a rename, and About as a window with the credit (PLAN.md, "Milestone 6 results, and NVDA").
- [x] Laurent's results in `test-milestone-6-round-2.txt`: JAWS still speaks everything, and NVDA now does. Narrator speaks the announcements too. Milestone 6 signed off.
- [x] Remove "Test a spoken announcement" from the Help menu once NVDA is signed off (Laurent's wish), with the guide and SPEC.md.

## Milestone 7: release

- [x] GitHub Actions workflow on a Windows runner: build, test, clippy, and on a version tag attach `notefetch.exe` and `notefetch.zip` to a draft release. (`.github/workflows/build.yml`.)
- [x] Version resource and application manifest in the `.rc` file. (Since milestone 2; a test now keeps the version in step with Cargo.toml.)
- [x] First draft of the README in StableSound's style: the download links first, then the problem, how to use it, accessibility, antivirus and SmartScreen, what has actually been tested, files written, building, known issues, and the licence.
- [x] Write to Plaud through their developer contact page. (Decided against: Laurent will respond if Plaud gets in touch. Draft kept in `log.md`.)
- [x] Check SignPath's eligibility for free open-source code signing. (Likely eligible once public and released; see PLAN.md, 24 October 2026.)
- [x] Scan the whole git history for secrets and test data before making the repository public. (Clean on 24 October 2026, twice: no secrets, test data or recording names.)
- [x] Version 1.0.0 (Laurent, 24 October 2026).
- [x] Make the repository public. (24 October 2026, with a description and topics.)
- [x] Tag v1.0.0 and check the draft release: both files attached, version 1.0.0 in the exe, release notes written.
- [x] Laurent publishes the draft release. (Published at his request, 24 October 2026: "Publish it please.")
- [x] Draft the "Code signing policy" section in the README.
- [ ] Apply to SignPath Foundation for code signing, once released (needs Laurent's account, with MFA). Steps in `log.md`, 24 October 2026.
- [ ] Once accepted: add the SignPath signing step to the build workflow, and change the README section to "Free code signing provided by SignPath.io, certificate by SignPath Foundation".
