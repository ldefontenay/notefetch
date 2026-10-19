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
- **They wait for a sync or download to finish.** The worker runs one command at a time, and renaming a folder a download is writing into would break the download. Asked meanwhile, the Log says "... cannot start while something else is running". `[reasoned]`; it may be annoying straight after start-up, while the sync on start runs. Ask Laurent.
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
