# NoteFetch specification

The name NoteFetch was chosen on 26 September 2026; see PLAN.md. The public subtitle is "Download your Plaud recordings, transcripts and summaries". It uses Plaud descriptively, subject to Plaud's trademark terms.

## Purpose

NoteFetch downloads recordings, transcripts and AI summaries from a Plaud account to a local folder, so the audio can be transcribed elsewhere and the text kept offline. It is built for blind Windows users first, and tested with JAWS by a JAWS user.

The first use is lecture recordings made on a Plaud Note Pro. The app will be released free, under the MIT licence, once it is ready.

## Non-goals

- No transcription of its own. It fetches what Plaud already holds.
- No editing, renaming or deleting anything in the Plaud account. The Plaud MCP server is read-only in any case.
- No background or tray operation. Sync runs when the app starts and when asked.
- Not designed or assessed for clinical or other special category data.

## Platform and distribution

- Windows 10 and 11, 64-bit.
- A single executable, with no installer, no runtime to install, and no administrator rights needed.
- Released on GitHub as both `notefetch.exe` and `notefetch.zip`, as StableSound is, because Edge makes keeping an unsigned exe awkward with a screen reader.
- Unsigned at first. The README explains SmartScreen and the Defender cloud scan in the same way StableSound's does.

## How it talks to Plaud

Plaud runs a hosted MCP server. NoteFetch is a native MCP client of that server. It does not use the npm packages, so users do not need Node.js.

- Server: `https://mcp.plaud.ai/mcp`, Streamable HTTP transport.
- Authorisation: OAuth 2.1 as defined by the MCP authorisation spec. This was checked against the live discovery documents on 26 September 2026:
  - protected resource metadata names `https://mcp.plaud.ai/` as the authorisation server;
  - the authorisation server offers a registration endpoint (dynamic client registration), the authorisation code grant with PKCE S256, refresh tokens, and `none` as a token endpoint auth method (a public client with no secret).
- Client: a small MCP client of NoteFetch's own over reqwest, adapted from plaud-cli (MIT), not rmcp. See PLAN.md.
- The server is stateless (no session ID) and wraps every tool reply in a safety envelope: a sentence, then the JSON between `<untrusted-user-data-XXXX ...>` and its closing tag, where XXXX is random. The client takes the JSON from inside the tag that starts a line.
- The free Starter plan has full access (proven in milestone 1).
- Tools used (proven in milestone 1, see `test-milestone-1.txt`):
  - `list_files`: newest first, `page_size` from 10 to 100, no total count. NoteFetch asks for 100 per page and stops at the first page with fewer than 100.
  - `get_file`: everything about one recording in one call: metadata, `presigned_url` (the audio link, valid 24 hours), every note (one per tab in the Plaud app, as Markdown) and every transcript block (as JSON).
  - `get_transcript` and `get_note`: used only if `get_file` ever returns a block without its content. `get_transcript` pages 50 segments at a time with a cursor.
- Times in replies carry no zone and are UTC (proven). They are converted to local time everywhere they are shown or used in a name.
- Durations and transcript times are in milliseconds.
- Signed links (`presigned_url`, and the `data_link` on every note and transcript block) are secrets and are never logged.

## Sign-in

- The first run, or the Sign in command, starts the OAuth flow:
  1. It listens on a free loopback port (`http://127.0.0.1:<port>/callback`).
  2. It registers a new client for that exact address with dynamic client registration. This happens at every sign-in, so the port never has to match an earlier registration.
  3. It opens the system browser at Plaud's authorisation page, with PKCE and `resource=https://mcp.plaud.ai/mcp`.
  4. It waits for the redirect, exchanges the code and stores the tokens.
- The page returned to the browser after sign-in is plain, accessible HTML with `lang="en-GB"`, an `h1`, and one sentence telling the user to return to NoteFetch.
- While waiting, the app says so in the log and offers Cancel. Sign-in times out after five minutes.
- Access tokens last 24 hours and refresh automatically. Each refresh returns a new refresh token, which is saved before anything else happens, and only one refresh runs at a time. If refresh fails, the app says in plain words that the user needs to sign in again, and offers to do it.
- Sign out deletes the stored tokens and client registration.

## Main window

A modeless dialog built from a resource template, like StableSound's settings dialog. Controls, in tab order:

1. "Show" combo box, with the choices "All recordings", "Downloaded", "Not yet downloaded", "Incomplete" and "Deleted". The Deleted view is part of the same list, not a separate window, so the keys and context menu work the same way everywhere.
2. "Filter by name" edit box. Filtering happens locally as the user types, after a short pause.
3. "Recordings" list view in report view, with the columns Name, Date, Duration and Status.
4. Buttons: "Sync new", "Download selected", "Open folder".
5. "Log", a read-only multi-line edit box holding the running record of what happened, reviewable with the arrow keys.

Presentation rules for the list:

- Newest first.
- Date written as "Thu 24 Sep 2026, 14:30" in local time.
- Duration in words, for example "1 hour 12 minutes".
- Name is the local name, which the user can rename. The original Plaud name is kept in `metadata.json`.
- Status in words: "Downloaded", "Not downloaded", "No transcript", "No summary", "Audio only" (neither), "Failed", "Skipped", "Deleted". No icons carry meaning.
- After a refresh, selection and focus stay on the same recording, matched by Plaud ID.

## Keyboard

Every control has an Alt mnemonic on its label.

The keys listed under "In the recordings list" work only when the list has focus. They are handled by the list itself, not the window's accelerator table, so Delete, Ctrl+A and the others keep their usual meaning in the filter box.

### In the recordings list

- Enter: the default action for the focused recording. This plays a downloaded recording, downloads a recording that is not yet downloaded, and downloads a deleted recording again. The context menu always shows which action Enter will take.
- Ctrl+D: download all selected recordings.
- Applications key or Shift+F10: open the context menu for the focused recording.
- F2: rename the focused recording.
- Delete: move the selected recordings to the Recycle Bin.
- Ctrl+T: open the transcript of the focused recording.
- Ctrl+M: open the summary of the focused recording.
- Ctrl+Shift+C: copy the full path of the focused recording's audio file.
- Ctrl+Shift+O: open the focused recording's folder in File Explorer.
- Ctrl+A: select all visible recordings.

### Anywhere in the window

- Ctrl+S: sync new recordings.
- F5: refresh the list from Plaud without downloading.
- Ctrl+F: move to the filter box.
- Ctrl+O: open the output folder in File Explorer.
- Ctrl+comma: settings.
- F1: help.
- Escape: cancel the running operation. It does not close the app.
- Alt+F4: exit. If a download is running, the app asks first.

The app also has a menu bar (File, Recording, Actions, Help) that lists every command with its shortcut, so commands can be discovered without the help file. The Recording menu holds the same commands as the context menu.

## Sync

"Sync new" runs when the app starts (a setting, on by default) and on Ctrl+S.

- It pages through `list_files` and compares against the local database, then calls `get_file` for each recording it needs.
- It downloads anything not yet complete: new recordings, earlier failures, and recordings still waiting for a transcript or summary.
- It never downloads recordings marked Deleted or Skipped. Only the user can bring those back, with Download again.
- Recordings with no transcript or summary yet are checked again on later syncs for a set number of days (a setting, 14 by default), because Plaud may generate them later. After that the missing parts count as "not available" and are no longer retried.
- **First sync.** On the first sync the app asks whether to download the whole history, only recordings from the last N days, or nothing (start from now). This prevents an unexpected download of hundreds of hours of audio.

## Playing, renaming and deleting

These let the user work out which recording is which, give it a meaningful name, and clear out what is not wanted.

### Playing

- Playing opens the audio file in the user's default player through the Windows shell. The app has no player of its own in version 1 (see PLAN.md).
- If Windows has no default player for the format, Windows asks which program to use. If opening fails, the Log says why.
- Open transcript and Open summary use the default program for `.txt` and `.md` files. For lectures, the summary is often the quickest way to tell recordings apart.

### Renaming

- F2 edits the name in place, as in File Explorer. Only the name part is edited; the date and time suffix is kept automatically.
- Renaming changes the folder and every file inside it to the new name, and records the new name in the database and `metadata.json`.
- Renaming is local only. Plaud's MCP server is read-only, so the name in the Plaud app does not change.
- Sync never overwrites a local name.
- If a file is open in another program, for example the media player, Windows will refuse the rename. The app then says so in plain words and asks the user to close the player and try again. Nothing is left half-renamed: either every file is renamed, or everything is put back as it was.
- In-place editing is tested with JAWS in milestone 4. If it proves unreliable, a small rename dialog replaces it.

### Deleting

- Delete moves the recording's whole folder to the Recycle Bin, so it can also be restored from there.
- A confirmation question is asked first, naming the recording and saying it will not be downloaded again unless the user chooses Download again. The question can be turned off in settings. It is on by default.
- The database keeps the recording with the status "Deleted", so sync never downloads it again by itself.
- Deleting in NoteFetch never deletes anything in the Plaud account.

### The Deleted view

- Choosing "Deleted" in the Show combo box lists deleted recordings, newest first.
- Enter, or Download again in the context menu, downloads the recording again and returns it to the normal list.
- If a folder is restored from the Recycle Bin, or its files reappear, the next refresh notices and marks the recording as downloaded again.
- If a folder is deleted outside the app, for example in File Explorer, the next refresh treats it as deleted, the same as deleting in the app. This respects what the user did, rather than quietly downloading it again.

### Skipping

- A recording that has never been downloaded can be marked "Don't download", for example a test recording. Sync then leaves it alone. Skipped recordings appear in the Deleted view, and can be downloaded from there like deleted ones.

## Organising recordings

Laurent asked for features that help him keep recordings organised. The design waits for milestone 1 to show whether the server returns Plaud's folders or tags. If it does, the list and the output folder follow them. If not, NoteFetch offers groups of its own, for example one per course, each with a subfolder. Either way it is specified here before it is built.

## Context menu

Opened with the Applications key or Shift+F10 on the focused recording. The menu appears at the focused row, not at the mouse pointer. Items depend on the recording's status, and the item Enter would carry out is the menu's default item, shown in bold, which JAWS reads as the default.

- Downloaded:
  - Play (default)
  - Open transcript
  - Open summary
  - Open folder
  - Copy audio file path
  - Rename
  - Download missing parts, shown only if a transcript or summary is missing
  - Delete
- Not yet downloaded:
  - Download (default)
  - Don't download
- Deleted or skipped:
  - Download again (default)

With several recordings selected, only the items that make sense for a group are available: Download, Delete and Don't download. The others are shown but disabled, so the menu does not change shape unexpectedly.

## What is saved

One folder per recording inside the output folder. The folder name is the recording name first, then a comma, then the local start date and time, for example `Cognitive neuroscience week 3, 2026-09-24 1430`.

Plaud's default name for a recording is its date and time, such as "2026-09-09 17:27:14", and a renamed recording may keep them at the end, such as "Staff training 2026-09-24 09:00:26". A trailing date and time in that form is removed from the Plaud name before the base name is built, so it is not repeated. If nothing is left, the name is "Untitled", for example `Untitled, 2026-09-09 1727`.

- The name comes first so that screen reader users hear the meaningful part straight away, and first-letter navigation works in File Explorer and file dialogs.
- The date and time suffix keeps names unique when the same name is used twice, for example a weekly lecture.
- The folder and every file are given a modified date equal to the recording's start time, so sorting by Date modified in File Explorer still gives date order.

- The name is cleaned of characters Windows forbids and of reserved device names (CON, NUL and so on), and has trailing dots and spaces removed.
- It is shortened so full paths stay under 240 characters.
- A clash that remains, such as two recordings with the same name started in the same minute, gets " (2)" added at the end.
- If the recording is later renamed in Plaud, nothing local changes; `metadata.json` records the new Plaud name.

Every file inside the folder carries the same base name as the folder, so a file keeps its meaning when it is copied elsewhere, for example into a transcription tool. With the base name `Cognitive neuroscience week 3, 2026-09-24 1430`, the folder holds:

- `Cognitive neuroscience week 3, 2026-09-24 1430.opus`: the audio. Plaud serves Ogg Opus (`audio/ogg`), mono, about 34 kilobits per second, so two hours is about 30 MB. The extension, `.opus` or `.ogg`, is still to be settled with Laurent.
- `Cognitive neuroscience week 3, 2026-09-24 1430 transcript.txt`: one line per segment, as start time, speaker, then text, from the `transaction` block.
- `Cognitive neuroscience week 3, 2026-09-24 1430 summary.md`: the note on the Summary tab. Each other tab in the Plaud app is saved as its own file named after the tab, for example `... Meeting minutes.md`.
- `metadata.json`: Plaud ID, original Plaud name, current local name, start time, duration, and when each part was downloaded. This keeps a fixed name, because the app uses it to recognise the folder.

Settings choose which of audio, transcript and summary to download. All three are on by default.

## Downloads and network behaviour

- Audio is streamed to `<name>.part` and renamed only when complete and matching the Content-Length.
- If the temporary link has expired or returns 403, the app calls `get_file` again for a fresh link, once.
- Requests that fail with 429, 5xx or a network error are retried three times with increasing delay.
- At most two audio downloads run at the same time.
- Cancel stops cleanly and leaves no `.part` files behind.

## Announcements

- Start, finish and errors are announced through UI Automation notification events, and the same text always goes to the Log.
- Progress is announced at most once every few seconds, for example "Downloaded 3 of 12". Every item is logged.
- Settings allow announcements to be "All", "Errors and completion only" or "Off".
- An optional short tone plays on completion (off by default).
- No message boxes for routine information. Message boxes are used only for questions that need an answer, such as the first-sync choice or confirming exit during a download.

## Settings

A dialog built from a resource template, with:

- output folder, set with the Windows folder picker (`IFileOpenDialog` with `FOS_PICKFOLDERS`);
- sync on start (on or off);
- confirm before deleting (on or off);
- what to download (audio, transcript, summary);
- how many days to keep checking for missing transcripts and summaries;
- announcement level and completion tone;
- a Sign out button.

## Files the app writes

- `notefetch.conf`: plain-text settings, safe to edit by hand. Stored beside the exe, or in `%APPDATA%\NoteFetch` if that folder cannot be written to.
- `notefetch.db`: SQLite state, in `%LOCALAPPDATA%\NoteFetch`.
- `auth.bin`: the OAuth client registration and tokens, encrypted with Windows DPAPI for the current user, in `%APPDATA%\NoteFetch`.
- `notefetch.log`: only if logging to file is turned on (off by default).
- `notefetch-help.html`: written out when help is opened, from a copy compiled into the exe.

## Accessibility requirements

These are hard requirements, not preferences.

- Real Win32 controls only, from `.rc` templates. No drawn-UI toolkit.
- Every control has a visible static label placed just before it in tab order, and an Alt mnemonic.
- Tab order is logical and matches the order listed above.
- Focus is never lost. After any operation, focus returns to where the user was, usually the list.
- No meaning is carried by colour, icon or position alone.
- Anything announced is also written to the Log, so nothing is heard once and then gone.
- Errors say what happened and what to do next, in plain words.
- Testing:
  - JAWS on Windows 11 is primary.
  - NVDA is secondary, before the public release.
  - Narrator gets a quick smoke test.
  - Every release is tested by keyboard alone.

## Privacy and security

- No telemetry. Network traffic goes only to Plaud's MCP and authorisation endpoints and the host serving the temporary audio links.
- Tokens are encrypted with DPAPI and never written to logs.
- Temporary audio links are never logged, because they carry signatures.
- Logs do not contain transcript or summary text.
- The repository must never contain tokens, real recordings, transcripts or summaries, even temporarily, because the full history becomes public on release.

## Open questions

Milestone 1 answered the earlier questions; see `test-milestone-1.txt`. Still open:

- The audio file extension, `.opus` or `.ogg`, and whether Laurent's player and transcription tool accept Opus.
- The organisation feature (Plaud exposes no folders or tags).
- Whether plain paging of `list_files` goes past 500 recordings. The test account has 71.
