# NoteFetch design record

This file records what was decided, why, and on what evidence. Read it before changing the design. Add to it as decisions change; do not rewrite history. Raw hardware and screen reader results go in `test-milestone-N.txt` files beside it.

## Decisions to date (26 September 2026)

### Rust and raw Win32, not Python and wxPython

- The first plan was a wxPython app wrapping Plaud's npm tools.
- Changed to Rust with real Win32 dialogs from resource templates, following StableSound. The controls are genuine system controls that screen readers already know, and the result is a single small exe with no runtime.
- Drawn-UI toolkits (egui, iced, Slint and similar) are ruled out, for the same reason as in StableSound.

### Hosted MCP server, not the npm MCP server or CLI

- Plaud's official MCP server (`@plaud-ai/mcp`) and CLI (`@plaud-ai/cli`) both need Node.js 20 or later. That is an unacceptable install burden for a free app aimed at blind users.
- Plaud also hosts its MCP server at `https://mcp.plaud.ai/mcp` with OAuth, for any MCP-compatible app to add by URL.
- Discovery documents fetched on 26 September 2026:
  - `/.well-known/oauth-protected-resource` gave resource `https://mcp.plaud.ai/mcp`, authorisation server `https://mcp.plaud.ai/`, and bearer tokens sent in the header.
  - `/.well-known/oauth-authorization-server` gave `/authorize`, `/token` and `/register` endpoints; the code response type; PKCE S256; token endpoint auth methods `client_secret_basic`, `client_secret_post` and `none`; and the `authorization_code` and `refresh_token` grants.
  - An unauthenticated POST to `/mcp` returned 401, with a `WWW-Authenticate` header pointing at the resource metadata.
- Conclusion: a native desktop client can register itself and sign each user in with their own account, with no shared secret in the repository.
- Evidence added 26 September 2026: plaud-cli (github.com/leegonzales/plaud-cli, Rust, MIT, Windows builds since version 0.3.2 in June 2026) does exactly this against the hosted server. It binds `127.0.0.1:0`, registers a public client for `http://127.0.0.1:<port>/callback`, and signs in with PKCE. `[reasoned]` from its published code; milestone 1 confirms it on Laurent's account.
- Fallback, if loopback redirects are refused all the same:
  1. Ask Plaud through their developer contact page.
  2. Failing that, run the npm server as a child process over stdio. Node.js would then be a documented prerequisite. This is a last resort.

### Plaud MCP is read-only

- It returns content already generated in Plaud and cannot trigger transcription or summaries. This suits the app's purpose, which is to fetch audio for transcription elsewhere.
- **Checked again on 18 October 2026,** at Laurent's request ("our knowledge of the capabilities dates from about a month ago"), with the new `--tools` developer flag. Still the same five tools: `list_files`, `get_file`, `get_note`, `get_transcript` and `get_current_user`, so renaming in Plaud is still not possible. Only `list_files` changed: `date_from` and `date_to` now filter on Plaud's server, exactly and with no 500-recording limit, while `query` still scans only the newest 500. An unofficial "plaud-connector" project on GitHub can rename by using Plaud's private app interface; NoteFetch keeps to the published MCP server.

### Sync on start plus a shortcut, no tray or timer

- Laurent's choice. It keeps the app simple and avoids unattended network activity.

### First-sync choice

- Added so a new user with a long Plaud history does not trigger an unexpected bulk download.

### UI Automation notifications for announcements, not Tolk

- They are built into Windows, so there are no extra DLLs to ship.
- To verify in milestone 5 that `UiaRaiseNotificationEvent` from a plain Win32 dialog is spoken reliably by JAWS and NVDA. If not, consider Tolk as an optional extra.
- Milestone 6: JAWS speaks them; NVDA never receives them. With NVDA running, NoteFetch uses an MSAA live region instead, still with no DLLs. See "Milestone 6 results, and NVDA".
- Everything announced is also written to the reviewable Log, as in StableSound.

### DPAPI-encrypted file for tokens, not Credential Manager

- A public app should not keep refresh tokens in plain text.
- Credential Manager has a size limit on stored secrets that OAuth tokens plus client registration may exceed. DPAPI with a per-user file has no such limit. Revisit if this proves wrong.

### Folder naming by date, time and recording name

- Confidentiality is not a concern for the intended use (lectures), and readable folder names are easier to navigate by screen reader.

### Playback in the default player, not a built-in player

- Laurent asked for downloaded recordings to be playable, to identify them.
- Version 1 opens the audio in the user's default player. Blind users have usually already chosen and configured a player that works with their screen reader. A built-in player would add another set of controls to make accessible, plus audio decoding code, for something Windows already does.
- The cost is a focus switch to the player and back (Alt+Tab).
- Revisit after real use. If a quick in-app preview proves worth it (play, pause, skip 30 seconds, all without leaving the list), Windows Media Foundation could provide it without shipping extra DLLs.
- For lectures, Open summary is often a faster way to identify a recording than listening, so it has its own shortcut.

### Enter is the default action, varying by status

- Enter plays a downloaded recording, downloads one that is not yet downloaded, and downloads a deleted one again. This follows the Windows convention that Enter carries out a menu's default item. The context menu always shows that item as its default, so the user can check what Enter will do.
- Ctrl+D remains the explicit "download all selected" command, so downloading a multiple selection never depends on which row has focus.

### Every file named after the recording, name first

- Laurent's decision: the user's name for the recording comes first and the date and time last, for example `Cognitive neuroscience week 3, 2026-09-24 1430`. Screen reader users hear the meaningful part first, and first-letter navigation works in File Explorer and file dialogs.
- The cost is that sorting by name no longer gives date order. To make up for it, the app sets each folder's and file's modified date to the recording's start time, so File Explorer's Date modified sort still works. NoteFetch's own list always sorts by date regardless.
- The comma separator reads as a natural pause in speech, where brackets or underscores may be announced.

- Audio, transcript and summary files carry the folder's base name, not generic names like `audio.mp3`. A file copied elsewhere, for example into a transcription tool, then keeps its meaning.
- `metadata.json` keeps a fixed name so the app can recognise the folder after a rename.

### Rename is local only

- The Plaud MCP server is read-only, so a rename cannot be sent back to Plaud. The original Plaud name is kept in the database and `metadata.json`.
- The date and time suffix is kept automatically.
- A rename either completes for every file or is fully undone, so a folder is never left half-renamed, for example when the media player has one file open.

### Delete to the Recycle Bin, and remember it

- Deleted recordings go to the Recycle Bin, which gives Windows' own undo as well as the app's Deleted view.
- The database keeps them as Deleted, so sync does not bring them back.
- Folders deleted outside the app are treated the same way, because that is almost certainly what the user meant. Folders that reappear, for example restored from the Recycle Bin, are marked as downloaded again.
- The Deleted view is a choice in the Show combo box rather than a separate window, so the same list, keys and context menu apply.

### The Log shows the newest entry first

- Decided by Laurent on 26 September 2026, answering where the caret should sit in the Log: "Put the newest entry at the top."
- New lines are inserted at the start of the Log edit box, and the caret is set to the start after every write (`EM_SETSEL` 0, 0 and `EM_SCROLLCARET`). The first line read is then always the latest event, and the down arrow goes back in time, as in `log.md`.
- `[untested]` with JAWS in NoteFetch. The TweakSound finding that a multi-line edit is not read out when it gets focus still applies, so milestone 3 checks what Tab to the Log says.

### Changes after the plan review and GitHub search (26 September 2026)

Proposed in `log.md` and accepted by Laurent: "Happy with all of them."

#### Our own small MCP client, not rmcp

- Laurent agreed ("Yes") to drop rmcp. NoteFetch needs four tool calls, rmcp's OAuth API has changed between versions, and rmcp would bring in more code than the rest of the app.
- plaud-cli shows the whole client (discovery, registration, PKCE, loopback listener, token exchange and refresh, `initialize`, `tools/call`, plain JSON or SSE replies) in about 500 lines.
- Laurent agreed ("Yes") to borrow from plaud-cli with credit: keep its MIT copyright notice in each file that takes code from it, and credit it in the README.
- Dependencies approved for this: reqwest, sha2, base64, rand and url, alongside tokio, serde and serde_json. Use reqwest's default TLS, which is Windows' own (schannel), so there is no extra C build and no certificate bundle to ship.
- Cost: changes to the MCP protocol must be followed by hand. The client sends `MCP-Protocol-Version: 2025-06-18`, as plaud-cli does.

#### Register a new client at each sign-in

- Bind a free loopback port, then register a client for that exact redirect address. This avoids the question of whether the port can change after registration. plaud-cli's reason: "to dodge loopback port-mismatch issues".
- The client ID is stored in `auth.bin` with the tokens, because refresh needs it.
- The authorisation request carries `resource=https://mcp.plaud.ai/mcp` (RFC 8707).
- After a token refresh, drop the MCP session and run `initialize` again before retrying, because the server ties the session to the token.

#### Plaud times are UTC

- Plaud returns times with no zone. plaud-mcp-connector and plaud-obsidian-sync both found they are UTC. Convert to local time for folder names and the Date column, with unit tests either side of the clock changes in March and October. `[reasoned]`; milestone 1 checks it against a recording's default name, which is its local start time.

#### Plaud's default names are the date and time

- Laurent's examples from the Plaud web app: a renamed recording, "Staff training 2026-09-24 09:00:26" (the name changed here to keep work details out of the repository), and an unrenamed one, "2026-09-09 17:27:14".
- So the date and time appear in the Plaud name already, and the folder suffix would repeat them. Rule: remove a trailing `YYYY-MM-DD HH:MM:SS` (or `YYYY-MM-DD HH:MM`) from the Plaud name before building the base name. If nothing is left, the name is "Untitled", giving for example `Untitled, 2026-09-09 1727`. `[reasoned]`: untitled recordings then sit together under U in File Explorer, which invites renaming them. Laurent can overrule.
- The Plaud name is kept unchanged in the database and `metadata.json`.

#### List statuses use "No transcript" and "No summary"

- Laurent's choice: "Rather use 'no summary' or 'no transcript'." The statuses are "Downloaded", "Not downloaded", "No transcript", "No summary", "Audio only" (neither), "Failed", "Skipped" and "Deleted". "Waiting for transcript" is gone. Sync still retries missing parts for the set number of days, whatever the status says.

#### Keys that belong to the list are handled by the list

- Enter, Delete, F2, Ctrl+A, Ctrl+D, Ctrl+T, Ctrl+M and Ctrl+Shift+C are handled in the list view's subclass, not the window-wide accelerator table. Otherwise Delete would recycle a recording while typing in the filter box, and Ctrl+A would select every recording instead of the filter text. The accelerator table keeps only the window-wide keys.

#### Spoken announcements tested in milestone 3

- PLAN.md said milestone 5 and TASKS.md said milestone 6. Moved to a small test in milestone 3, when the main window first exists, because the announcement design depends on whether JAWS speaks `UiaRaiseNotificationEvent` from a plain Win32 dialog. Neither StableSound nor TweakSound tested this.

#### Never show a raw server reply

- When sign-in, refresh or a tool call fails, say what happened in plain words. Never show or log the server's raw reply, which is unreadable and might echo back part of what was sent. The details that are safe (HTTP status, error code such as `invalid_grant`) go in the Log.

#### Plaud plan

- Laurent is on the free (Starter) plan. Plaud's pricing page lists "Plaud MCP server (Beta)" for Starter, Pro, Unlimited and Team. A third-party article and plaud-obsidian-sync say Pro or above is needed. Milestone 1's first live call settles it; if Starter is refused, stop and tell Laurent.

#### Organising recordings

- Laurent, asked whether he uses Plaud folders: "I would like to be better organised, so please implement relevant features."
- The documented `list_files` parameters are `query`, `date_from`, `date_to`, `page` and `page_size`, with no folder or tag. Milestone 1 checks whether the raw replies carry folder or tag fields anyway. The organisation feature is designed after that, in its own milestone, from what the server actually provides. If it provides nothing, NoteFetch can offer local groups (for example one per course) with a subfolder each.

### What milestone 1 proved (26 September 2026)

Full findings in `test-milestone-1.txt`. The ones that change the design:

- **The free Starter plan works.** `[proven]`
- **Loopback sign-in with a fresh registration works first time.** `[proven]` The fallback to the npm server is no longer needed.
- **Refresh tokens rotate.** `[proven]` Every refresh returns a new refresh token. Save it at once, and let only one refresh run at a time, or two parallel downloads could each refresh and one would be left holding a dead token.
- **Replies come in a safety envelope** with a random tag. `[proven]` plaud-cli predates it and reads nothing from the current server, so code borrowed from it must unwrap the envelope.
- **`get_file` returns everything.** `[proven]` Notes as Markdown and the full transcript arrive with the audio link, so a sync needs `list_files` plus one `get_file` per recording. `get_transcript` and `get_note` stay as fallbacks.
- **Audio is Ogg Opus, not MP3.** `[proven]` About 34 kilobits per second, mono. The `.mp3` in the original spec was an assumption. Which extension to use is being asked.
- **No folders or tags are exposed.** `[proven]` Organisation has to be NoteFetch's own. Proposed to Laurent in `log.md`.
- **Times are UTC; default names are local.** `[proven]`
- **Transcript segments have end times.** `[proven]` Contrary to another project's report.

### Answers after milestone 1 (26 September 2026)

- **Audio saved as `.ogg`.** Laurent: "Yes, both played. Agreed on using .ogg". Upload forms accept `.ogg` more often than `.opus`.
- **Transcription happens in Heidi Health.** Laurent: "I'll be uploading lectures to Heidi Health to transcribe, using a lecture template I've set up there." Heidi's public help and developer pages do not list accepted audio formats (checked 26 September 2026). Laurent is asked to try uploading the sample. If Heidi refuses Ogg Opus, the fallback is an optional "also save as MP3" setting that uses FFmpeg when it is installed (it is on Laurent's machine), not a converter shipped inside NoteFetch, which would make the exe many times larger.
- **Folders, one per recording, mirroring File Explorer.** Laurent: "Folder is good, and ideally what shows in the app will also mirror what shows in Windows Explorer, so tags may be a bit much." So the disk is the source of truth: a folder is a subfolder of the output folder, and NoteFetch reads the folder structure from disk on every refresh rather than keeping its own copy that could disagree. One level only, to keep both the list and File Explorer simple to move around by keyboard. `[reasoned]`
- **Save every part Plaud has, and nothing empty.** Laurent: "Everything for now please. No empty placeholders for recordings where these items don't exist." So the cleaned transcript, outline and highlights are saved when present. Highlights arrive in `get_file`'s `source_list` only when the recorder's button was pressed, so no extra call is needed.

### Milestone 2: the engine (27 September 2026)

- **Built and tried live** on a development copy (`NOTEFETCH_DATA=test-data\dev`, output in `test-data\output`): listing all 71 recordings, a first sync of the last 10 days (2 recordings, 15 seconds), rename, folder create, rename and delete, moves, a recording moved by hand and found on refresh, and Don't download. `[proven]` on the machine, not yet by Laurent.
- **Web requests use Windows' own TLS (schannel).** reqwest 0.13 changed its default to rustls with aws-lc-rs, which needs extra build tools; `native-tls` is set explicitly, as this plan intended.
- **Local time through Windows' zone rules** (`SystemTimeToTzSpecificLocalTimeEx`), not a time crate, so there is no zone database to ship. Tests pin the UK zone and cover both clock changes of 2026.
- **The default output folder is `Documents\NoteFetch`**, wherever Windows has put Documents (it may be in OneDrive). `[reasoned]`: it is where a user looks first. Changeable in Settings.
- **Release exe is 2.5 MB**, against StableSound's 1 MB. Most of it is SQLite, compiled in, and the HTTP stack. Acceptable for a single-file app; revisit only if it matters to anyone.
- **Not yet tried:** cancelling a download part way (the code path exists and removes the `.part` file; the window's Escape will exercise it in milestone 3). A process killed mid-download leaves a `.part` file until that recording is next fetched, when it is overwritten.

### What Laurent's milestone 2 test settled (27 September 2026)

Full answers in `test-milestone-2.txt`. This replaces the naming in "Every file named after the recording, name first" above, which stays as the record of what was first decided.

- **Dates in names are "24 Sep 2026, 0900".** Laurent: "\"24 Sep 2026, 09:00\" reads well with jaws. The letters and comma prevent it from sounding like number salad." The colon is left out because Windows forbids it in names. `[proven]` that JAWS reads the old ISO form as numbers; the new form is `[reasoned]` until he hears it.
- **Each file starts with what it is.** Laurent: "Please follow this naming convention, making it quicker for a screenreader user to find what they're looking for: \"Transcript, [name], 24 Sep 2026, 0900.txt\"". So every file is "Label, base name.ext": Audio, Transcript, Cleaned transcript, Outline, Highlights, Summary, or a note tab's own name. The audio follows the same pattern as the rest, `[reasoned]`: consistent, and A for Audio works with first-letter navigation.
- **Commas as pauses:** "Commas are good."
- **Name first works:** pressing T went straight to "Test lecture". `[proven]`
- **Date modified shows the recording start.** JAWS read 24/09/2026 09:00 and 21/09/2026 16:01. `[proven]`
- **Transcript layout stays** (time, speaker, text): "The current format is fine."
- **The outline is worth keeping:** "Useful, could help if I wanted to review bits of the actual recording."
- **Markdown summaries open in Laurent's own program** (Leasy Word). He asked for a setting: "provide an option to save files as a .txt (essentially just changing the extention), keeping .md as the default". Added as `notes_as_txt`.
- **Heidi Health accepts the .ogg audio.** Laurent: "I've tested the .ogg file with Heidi and it works perfectly." No MP3 conversion is needed.

### Milestone 3: the main window (27 September 2026)

- **Built from the template, as planned.** A modeless dialog with a menu and an accelerator table, pumped by our own loop: accelerators, then `IsDialogMessageW`. The list's keys (Ctrl+A, Ctrl+D, Ctrl+Shift+O) are handled in a subclass of the list, so Ctrl+A in the filter box selects the text.
- **Alt letters shared with the menu bar.** Controls and menu titles both answer Alt+letter in one window, so the menu took F, R, L, A and H, and the controls S, E, N, C, Y, D, O and G. A unit test reads `notefetch.rc` and fails if any letter is used twice.
- **The probe, run on the live window:** every control is named by its label, every Alt key is present and unique, and the list rows carry their other columns as the description ("Date: ..., Duration: ..., Status: ..."). `[proven]` by the probe. Its one warning: ten rows are all named "Untitled", which a listener hearing only names cannot tell apart. The description carries the date, so whether this matters depends on how JAWS reads rows; asked in `test-milestone-3.txt`.
- **Focus in the list is remembered by Plaud ID** from the list's own change notifications, not read back from the list, because rebuilding the list moved its focus before it could be restored (seen once on the live window: focus landed on the second row). The fix is `[untested]` live: the check was stopped because Laurent was using the computer, and a test window takes the foreground.
- **The Log reports only what changed.** A recording checked again for a missing transcript now logs "Checked ... again: Plaud has no transcript or summary for it yet." rather than "Downloaded", and the final line of a sync is no longer logged twice.
- **Announcements** use `UiaRaiseNotificationEvent` on the provider from `UiaHostProviderFromHwnd`. `[untested]` with JAWS; the Help menu has "Test a spoken announcement" for this round.
- **Known limits in this version:** no sign-in from the window, no first-sync question, no Settings, no Help file, and Enter in the list does nothing. The worker's "not signed in" message tells the user to use a Sign in command that does not exist until milestone 5.
- **Testing on the development data:** `test-data\Start NoteFetch test.cmd` sets `NOTEFETCH_DATA` and starts the release build, so Laurent can test without a sign-in dialog.

### What Laurent's milestone 3 test settled (18 October 2026)

Full answers in `test-milestone-3.txt`. Steps 13 to 15 (downloading, progress, Escape, and closing during a download) could not be run, because the sign-in had expired and the window had no Sign in command.

- **JAWS speaks `UiaRaiseNotificationEvent` from the plain Win32 dialog.** `[proven]` He heard the start-up error "Your Plaud sign-in has expired..." and the Help menu's test announcement. The design in "UI Automation notifications for announcements" stands, and Tolk is not needed for JAWS. JAWS speaks the announcement after its own speech: "The line of the currently focussed recording was spoken, followed by the announcement. Presumably this is fine." Left as it is.
- **List rows read well.** `[proven]` JAWS reads the name, then Folder, Date, Duration and Status, without the column titles. Laurent: "The information is very good as presented."
- **The Name column stays as it is**, without the date. Laurent: "Fine as is for now."
- **Ctrl+A's announcement was not heard.** `[proven]` JAWS read the name of every recording as it was selected, and "All 71 recordings selected." only reached the Log. Laurent: "The expected message wasn't spoken, which would have been better, although its still clear what happened." Changed: the announcement now waits 300 ms after the selection. `[untested]`
- **Tab order, Alt keys, menus, Show, Folder, filter and the Log** all worked as designed. `[proven]` On the menus: "It behaves like a model accessible app. really good." Ctrl+A and Delete in the filter box acted on the text, not the recordings, so handling the list's keys in its subclass works. `[proven]` The Log reads newest first, line by line. `[proven]`
- **Focus on reopening** landed on the first recording. `[proven]` The fix recorded under milestone 3 holds.
- **A refresh token lasts less than three weeks unused.** `[proven]` The development sign-in, last used on 27 September, was refused on 18 October. Plaud does not say how long it lasts. Anyone who opens NoteFetch only now and then will meet this, so signing in again has to be easy.

### Sign in and Sign out brought forward from milestone 5 (18 October 2026)

- Laurent, at the end of the milestone 3 test: "I'll complete this test round once I can sign the app back into Plaud again, please build this functionality." So the File menu now has "Sign in to Plaud..." and "Sign out of Plaud", ahead of the rest of milestone 5 (the first-sync question and Settings).
- **When a command finds the sign-in missing or expired, NoteFetch asks** "Your Plaud sign-in has expired. Sign in again now?", with Yes already chosen. SPEC.md says the app "offers to do it", and this is a question the user must answer, so a message box is the right place for it. The error is still announced and logged first. After a failed sign-in it does not ask again, which would only go round in a circle.
- **While waiting for the browser,** Escape cancels, and it gives up by itself after five minutes, as SPEC.md asks. The Log and an announcement say the page is open in the browser.
- **After signing in, NoteFetch syncs** when sync on start is set, since that sync could not run at start-up.
- **Sign out does not ask first.** `[reasoned]`: it deletes nothing but the stored sign-in, and signing in again undoes it.
- **Closing asks only during a download or a sync,** the work that writes files. Closing while waiting for sign-in, connecting or refreshing just stops it. The sync question says "syncing" rather than "downloading".
- Not tried live: completing a sign-in needs Laurent's Plaud account in the browser, and opening the window would take the focus while he may be using the computer. Built and checked by tests and clippy only.

### What Laurent's milestone 3 round 2 test settled (18 October 2026)

Full answers in `test-milestone-3-round-2.txt`.

- **Signing in from the window works.** `[proven]` The expired sign-in question at start-up: "All as expected and the question was clear." The browser page and "Signed in to Plaud" page were clear. The announcements after sign-in were spoken while the browser still had the focus: "which is very good". Sign out, saying No to the sign-in question, and cancelling a sign-in with Escape all worked as designed.
- **Escape in the Log closed NoteFetch.** `[proven]` During a sign-in, Escape with focus in the Log "completely quits with no message instead of cancelling". Cause: a multi-line edit box asks the dialog for every key (`DLGC_WANTALLKEYS`), and on Escape it sends its parent `WM_CLOSE`. Closing does not ask while only signing in, so NoteFetch closed. Fixed by subclassing the Log so its `WM_GETDLGCODE` answer leaves Escape to the dialog, which turns it into Cancel as everywhere else. `[proven]` by a posted Escape to the Log: without the fix the window closed; with it, the Log said "Nothing is running, so there is nothing to cancel." Not yet heard by Laurent.
- **Ctrl+A.** `[proven]` With the 300 ms wait, "All 79 recordings selected." is now spoken, but only after JAWS has read every name. Laurent: "fine as a second-class non-dealbreaking behaviour as is and screenreader users would be used to this. First class would be to have the message ... immediately ... a bonus, not essential." Measured with a WinEvent hook: `LVM_SETITEMSTATE` with index -1 fires one `EVENT_OBJECT_SELECTIONADD` per row (78 for 79 rows), and `WM_SETREDRAW` off does not stop them. Rebuilding the list with every row inserted already selected fires the same 78 and more besides. Both tried and dropped. Left as it is; a virtual (owner-data) list might fire a single event instead, `[untested]`, worth trying only if the list is rebuilt for another reason.
- **Downloads, progress, Escape and closing during a download** work as designed. `[proven]` On progress every 5 seconds: "Very good. Messages just as expected and progress announcements timed well." Focus after a download in "Not yet downloaded" moved to the next recording as the downloaded one left the view.
- **A sync downloads new recordings,** as SPEC.md says. Laurent: "All new recordings were actually downloaded, as opposed to just syncronised. Not terrible, but still thinking through whether this is desireable behaviour or not." Asked in log.md; unchanged until he answers.
- **The Log during a sync was confusing.** Laurent: "I find the log a bit confusing." It said "None are new" and then downloaded two recordings, which were downloads his Escape had interrupted earlier, now being finished. It also named "Untitled, 15 Oct 2026, 1229" twice: two different recordings started in the same minute, whose folders are told apart only by "(2)". Changed:
  - after "Plaud has 79 recordings", a line saying what the sync will fetch and why, for example "Fetching 8 recordings: 2 not finished last time, and 6 to check again for a transcript or summary.";
  - in a sync, the "Checked ... again" lines become one: "Checked 6 recordings again for a transcript or summary: Plaud has nothing new for them yet." Download selected still gives each its own line, since the user picked them;
  - messages name a downloaded recording by its folder's own name, so the clash number "(2)" is included.
- **Pressing No to the close question.** Laurent: "I think that the total of recordings to be downloaded refreshed after pressing no, which may be a bit confusing." Not yet understood: the progress total does not change during a download, so this may be the list's own count shrinking in the "Not yet downloaded" view. Asked in log.md.

### What Laurent's milestone 3 round 3 test settled (18 October 2026)

Full answers in `test-milestone-3-round-3.txt`, and his answers to the two questions in log.md.

- **Escape in the Log now cancels.** `[proven]` Idle, it said "Nothing is running..." and focus stayed in the Log; during a sign-in and during a download it cancelled and NoteFetch stayed open.
- **The sync Log makes sense.** Laurent: "It makes more sense now." One wording fix: a sync that only checked recordings again said "Downloaded 1 of 6." Progress now says "Fetched" during a sync and "Downloaded" only for Download selected.
- **Recordings started in the same minute.** The Log lines with "(2)" were "very helpful". He asked: "It would be helpful to have the (2) in the title as well." The Name column now adds the folder's clash number: "Untitled (2)". When F2 rename arrives in milestone 4, it must edit the name without the clash number.
- **Downloading new recordings when syncing becomes a setting, on by default.** Laurent, choosing option C: "Yes, go with C with downloading on by default." `sync_downloads_new` in `notefetch.conf` now; in the Settings dialog in milestone 5. Off, a sync still finishes interrupted downloads and checks for transcripts, retries earlier failures, and says the new ones are in "Not yet downloaded".
- **Rows stay in the list until the command finishes.** On the shrinking list count during a download, Laurent: "Yes, that's probably what I heard. And it would make sense for the downloading recording to remain in the view until its download is complete." Read as: rows in the list when a command starts stay there, with their status updated, until it finishes, and then the view is applied again. Changing the Show or Folder choice or the filter meanwhile drops them. `[untested]` with JAWS; check in the milestone 4 round, including that focus still moves to the next recording afterwards.

### Milestone 4: local recordings (18 October 2026)

Built and checked on the machine by tests (74), clippy, the probe, posted messages and one short run of real keystrokes, on a separate copy of the test data with no sign-in and announcements off. Nothing here is yet heard by Laurent; `test-milestone-4.txt` checks it.

- **Renaming, moving, deleting and folders need no sign-in.** They work on the disk and the database alone (`actions::Local`), so they run on the worker without connecting to Plaud. `[proven]`: the probe copy had no sign-in. The developer flags for them no longer need one either.
- **They waited for a sync or download to finish.** Replaced after the test round: see "Changes during a sync" below.
- **Enter in the list.** The list's subclass answers `WM_GETDLGCODE` with `DLGC_WANTALLKEYS` for Enter, so the dialog manager hands it to the list instead of looking for a default button. `[proven]` by a posted Enter.
- **In-place rename in a dialog.** The list's edit box also has to answer `DLGC_WANTALLKEYS`, or the dialog manager takes Enter as OK and Escape as Cancel (which would cancel whatever is running, not the edit). Subclassed on `LVN_BEGINLABELEDIT`, removed on `WM_NCDESTROY`. The accelerator table is skipped while the edit box exists, so Ctrl+S or F5 cannot start something mid-word. `[proven]` by posted keys: Enter renamed, Escape cancelled the edit only and left the Log alone. The edit box holds the name part alone, without the date or "(2)". How JAWS reads it is `[untested]`, the point of the test round; the fallback is the small name window already built for folders.
- **A clash number only while needed.** This replaces "keep a clash number if the old name had one" in the milestone 2 code. Renaming "Untitled (2)" to "Seminar" now gives "Seminar, 15 Oct 2026, 1229", not "... (2)", unless a "Seminar" of the same minute is already there. Found in the probe run, where the kept "(2)" no longer told anything apart. `[reasoned]`; ask Laurent.
- **The context menu.** The list posts its `WM_CONTEXTMENU` to the dialog, which builds the menu from the focused recording and shows it with `TrackPopupMenu` outside the window's state, so worker events still get through. From the keyboard (coordinates -1) it opens at the focused row's label. The default item is set with `SetMenuDefaultItem`. `[proven]` with real Shift+F10, and Escape closed it without cancelling anything. Whether JAWS says which item is the default is `[untested]`.
- **Several selected.** Enter does the focused recording's default action. When that is Download or Download again, every selected recording is downloaded, as File Explorer's Enter opens every selected file; Play plays only the focused one. In the menu, the default item stays available, so the menu always shows what Enter does. `[reasoned]`, recorded in SPEC.md.
- **Delete on a recording never downloaded marks it Don't download,** with no question, since nothing on disk is lost and Download again undoes it. Mixed selections do both; the question counts them. `[reasoned]`.
- **The delete question has Yes as its default,** as File Explorer's does, because the Recycle Bin can undo it. `[reasoned]`; ask Laurent. Probe: "Yes" focused and default, then the question's text.
- **After a delete, the next recording is focused and selected.** Found by the probe run: the rebuilt list focused the next row but selected nothing, because the old selection had gone, so the next Delete or Ctrl+Shift+M said nothing was selected. `[proven]` fixed.
- **The Recording menu in the menu bar** is set up on `WM_INITMENUPOPUP` from the same rules as the context menu, so unavailable items are disabled and the default is bold. **The Folder menu** names its folder: "Rename the folder "Neuroscience"...", or "Rename folder (choose one in the Folder box first)" when disabled, so a disabled item explains itself. `[proven]` by reading the menus after `WM_INITMENUPOPUP`.
- **Move to folder** starts on where the recordings are now, so the first thing heard says where they are. A typed name that matches an existing folder means that folder. Probe: "Folders:" list and "New folder name (optional):" edit both named, Alt+F and Alt+N.
- **Opening files** uses the file's default program (`ShellExecuteW` with no verb), and if none is set, Windows' Open With (`openas`). COM is now initialised on the interface thread, which the shell expects for this. Play is logged ("Playing ...") but not spoken, since the player takes over.
- **A file open in a player.** Windows' "in use" errors (32 and 33) now get their own advice: "A file in it is open in another program, such as a media player. Close it and try again." Before, they could fall through to "restart Windows". `[proven]` by the rename rollback test, which holds a file open.
- **Clipboard.** Needed the `Win32_System_DataExchange` feature of the `windows` crate: a feature of an existing dependency, not a new one.
- **The skill** gets the dialog findings (Enter for a list, in-place editing in a dialog) once Laurent has heard them.

### Milestone 4 results (19 October 2026)

Laurent ran all 29 steps of `test-milestone-4.txt` with JAWS. Everything worked as expected. His words, on what was in question:

- **The context menu and its default item** `[proven]`: "The first item says "Play	Enter p", perfect. The rest of the items make complete sense, including their shortcuts." The Applications key and Shift+F10 gave the same menu: "Yes, excellent." With several selected: "Yes, very well thought through."
- **In-place rename** `[proven]`, so no rename dialog is needed: "Jaws says "edit test lecture", followed by the rest of the line. Reading the current line does indicate that "test lecture" is the only thing in the edit box. Ideally that would be the only thing being read, but not a biggy." The rest of the row is JAWS reading the list row behind the edit box; nothing in our control is known to stop it.
- **Focus after delete** `[proven]`: "Focus landed correctly on the next recording." After Don't download: "Focus correctly jumped to the next recording."
- **Rows kept during a download** `[proven]`: "Yes."
- **A rename while Windows Media Player plays the file** succeeded and stopped the playback: "This is ok."
- **"(2)" dropped on a rename to a name that does not clash:** "Yes, that's right."
- **Yes as the delete question's default:** "Keep the Windows Explorer convention."
- **Changes during a sync:** "If possible, allow actions on recordings unaffected by the sync." See "Changes during a sync" below.
- **Delete on a recording never downloaded:** "Delete moves to "don't download" on such recordings by default, but make that a setting so that optionally delete does nothing." The setting is `delete_marks_dont_download`, on by default.
- **Open transcript and Open summary when there is none:** "it would be elegant for the summary and transcript options to be indicated as unavailable for downloaded recordings with no summaries or transcripts, in both the context menu and recording menu. The error message is still perfect for when ctrl+m or ctrl+t are pressed." Done: the items are unavailable, and the keys still explain.
- **Recording folders and category folders look alike in File Explorer:** "it would be helpful to have some distinction between what is a folder containing recordings and what is a category folder containing subfolders with recordings. ... Maybe have an "Uncategorised" folder for uncategorised recordings?" Ideas and a question are in log.md (19 October 2026).

### Changes during a sync (19 October 2026)

Laurent: "If possible, allow actions on recordings unaffected by the sync."

- **The worker's loop no longer waits on the network.** Sign-in, sync, refresh and download run as tokio tasks of their own; the loop goes straight back to its channel, so a rename or delete is taken at once. The window still starts only one network command at a time, as before. The loop keeps its own database connection; the tasks use theirs, with SQLite's five-second busy wait between them.
- **A guard keeps the two apart** (`sync::Guard`). Each change holds a disk lock while it runs. A fetch claims its recording under the same lock just before it starts and releases it when it ends, so a change never starts on a recording being fetched, and a fetch never starts in the middle of a change. Reconciling holds the lock too, because a rename half way through the scan could look like a deletion.
- **What is refused:** renaming, moving, deleting or Don't download on a recording being fetched at that moment, and renaming or deleting a folder that holds one. The Log names it and says to try again when the download finishes. A recording only waiting its turn is not refused.
- **Deleted while it waited:** a recording deleted or marked Don't download while it waits its turn is left out, and the Log says so. A recording that was already Deleted when asked for (Download again) is still downloaded.
- **The window** no longer counts a change as "something running": no `busy` flag, no Escape to cancel it, and a deleted or moved recording leaves the view at once even while a download keeps the view's rows.
- `[reasoned]`, with the guard unit-tested and a rename driven through the new loop on the probe copy. A real sync alongside a change needs a sign-in, so it is `[untested]` until Laurent's round 2.

### Milestone 4 round 2 results (23 October 2026)

Laurent ran `test-milestone-4-round-2.txt`. Everything he could test worked. Downloads were too fast to catch most of section 2. His words:

- **A rename straight after start-up:** "The sync happened to quickly. The rename worked fine." So a rename during the sync on start is still `[untested]`, but nothing is lost: it no longer waits.
- **Open transcript and Open summary unavailable when there is none,** in both menus, and available when there is one `[proven]`: "Yes."
- **A recording being downloaded is refused** `[proven]`: "I tried with one longer download again. It spoke the expected message and the downloaded wasn't deleted, just disappeared from the "no downloads" view soon after, with the download having completed."
- **Deleting or moving a recording waiting its turn, and the "Did not download" line** stay `[reasoned]`, covered by unit tests only: "Its amazing how quicly the downloads went. I couldn't test it propperly. Happy to leave as is."
- **Moving between folders** `[proven]`: "Moving between folders works well, I tested it several times now."
- **`delete_marks_dont_download = no`** `[proven]`: NoteFetch started "perfectly", and the message on Delete was clear.

### Uncategorised folder (23 October 2026)

Laurent, in milestone 4: "it would be helpful to have some distinction between what is a folder containing recordings and what is a category folder containing subfolders with recordings. ... Maybe have an "Uncategorised" folder for uncategorised recordings?" Offered three options in log.md (19 October), he chose: "Yes, go with option 1, and "uncategorised" is the right name."

- **A real folder called "Uncategorised"** in the output folder holds every recording not in a category. The output folder then holds only folders of folders, and each of those holds only recordings, so File Explorer shows the difference by where you are.
- **"Not in a folder" becomes "Uncategorised"** everywhere in NoteFetch, so the app and File Explorer use the same word.
- **It is always on**, not a setting, so the layout is the same for everyone and the help can describe it simply.
- **"Uncategorised" is reserved:** no category can be given that name (in any letter case), and it cannot be renamed or deleted from the Folder menu.
- **The one-off move** of existing recordings into it runs at the first refresh after the change, one recording at a time, with the same all-or-nothing rule as Move to folder. A recording whose files are open in another program stays where it is, and the Log says so; the next refresh tries again.
- **A recording found directly in the output folder later**, for example moved there in File Explorer, is moved into Uncategorised by the next refresh, and the Log says so. `[reasoned]`: the alternative, leaving it there and listing it as Uncategorised, would make the app and File Explorer disagree. Asked in log.md.
- Built in milestone 5, because it changes the output folder's layout.

### A sound palette (23 October 2026)

Laurent, after round 2: "I'd like my apps, including this one, to use cute distinct little sounds similar to those used in StableSound (check them out). This makes the app feel friendlier. Please include in the plan to design a little sound pallet for NoteFetch at a logical juncture."

- **Where:** milestone 6, in place of the single optional completion tone, alongside the announcements, since sounds and announcements answer the same events and are best designed together.
- **How StableSound does it:** short sine notes with a few milliseconds of fade at each end so they do not click, two notes rising for on and falling for off, at a volume of about 0.1 (`stablesound/src/audio/earcon.rs`). TweakSound uses the same two pitches an octave lower, and StableSound's quick release one 880 Hz note.
- **How NoteFetch would do it, with no new dependency:** build each sound in code as a small WAV in memory, from a list of notes in the same style, and play it with `PlaySoundW` and `SND_MEMORY | SND_ASYNC` (the `Win32_Media_Audio` feature of the `windows` crate). NoteFetch does not hold an audio stream open, so StableSound's Bluetooth lead-in problem may show up: the first sound after a silence could be clipped on a Bluetooth headset. `[untested]`; a short silence before the notes is the known fix.
- **Pitches of its own,** away from StableSound's and TweakSound's E and B and from StableSound's A, so the apps cannot be confused when they run together.
- **Sounds only for what Laurent did**, the proven rule from StableSound: a sync he started, a download, a rename, a move, a delete. The sync on start and any automatic refresh stay silent unless something goes wrong.
- **A first draft of the palette,** to be heard before it is wired in: done (a rename, move or delete worked), finished (a download or sync he started has finished), refused (something could not be done, with the reason in the Log), and attention (a question, or a sign-in is needed). A hidden developer flag will play each one, so he can judge them in one sitting.
- **A setting** for sounds on or off, and a volume.

### A user guide with tutorials (23 October 2026)

Laurent, after round 2: "I want my apps to be easy for new users to get to grips with. ... I'd want Notefetch to have a user guide written along similar principles, very humanised, with worked examples of how to use the app in little tutorials." His models are "Getting Started with Clipman" (`GettingStarted.html` on the `getting-started-guide` branch of his Clipman fork) and Photo Scribe's `src/help.html`.

The principles taken from them, for milestone 6's `help.html`:

- **The opening:** a one-line welcome, the byline "By Laurent Cadet de Fontenay, with assistance from Claude", a contents list, then "About this guide" (you do not need to read it all; every tutorial stands on its own; headings are your friend; names on screen in quotation marks, exactly as the screen reader says them; menus written with a comma, such as "Recording menu, Rename"; screen reader keystrokes left out; F1 opens this guide), and the version it was written for.
- **"Why use NoteFetch?"** in human terms first: the problem it solves, then how, then "work through Tutorials 1 and 2 and you will have ...".
- **"What NoteFetch does"** and **"Words you will meet"**: recording, transcript, summary, sync, download, category folder, Uncategorised, the Deleted view, Don't download.
- **Parts as level 2 headings, tutorials as level 3,** each with "What you will learn", "Worked example" (a believable scene, such as a week of lecture recordings), numbered steps saying what to press and what you will hear, and "What just happened".
- **"The sounds NoteFetch makes",** a troubleshooting part, a quick reference of keys, the files NoteFetch keeps, and credits and licence.
- Plain words, warm and direct, UK English, no em dashes, no tables unless genuinely tabular with headers.

### Milestone 5: the Uncategorised folder, Settings and the first sync (23 October 2026)

Built and checked on the machine by tests (81), clippy, the probe and posted messages, on a separate copy of the test data with no sign-in, no audio and announcements off. Nothing here is yet heard by Laurent; `test-milestone-5.txt` checks it.

- **Uncategorised, as designed above.** New downloads go into it. At each start and refresh, a recording found directly in the output folder moves into it (`Library::tidy`), one at a time with the same rename as Move to folder, skipping any being downloaded. `[proven]` on the probe copy: "Moved 50 recordings into the Uncategorised folder, so the output folder holds only folders." The Folder column says "Uncategorised" for a downloaded recording in no category, and stays empty for one not downloaded. Typing "uncategorised" in Move to folder's new folder box means the Uncategorised folder, not a new one.
- **Settings** (Ctrl+Comma, and File menu, Settings) is a modal dialog from the template, in SPEC.md's order: the output folder as a single-line read-only box with Browse beside it, then a checkbox each, the days box, the announcements box, the Log file checkbox, and one account button whose text says "Sign in to Plaud" or "Sign out of Plaud". `[proven]` by the probe: every control named by its label, every Alt letter unique. OK saves `notefetch.conf` at once, and the worker takes the new settings for its next command (`Command::Settings`); a running sync carries on with the old ones. `[proven]` on the probe copy: the file changed and the Log said "Settings saved."
- **A refused days box** keeps the window open, says what to type, and puts focus back in the box, the skill's rule for refusals. `[reasoned]`
- **Browse** opens the Windows folder picker (`IFileOpenDialog` with `FOS_PICKFOLDERS`). Afterwards focus goes to the output folder box, so the new path is read in full. How JAWS handles the picker is `[untested]`.
- **Changing the output folder moves the recordings.** When the old folder holds downloaded recordings, OK asks "Move your N downloaded recordings to the new output folder?" No goes back to Settings with the old folder. Yes moves them one at a time, each folder whole, keeping their category: a rename on the same drive, a copy then delete on another, so a failed copy leaves the original. `[reasoned]`, with `move_out` unit-tested. The alternative, switching without moving, would make every recording look deleted, since the disk is the truth.
- **A move cannot be cancelled or closed part way.** Escape says it carries on, and Alt+F4 says to wait, so no recording is left behind by accident. Changes in the list are refused while it runs. A recording that cannot move stays in the old folder; the Log says to move it by hand and press F5. The output folder cannot change while a sync or download runs; OK says so and stays open. `[reasoned]`
- **Sign in from Settings** saves the settings and closes the window, since sign-in happens in the browser. **Sign out** works at once and leaves Settings open, with the button renamed. `[reasoned]`
- **The first-sync question** replaces the "arrives in a later version" message. It is asked once the sync that found it has finished, so OK can start the sync at once. Three radio buttons, then the number of days: "Everything: 79 recordings, about 112 hours", "Only the last 30 days: 12 recordings, about 9 hours" (chosen at first), and "Nothing old: start from now, and download only new recordings". Typing a number of days chooses that option and its text follows the number. `[proven]` by the probe and a posted number: "Only the last 200 days: 24 recordings, about 10 hours". Cancel downloads nothing, and the next sync asks again. Whether JAWS reads the explanation at the top by itself is `[untested]`.
- **The days box comes after the radio buttons,** because a box between two radio buttons would split their group, and the arrow keys would stop at it. `[reasoned]`
- **Sync on start** was already in place since milestone 3; it is now in Settings.
- **The completion tone setting** stays out of Settings, because milestone 6 replaces it with the sound palette (done: see The sound palette, first draft).
- **New developer flag** `--first-sync-question` shows the question on its own, so it can be probed without signing in afresh. **New test launcher** `test-data\Start NoteFetch first sync test.cmd` starts an empty copy in `test-data\fresh`, for a real first sync.

### Milestone 5 results (24 October 2026)

Laurent ran `test-milestone-5.txt`. Everything worked, including the folder picker ("All good, standard dialogues as expected"), moving to another output folder and back, signing in from Settings ("Perfectly") and a real first sync of the last 3 days. Recording folders and category folders now tell apart: "Yes, excellent." The question about a recording put straight into the output folder was left blank; it stays as built (moved into Uncategorised at the next refresh), which section 1 showed him and he called "Very good."

His changes, all made the same day:

- **The American spelling is reserved too.** "Also make sure that "uncategorized" can't go through." Creating or renaming a folder to it is refused, and typing it in Move to folder means Uncategorised (`library::means_uncategorised`). The folder on disk keeps the UK name. He asked for a steer on UK or US spelling; the log recommends UK throughout, with both spellings accepted wherever he types a name.
- **Settings labels without "the":** "Rather say "download audio", "download transcripts", "download summaries", etc. just sounds better." Now "Download audio", "Download transcripts", "Download summaries", "Download other parts: ...".
- **The folder picker starts one level up:** "Ideally it should be a level up the tree, i.e. listing the folder containing the output folder." It now opens in the folder holding the output folder, so the output folder is in the list beside its neighbours.
- **The move question defaults to No:** "Please make "no" the default option when enter is pressed and have focus land on "no" instead of "yes". More cautious, but could prevent some accidents." (`MB_DEFBUTTON2`.) The Delete and Delete folder questions still default to Yes; the log asks whether they should follow.
- **Sign-in hints name Settings too:** "Maybe say to sign in via the file menu or settings dialogue." Every "from the File menu" hint about signing in now says "from the File menu or from Settings".
- **The first-sync text says what is downloaded:** "instead of "recording history", say "audio, transcripts and summaries"."
- **The days choice says it can be changed:** "for a new screenreader user going down the radio buttons, it may not be clear that the 30 days is customisable. Maybe change the lable to "specified duration: {number} days"." Now "Specified duration, the last 30 days: 12 recordings, about 9 hours", Alt+S, and the box below is "Specified duration, number of days:", so the two read as a pair. `[proven]` by the probe: names and Alt letters correct.
- JAWS read the first-sync explanation by itself (step 19, "Yes"), so that is now `[proven]`.

### Milestone 5 round 2, and sign-off (24 October 2026)

Laurent ran `test-milestone-5-round-2.txt`: every change worked. "No, happy to sign off this milestone." On the first-sync question asking again until answered: "Also brilliant how, if the dialogue is exited, the options are presented again on the next sync until a choice is made."

- **UK spelling throughout, both accepted when typed.** He agreed with the steer: "Yes."
- **Delete folder now defaults to No; deleting recordings keeps Yes.** "Deleting folders should default to no. Deleting recordings to yes, as it could get frustrated when deleting multiple recordings and there is a good safeguard." (The safeguard is the Recycle Bin.)
- **Sign in is unavailable in the File menu while signed in,** and Sign out while signed out. "The sign in option is available in the file menu even when already signed in. This does no real harm, but can be confusing. Please have this option grayed out when already signed in, unless there is a good reason to leave it as is." The one good reason is an expired sign-in: the file is still there but Plaud refuses it. So the window remembers when the refused sign-in was saved (`auth::saved_at`), and offers Sign in again until a newer one is saved. Settings' account button follows the same rule. `[proven]` on a scratch copy by reading the menu's state: with no sign-in, Sign in available and Sign out not; with one, the reverse. The expired case is `[reasoned]`.
- **A settings file saved with a byte order mark** (as Notepad or PowerShell can) lost its first setting, found while probing. The mark is now ignored, with a test.

### The sound palette, first draft (24 October 2026)

Built in `src/sound.rs`, wired in, and waiting for Laurent's ears (`test-milestone-6-sounds.txt`).

- **TweakSound's method, unchanged where it was proven:** sine notes rendered into a WAV in memory and played with `PlaySoundW`, which needs only the `Win32_Media_Audio` feature of the `windows` crate already used, no new dependency. Its lead-in of a twelfth of a second (Laurent's own choice in TweakSound round 3, against Bluetooth clipping), its volume of 0.15 ("Yes, right volume") as Normal, and its fades.
- **NoteFetch's own notes:** only C sharp, D, F sharp and G, each at least a whole tone from StableSound's E, B and A and TweakSound's B and E. A test holds it.
  - **Done:** D then F sharp, quickly rising. A rename, move, delete, skip or folder change worked.
  - **Finished:** Done's two notes then the D an octave up, so it sounds like Done's bigger sibling. A sync, download, sign-in or output-folder move he started has finished.
  - **Refused:** G falling to C sharp, a tritone, the most unresolved interval. Something could not be done, or went wrong.
  - **Attention:** F sharp, a short gap, then the F sharp an octave up, like a call. Just before a question: sign in, delete, delete folder, closing while busy, moving recordings, and the first sync.
- **When they play,** in the interface only: Done or Finished when a command he started ends well; Refused at once for a refusal of something just pressed, or once at the end of a running command, however many things went wrong; nothing for a command he cancelled; nothing for the sync on start unless something went wrong, the plan's one exception to "only for what the user did". `[reasoned]`
- **Settings:** "Play sounds for what you do" (Alt+P, on at first) and "Sound volume" (Alt+V: Quiet 0.07, Normal 0.15, Loud 0.3). Choosing a volume plays Done at it, so it can be judged before OK. `[proven]` by the probe: names and Alt letters correct. The old `completion_tone` line in a settings file is accepted and ignored.
- **Developer flag** `--sounds` plays all four in turn, and `--sounds done` (or finished, refused, attention) one. `test-data\Play NoteFetch sounds.cmd` runs it.

### The sound palette, round 1 results (24 October 2026)

Laurent ran `test-milestone-6-sounds.txt`. The four sounds themselves passed as they are: each complete, no clipped start or click, each suits its meaning and is told apart from the others and from StableSound's and TweakSound's ("Yes" to both). The sign-in question's Attention was "an appropriate sound", and Finished after the sync that followed the sign-in "is nice". `[proven]`

- **Quiet is now the default volume.** "I chose the quiet volume. There, but subtle."
- **A rename makes no sound.** None played, which was a bug (below), but his verdict: "No sound was played, but that's atually ok for a rename." JAWS already says the new name. Its refusal still sounds.
- **Refused for a change was missing:** "No sound played. On this ocasion the refusal sound would have been good." The cause: changes (rename, move, delete, skip, folders) run alongside a sync and report through their own events, never through the path that played sounds. They now send the window a sound of their own (`UiEvent::Sound`): Done when everything worked (except a rename), Refused when anything was refused. `[reasoned]` until round 2.
- **No Attention before a question he has just asked for.** On the Delete question: "A bit too much, not necessary." Attention now plays only before the questions NoteFetch raises by itself, the sign-in and the first sync. Delete, Delete folder, closing while busy and moving to a new output folder are silent.
- **Found while fixing it:** a refusal pressed during a sync (say, starting a second sync) would have replaced that sync's Finished with Refused. Now a refusal of a key press sounds at once and leaves the running command's sound alone; only errors in the running command itself turn its ending into Refused.

### The sound palette, round 2, and sign-off (24 October 2026)

Laurent ran `test-milestone-6-sounds-round-2.txt`. All seven steps as expected `[proven]`: Refused for a refused folder name, Done for a new folder, a move and a folder delete, silence for a rename that worked, no Attention before Delete or Delete folder, and a second Ctrl+S during a sync refused at once while the sync still ended with Finished: "Yes, worked well as expected." Anything else: "No, excellent work." The sounds are signed off.

### The user guide, and the rest of milestone 6 (24 October 2026)

Built straight after the sounds sign-off, as Laurent asked: "Please plan to do as much work as possible before the next batch of tests are needed." Nothing here is yet heard by Laurent; `test-milestone-6.txt` checks it.

- **The guide** is `src/help.html`, compiled in with `include_str!`, following the principles recorded in "A user guide with tutorials" above and the layout of Photo Scribe's guide: welcome and byline, contents, About this guide, Why use NoteFetch?, What NoteFetch does, Words you will meet, five parts with eleven tutorials (each "What you will learn", "Worked example", numbered steps, "What just happened" as level 4 headings), the settings, announcements and sounds, what is saved, privacy, troubleshooting with a heading per symptom, the keys, the files, and credits. `[reasoned]`; his verdict is the point of the round.
- **No tables.** The quick reference is lists under level 3 headings, where Photo Scribe has a table with proper headers. Either meets the skill's rule; lists were chosen because a list item reads the task and its key together in one line. Asked in the test.
- **Written to `%LOCALAPPDATA%\NoteFetch\notefetch-help.html`** afresh on every F1, and opened as a `file:///` URL, the skill's `[proven]` recipe from TweakSound. The version is filled in from Cargo.toml.
- **F1 everywhere.** In the main window it is an accelerator. In Settings, Move to folder and the other modal windows, nothing handles WM_HELP, so Windows passes it to the owner, the main window, which opens the guide. `[reasoned]` from Microsoft's documentation of DefWindowProc; `[untested]` live. A one-second guard stops a single press opening two tabs, in case Windows sends both.
- **The Help menu** now has "NoteFetch guide F1", "Test a spoken announcement" and "About NoteFetch". About says the version in the Log and aloud, not in a message box, since it is routine information. (Replaced by a small window after Laurent's test; see below.)
- **Tests keep the guide honest:** every contents link has a target, every Alt letter it gives for the main window matches `notefetch.rc`, every shortcut it lists is there, no em dashes, no tables, no American spellings outside the stylesheet, and the version filled in.
- **Error messages reviewed.** Every message in the code was read for what happened and what to do next. Changed: a reply from Plaud that cannot be read now has its own error, `Error::Unreadable` ("NoteFetch could not understand Plaud's reply. Try again later. If it keeps happening, Plaud may have changed how it answers, and a newer version of NoteFetch may be needed."), where before one of them named an internal tool such as `list_files`. Next steps added to: a recording no longer in the list (press F5; it was wrongly worded as a database failure), a folder that no longer exists, a folder name already taken, a folder left in place because it holds other files, a recording already in the new output folder, saving the sign-in, and metadata.json. Announcement levels were already built in milestones 3 to 5 and are heard in this round.
- **The release groundwork, from milestone 7:** `.github/workflows/build.yml` (test, clippy and release build on a Windows runner for every push; on a tag such as v0.1.0 it checks the tag matches Cargo.toml and attaches the exe and the zip to a draft release, published by hand), a test keeping the version resource in step with Cargo.toml, and a first README in StableSound's style, without its em dashes.
- **History scanned with gitleaks:** 15 commits, no leaks. Every file ever committed was listed: no test data, audio, databases or sign-in files. Run again just before the repository goes public.
- **SignPath Foundation** (signpath.org/terms, read 24 October 2026, marked draft there): free signing for projects under an OSI licence with no proprietary parts, actively maintained, already released in the form to sign, built automatically from the repository (the new workflow), with MFA for everyone in the team, a "Code signing policy" section on the home page naming SignPath and the team roles, and a privacy statement. NoteFetch looks eligible once it is public and has a first release. It sends data only to Plaud, at the user's request, which fits the privacy rule. `[reasoned]`; their acceptance is at their discretion.
- **Release exe:** 2.7 MB.
- **The workflow's first run failed, usefully.** Ten tests expected UK folder names such as "Lecture, 24 Sep 2026, 1430", and the runner is on UTC, so it made "1330". `[proven]` by the run. The code was right; the tests depended on the machine's zone. Under test, `clock::to_local` now always uses Windows' rules for "GMT Standard Time", so the tests pass in any zone. The second run passed on the runner. `[proven]`

### Milestone 6 results, and NVDA (24 October 2026)

Laurent ran `test-milestone-6.txt` with JAWS, then sections 5 with NVDA 2026.2.

- **F1 and the guide** `[proven]`: F1 opened the guide once, from the main window and from Settings, with focus back in Settings after. The headings read well. The guide is signed off: "Yes, I may edit and refine it for a later version, but its more than good enough for now. Thanks." and "Not that I could identify. Happy to sign off." The quick reference as lists: "Fine as is." The level 4 headings in tutorials: "Its good."
- **Announcement levels, the test announcement and the error messages** `[proven]` with JAWS: all as expected, including the network-off error.
- **About NoteFetch was not heard.** "No message was spoken by Jaws, it just went back to the list of recordings. The message did appear in the log. Users are unlikely to know to go and look there. It would be better, and more typical, for the about message to appear in a read only edit field ... and an ok button." He also asked for the guide's credit in it: "please indulge my vanity and credit us in the about message, same as in the guide." About is now a small window (`IDD_ABOUT`): label "Details", a single-line read-only box, which the probe and NVDA both read in full on focus, and OK. `[proven]` with NVDA here; `[untested]` with JAWS.
- **"Test a spoken announcement"** stays for now, to check the NVDA fix: "Ultimately, I'd like to get rid of the "test spoken announcements" item under help, especially if my testing with NVDA is all done and everything speaks as it should." Remove it once NVDA is signed off.
- **NVDA read the controls and rows well** `[proven]`: "Reads nicely", "Very well, sounds just like Jaws." The context menu, the rename box and F1 worked.
- **NVDA spoke no announcement at all**, not "Syncing with Plaud.", "Sync finished." or the test: "Silence, then the finish sound, then the currently selected recording line read out three times." Found, without needing Laurent, by reading NVDA's source and running it here with its UIA debug log on:
  - NVDA's log showed it never received a notification event from NoteFetch, while it did receive Calculator's. `[proven]` A native UI Automation listener built the way NVDA's is (root element, subtree, with and without a cache request and an event handler group) heard nothing from the bare host provider (`UiaHostProviderFromHwnd`), and heard every one once the event came from a small provider of NoteFetch's own (`IRawElementProviderSimple`, everything left to the host window). `[proven]` NVDA still received nothing even then. `[proven]` Why NVDA alone misses them is not known.
  - So with NVDA running (its hidden window, class `wxWindowClassNR`, title "NVDA"), NoteFetch now uses NVDA's other route: the text goes into a hidden static control in the active NoteFetch window, and `EVENT_OBJECT_LIVEREGIONCHANGED` asks NVDA to read it. NVDA reads the control when it gets to the event, so everything said within 150 ms is joined into one. The control is created on demand as the window's last child, so it labels nothing and is never reached with Tab. It has to be in the active window: NVDA ignores events from a window behind the one in front, so with Settings open the main window's would be lost. `[proven]` with NVDA: the sync, refresh, test and Ctrl+A announcements, a test announcement from the Help menu (spoken after NVDA re-reads the list as the menu closes), and one made while Settings was open.
  - Without NVDA, the UI Automation notification is raised as before, but on NoteFetch's own provider. JAWS heard the host provider's `[proven]`; whether it still hears the new provider's is `[untested]`, and the first thing to check.
  - The provider needs `windows-core` named in Cargo.toml so `#[implement]` can find it. It is not a new dependency: it was already built as part of `windows`, and Cargo.lock gained one line.
- **The row read three times** came from the list being emptied and refilled on every snapshot: each refill moved the focus to the first row and back, and NVDA read the focused row for each. `[proven]` by counting WinEvents: a refresh fired two full rebuilds, each with a focus event. `apply_filter` now changes only the cells, selection and focus that differ, so a sync that changes nothing fires no events at all. `[proven]` with NVDA and the event counter.
- **"The selected recording is read twice when tabbing to the listview."** Tab itself read it once here. `[proven]` with NVDA. The double reading was at start-up: the list took focus while still empty and gained its first row a moment later, and NVDA read that row twice. The window now waits up to half a second for its first list before it appears, then focuses the list, and NVDA reads it once. `[proven]` Laurent may also have heard rows repeated by the refills above while tabbing during the start-up sync.
- **Tab in the rename box** "moves to an edit field allowing the next item to be renamed. This might work visually, but could be confusing for a screenreader user." Tab and Shift+Tab now finish the edit as Enter does and leave focus on the same recording. A subclass of the edit box never saw the key, because the list handles it first; the message loop changes Tab to Enter while a name is being edited. `[proven]` with NVDA.
- Laurent: "Please see if you can fix the NVDA issues without breaking the excellent Jaws functionality and we'll note the learning in the skill for creating accessible apps used here." The findings are in the skill's `references/announcements.md` and `references/dialogs.md`.

### Milestone 6 round 2, and sign-off (24 October 2026)

Laurent ran `test-milestone-6-round-2.txt`, JAWS first, then NVDA. Every step as expected. `[proven]`

- **JAWS hears notifications raised on NoteFetch's own provider**, at start-up, after Ctrl+S, from the Help menu and from inside Settings, and the start-up row was read once. The `[untested]` above is now `[proven]`.
- **NVDA** spoke every announcement through the live region: start-up, Ctrl+S, the Help menu test ("Yes, perfect."), Ctrl+A, the Settings refusal, and "Errors and completion only". The row was read once on start-up, on Tab and after a sync. About and Tab in the rename box behaved as with JAWS ("Yes, perfect.").
- **About**: the whole text read by JAWS, and the wording right.
- **Narrator** speaks the announcements too: "Incidentally, it works with Narrator too. I don't want to do extensive testing with Narrrator now, but it should already be workable." `[proven]` for announcements only.
- **"Test a spoken announcement" is removed** from the Help menu, the guide and SPEC.md: "Yes. There are other ways of testing spoken announcements during development." The guide's troubleshooting now suggests Ctrl+S and listening for "Syncing with Plaud." instead.
- Anything odd: "No, very good." Anything else: "No." Milestone 6 is signed off.

### Release decisions (24 October 2026)

Laurent's answers in `log.md`:

- **No letter to Plaud.** "I'm not going to proactively contact Plaud. I'll react if approached." The draft stays in `log.md` should it be needed. The README and guide already say NoteFetch is not made, endorsed or supported by Plaud.
- **Trademark registers not searched.** "I'll approach any concerns reactively." Web searches on 24 October 2026 found no product called NoteFetch.
- **His Gmail address stays as the commit author:** "No, ok to use laurentd@gmail.com, same as for PhotoScribe." The history is not rewritten.
- **Public now:** "Make it public now. Would it be appropriate to change the version number to v1?" Yes: the spec is built, every window has been tested over more than ten rounds with JAWS and two with NVDA, and a 0.x number would say it was not ready. The version is 1.0.0, and the tag v1.0.0 makes the first draft release, published by Laurent.
- **Release published** on 24 October 2026 at his request: "Publish it please."
- **Code signing:** "Yes, and please write up a crash course for me on code signing with SignPath foundation." The crash course is in `log.md`. The README has a "Code signing policy" section saying releases are not yet signed. Findings, from docs.signpath.io and Microsoft Learn on 24 October 2026:
  - Signing uses `signpath/github-action-submit-signing-request@v3`, given the artifact ID from `actions/upload-artifact`, an API token secret, the organisation ID, project slug and signing policy slug. For open-source projects every job before the request must run on GitHub-hosted runners, which `build.yml` already does. Each request is approved by hand, so the wait timeout (default 600 seconds) needs raising.
  - The publisher shown is "SignPath Foundation", not the project.
  - EV certificates no longer bypass SmartScreen, and reputation builds per file as it is downloaded, so a signed release can still warn at first. `[reasoned]` from Microsoft's page; not observed with NoteFetch.

### Name: NoteFetch

- Rejected names:
  - Ploader, because a dormant Piwigo photo uploader of the same name exists, and speech synthesisers may say "PLOH-der".
  - Unspool, because a voice-journal app with AI transcription and an open-source Mac downloader for Telegram both use it, among others.
  - Unreel, because it is crowded, with video, presentation and advertising products.
  - FetchNote, because of Fetchnotes, a 2012 to 2013 note-taking startup, and because it is a common function name in code.
- NoteFetch returned no existing product in web searches on 26 September 2026. It echoes Plaud's "Note" devices without using the Plaud name.
- Still to do before the public release: check the UK IPO and EUIPO trademark registers, and confirm how JAWS and NVDA pronounce it.

### MIT licence

- The same as StableSound. Permissive, short, and widely understood.

## Sources

Checked on 26 September 2026. Re-check before release, as all of this is new and may change.

- Plaud MCP developer docs: https://docs.plaud.ai/plaud-mcp-cli/mcp
- Plaud CLI developer docs: https://docs.plaud.ai/plaud-mcp-cli/cli
- Plaud MCP and CLI changelog (version 1.0.0 generally available 12 May 2026): https://docs.plaud.ai/plaud-mcp-cli/changelog
- Plaud support article giving the hosted server URL and confirming it is read-only: https://support.plaud.ai/hc/en-us/articles/57751078986265-Plaud-MCP
- Official Rust MCP SDK (rmcp), no longer used: https://github.com/modelcontextprotocol/rust-sdk
- Plaud plan comparison, listing "Plaud MCP server (Beta)" on every plan: https://www.plaud.ai/pages/plaud-ai-plan-pricing

GitHub projects read on 26 September 2026:

- plaud-cli (Rust, MIT), a thin client of the hosted server; source of the sign-in and MCP client design: https://github.com/leegonzales/plaud-cli
- plaud-mcp-connector (Python, MIT), notes on UTC times, the 500-recording name search limit, and missing end times: https://github.com/PsychQuant/plaud-mcp-connector
- plaud-obsidian-sync (JavaScript, MIT), notes on UTC times and the plan requirement: https://github.com/simonsez9510/plaud-obsidian-sync
- plaudit (Rust, no licence file, so not reusable), which uses Plaud's REST developer API with the official CLI's registration. Not followed: https://github.com/lmmx/plaudit
- Plaud_BulkDownloader (C#, MIT), password login to the unofficial web API, one folder per Plaud tag. Not followed: https://github.com/JamesStuder/Plaud_BulkDownloader
- No project found gives a Windows interface or mentions screen readers.
- Date conflict to resolve: Plaud's changelog gives 12 May 2026 for general availability of the npm MCP and CLI; plaud-mcp-connector says Plaud "shipped an official MCP server on 2026-08-06", possibly the hosted server.

## Not yet verified

- Whether plain paging of `list_files` stops at 500 recordings (the test account has 71).
- How long a refresh token lasts if unused. The server does not say. Less than three weeks: one unused from 27 September was refused on 18 October.
- Plaud's terms of service on third-party clients, and use of the Plaud name.
- Whether SignPath's free code signing for open-source projects is available for this project.
