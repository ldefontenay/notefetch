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
- How long a refresh token lasts if unused. The server does not say.
- Plaud's terms of service on third-party clients, and use of the Plaud name.
- Whether SignPath's free code signing for open-source projects is available for this project.
