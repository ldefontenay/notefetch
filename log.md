# Project log

Newest entry first. Each entry starts with a level 2 heading holding the date and a short title, so you can jump between entries with your screen reader's heading keys.

## 2026-09-27: Milestone 3 built: NoteFetch has a window

### The short version

- **Your milestone 2 answers are applied.** Names now read "Test lecture, 21 Sep 2026, 1601", and every file starts with what it is: "Transcript, Test lecture, 21 Sep 2026, 1601.txt", "Summary, ...", "Audio, ...". There is a setting to save summaries as .txt. And good news on Heidi: no conversion needed.
- **NoteFetch now has its main window:** the Show and Folder boxes, the filter, the Recordings list, the three buttons, the Log with the newest line at the top, the menu bar, and the keyboard shortcuts.
- **Please test it:** test-milestone-3.txt, 20 steps, perhaps 30 minutes. You start NoteFetch with "Start NoteFetch test.cmd" in the test-data folder, so it uses the test copy of your data and needs no sign-in.
- **The most important question is in step 1:** whether JAWS speaks NoteFetch's announcements at all. Everything about progress and completion depends on it.
- All automatic checks pass: 65 tests, and clippy is clean.

### Your milestone 2 answers

- **Dates:** "24 Sep 2026, 0900". The colon in 09:00 had to go, because Windows does not allow colons in file names.
- **File names:** your convention, part first. I applied it to the audio too, "Audio, name, date.ogg", so every file follows one pattern and A finds the audio. Say if you would rather the audio had no label.
- **Summaries as .txt:** a setting, off by default, as you asked. It will be in the Settings window in milestone 5; for now it is a line in the settings file.
- **Heidi:** recorded in the plan with your words. No MP3 copy needed.

### What I checked on the window myself

I ran the accessibility probe from the skill against the live window before writing your test:

- Every control is named by its label: "Show:", "Folder:", "Filter by name:", "Recordings:", the three buttons, and "Log:".
- Every Alt key is present and different. There is also an automatic test that fails if two ever clash.
- The list rows carry their other columns, so JAWS has the date, duration and status to read with each name.
- The probe warned that ten rows are all called "Untitled". Heard one name at a time, they cannot be told apart; the date should make the difference, and step 4 asks you whether it does.

### Two things I fixed after watching it run

- **The Log said "1 recording fetched"** when NoteFetch had only checked a recording again for a transcript Plaud has not made. It now says "Checked ... again: Plaud has no transcript or summary for it yet."
- **The last line of a sync appeared twice** in the Log. Now once.

### One thing I could not finish checking

When NoteFetch opens, focus should be on the newest recording. Once it landed on the second one. I changed how NoteFetch remembers the focused recording, but could not confirm the fix: while I was checking, you were using the computer, and my test window would have taken the focus from you. So I stopped opening windows on your desktop. Step 20 checks it.

### Not in this version

Signing in from the window, the first-sync question, Settings, Help, Enter on a recording, the context menu, renaming and deleting. The test file says so at the top.

## 2026-09-27: Milestone 2 built: the engine works on your real recordings

### The short version

- **The engine is built and works.** It lists all 71 of your recordings, downloads them with every part Plaud has, and handles renaming, folders, moving and deleting. There is no window yet; that is milestone 3.
- **I tried it on two of your real recordings,** downloaded into the NoteFetch folder under test-data, output, away from your Documents. It took 15 seconds for both.
- **Please do a short test in File Explorer:** test-milestone-2.txt, in the NoteFetch folder. It checks how JAWS reads the folder and file names, the transcript layout, and whether the audio plays. Eleven steps, about ten minutes.
- **One question below,** about Heidi Health.
- All automatic checks pass: 59 tests, plus 2 that touch the real machine (encrypting your sign-in, and the Recycle Bin).

### Your answers, applied

- **.ogg:** the audio is saved as .ogg.
- **Folders mirror File Explorer.** A folder in NoteFetch is simply a folder inside the output folder. Move a recording in File Explorer, and NoteFetch sees it in its new place next time. I tested exactly that.
- **Everything saved, nothing empty.** Your 24 September recording got its audio, transcript, summary and outline. Plaud had no cleaned transcript or highlights for it, so there are no files for them. The 21 September recording has audio only, because Plaud has no transcript or summary for it.

### What I checked on your recordings

- **Names:** the 21 September recording had no name in Plaud, so it became "Untitled, 2026-09-21 1601". I then renamed it to "Test lecture": the folder and every file in it changed together, and the name in Plaud stayed as it was.
- **Times:** the folders say 0900 and 1601, the local start times in UK summer time, and File Explorer's Date modified shows the same times rather than today.
- **Folders:** created a folder, moved the recording into it, renamed the folder, moved the recording out by hand as File Explorer would, then deleted the folder. NoteFetch followed every step, and deleting the folder moved nothing to the Recycle Bin.
- **Recycle Bin:** a small test folder called "NoteFetch recycle test" went to your Recycle Bin during testing. You can empty it or leave it.
- **Renaming when a file is open:** tested with the audio held open, as a media player would hold it. Windows refused, and NoteFetch put back everything it had already renamed, so nothing was left half-renamed.

### Things worth knowing

- **Size:** the program is 2.5 MB. StableSound is 1 MB. The difference is the database and secure connection code built into it, so nothing extra has to be installed. I think that is fine for a single-file app.
- **Default save location:** a NoteFetch folder in your Documents, changeable in Settings once Settings exists.
- **Not yet tried:** cancelling a download half way. The window's Escape key will test it in milestone 3.

### Question

#### Question 1: does Heidi Health accept the .ogg file?

Heidi's help pages and developer pages do not say which audio formats it accepts. The only sure way to know is to try. When convenient, please upload a short .ogg recording to Heidi as you normally would. The 21 September one in test-data, output, "Test lecture", is half an hour long.

If Heidi refuses it, NoteFetch can also save an MP3 copy using FFmpeg, which is already installed on your computer, without making NoteFetch itself bigger. I would add that as a setting, off by default.

ANSWER:
I've tested the .ogg file with Heidi and it works perfectly.
## 2026-09-26: Milestone 1 finished: Plaud works on your free plan

### The short version

- **Everything works on your free plan.** Sign-in, listing all 71 of your recordings, fetching a transcript and summary, downloading audio, and refreshing the sign-in.
- **The audio is not MP3.** Plaud sends Ogg Opus, a compact format: your two-hour training session is 30 MB. I need your help deciding the file extension, see question 1.
- **Plaud does not share its folders** with outside apps, so NoteFetch has to keep its own. I have a proposal below and would like your view before I build anything.
- **Two surprises the other projects had not seen,** both handled: Plaud now wraps its replies in a safety envelope, and every refresh of the sign-in replaces the refresh token.
- The full findings are in test-milestone-1.txt, in plain text. PLAN.md, SPEC.md, TASKS.md and CLAUDE.md are updated. Milestone 2, the engine, waits for your answers below, because two of them change what gets saved.

### What the probe found

- **Sign-in:** worked first time. The sign-in lasts 24 hours and then renews itself in the background. You will not normally see it again.
- **The safety envelope:** each reply from Plaud now starts with a sentence saying the contents are recording data, not instructions, and puts the data inside a tag with a random name. This protects AI assistants from instructions hidden in a recording's name or transcript. plaud-cli was written before this and would find no recordings at all; my first run did the same until I handled it.
- **Refresh tokens change each time.** NoteFetch must save the new one straight away, and must never renew twice at once, or it could lose your sign-in. That is now in the plan.
- **One call per recording gets everything:** the audio link, the whole transcript (all 572 lines of your two-hour session), and every note tab. That makes syncing simpler and faster than planned.
- **Times:** Plaud stores them in UTC. Your recording named "2026-09-09 17:27:14" has a start time of 16:27:14 UTC, which is 17:27 in British Summer Time. So the times in Plaud's names are local, and NoteFetch converts correctly.
- **Transcripts** have start and end times and speaker names. Another project said end times were missing; they are not.
- **Notes:** your recording had one note tab, "Summary", in Markdown. Other tabs, such as a template or a saved Ask Plaud answer, would each be saved as their own file named after the tab.
- **Listing:** 10 to 100 recordings per page, newest first. Your oldest recording is from February 2026.
- **Not testable:** whether listing stops at 500 recordings, since you have 71. Plaud's own description says the 500 limit applies to searching by name or date, which NoteFetch does not use.

### Proposal: organising recordings

Plaud has folders in its app, but they are invisible to outside apps. So NoteFetch would keep its own folders. My proposal, to change as you like:

- **A recording can be put in one folder,** such as "Neuroscience" or "Work training". On disk, the folder is a subfolder of the output folder, so File Explorer shows the same organisation.
- **A "Folder" combo box** after the "Show" combo box, with "All folders", "Not in a folder" and then each folder by name. Together with Show, this answers questions like "which neuroscience lectures have I not downloaded yet?"
- **A Folder column** in the list, after Name.
- **"Move to folder"** on the context menu and the Recording menu, with a shortcut such as Ctrl+Shift+M. It opens a small dialog: a list of existing folders, and an edit box to type a new folder's name. Moving works on several selected recordings at once.
- **Rename and delete folders** from a Folder menu. Deleting a folder moves its recordings back to "Not in a folder", never to the Recycle Bin.
- **Later, if useful:** automatic rules, such as "recordings whose name contains 'lecture' go to Neuroscience". Your recordings are mostly named by date until you rename them, so rules would rarely match, and I would leave this out of the first version.

### Questions

#### Question 1: .opus or .ogg?

Both extensions describe the same file. On your computer, .opus, .ogg and .mp3 all open with the same default program, so playing is fine either way. The difference is other programs: many upload forms on transcription websites accept ".ogg" but not ".opus". I recommend .ogg.

To hear it for yourself, there are two copies of your newest recording in the NoteFetch folder, under test-data, milestone-1, audio: "sample recording.opus" and "sample recording.ogg". Pressing Enter on each in File Explorer should play it. Did both play?

ANSWER:
Yes, both played. Agreed on using .ogg
#### Question 2: which transcription tool do you plan to use?

If it does not accept Opus audio at all, NoteFetch would need to convert to MP3, which means shipping a converter. I would rather know now.

ANSWER:
I'll be uploading lectures to Heidi Health to transcribe, using a lecture template I've set up there. Please check if you can whether it accepts .ogg -- I think it does, but not sure.
#### Question 3: is the folder proposal right?

Is "Folder" the right word, or would you prefer "Course", "Group" or something else? Is one folder per recording enough, or do you want a recording in several at once, like tags?

ANSWER:
Folder is good, and ideally what shows in the app will also mirror what shows in Windows Explorer, so tags may be a bit much.
#### Question 4: which extra transcript parts do you want?

Besides the transcript and the notes, Plaud can provide:

- an AI-cleaned transcript, tidier to read but further from what was actually said (empty for the recording I tried);
- an outline of the recording's topics;
- the moments you flagged with the button on the recorder while recording, if you use it.

Which of these, if any, should NoteFetch save?
ANSWER:
Everything for now please. No empty placeholders for recordings where these items don't exist.

## 2026-09-26: Your answers applied, repository created, and the probe ready for your sign-in

### The short version

- **I need you for one step.** A Plaud sign-in page should now be open in your default browser. Please sign in there. The page afterwards says "Signed in to Plaud"; then close the tab. I am waiting for up to five minutes. When it succeeds, I run the rest of the probe and report here.
- **All your answers are in the plan.** PLAN.md records each decision with your words; SPEC.md, TASKS.md and CLAUDE.md are updated to match.
- **Milestone 0 is done.** The private repository is at github.com/ldefontenay/notefetch, with three commits.
- **The milestone 1 probe is written and has passed its first step.** Plaud's discovery documents read exactly as in PLAN.md.

### Your free plan

Plaud's own pricing page lists "Plaud MCP server (Beta)" for every plan, including the free Starter plan. One outside article says Pro is needed. The first call after your sign-in settles it. If Starter is refused, I stop and tell you here before anything else.

### What changed in the plan, from your answers

- **rmcp dropped.** NoteFetch gets its own small client, adapted from plaud-cli, with Lee Gonzales's MIT notice kept in the code.
- **Web requests use Windows' own security layer.** The web library's newest version defaults to a different one that needs extra build tools, so I switched it back to Windows' own, as PLAN.md intended.
- **Names.** Your examples showed that Plaud's default name is the date and time, and that you add the date and time to renamed ones too. So NoteFetch removes a date and time from the end of a Plaud name before adding its own. An unnamed recording becomes "Untitled, 2026-09-09 1727", so all the unnamed ones sit together under U in File Explorer, ready to rename. Say if you would rather have something else.
- **Statuses.** "No transcript", "No summary", and "Audio only" when both are missing, as you asked. "Waiting for transcript" is gone.
- **Organising.** You asked for features to help you stay organised. Plaud's documentation shows no folder or tag options in the server, but the probe checks whether the replies carry folder or tag information anyway. I will design the feature from what the probe finds and bring it to you before building. If Plaud gives us nothing, NoteFetch can keep its own groups, for example one per course, each with its own subfolder.
- **The other changes you accepted** (fresh registration at each sign-in, UTC times, list keys only in the list, an early announcement test, plain error messages) are all recorded.

### Milestone 0: the repository

- Created privately on GitHub with the planning files, log.md and the MIT licence (the same wording as StableSound's).
- GitHub's secret scanning is not available for private repositories on your account. GitHub replied "Secret scanning is not available for this repository".
- Instead, I installed gitleaks with winget and added a check that runs before every commit. It is in the repository's hooks folder, so it survives a fresh copy. I tested it: it refused a commit holding a made-up GitHub token.

### One change to your words

Your example of a renamed recording named a workplace training course. The repository will be public one day, with its whole history, so I replaced it with "Staff training 2026-09-24 09:00:26" in PLAN.md, SPEC.md and in your answer in this log, with a note saying so. The date and time, which were the point of the example, are unchanged.

### The probe

It is a small throwaway program in spike/plaud-probe. It prints plain text only and never prints tokens, client IDs, codes or audio links. Anything it saves goes in test-data, which git ignores. It does, in order:

1. Discovery (done): the server, sign-in and registration addresses, PKCE, and a public client with no secret are all as PLAN.md says.
2. Sign-in (waiting for you): registers on a free port, opens your browser, and records how long tokens last.
3. The tool list and their parameters.
4. Your account details, which is also the free plan check.
5. list_files at page sizes from 1 to 1000, then every page, to count recordings and see whether paging stops at 500.
6. Any folder or tag fields.
7. Whether times are UTC, by comparing recordings whose name is still their date and time with their start time.
8. get_file, get_transcript and get_note on your newest recording.
9. Downloading that recording's audio, to learn its format.

Token refresh is checked later, once the first token has expired.

## 2026-09-26: Plan review, and what already exists on GitHub

### The short version

- **Good news from GitHub.** A small open-source Rust program, plaud-cli by Lee Gonzales (MIT licence), already signs in to Plaud's hosted server exactly the way our plan intends: it registers itself, listens on a loopback address, and gets tokens. It has published Windows builds since June. So the biggest risk in the plan, whether Plaud accepts a desktop app's loopback sign-in, is very likely already answered. Milestone 1 still confirms it on your account, but it is now a confirmation rather than a gamble.
- **Nothing on GitHub does what NoteFetch does.** There are over thirty Plaud projects. None has a proper Windows interface, and none mentions screen readers. NoteFetch is still worth building.
- **I recorded your Log answer.** Newest entry at the top. It is in PLAN.md with your words, and CLAUDE.md now says so.
- **Seven questions for you below**, and eight changes I would like to make. I have not made any of the eight yet; they wait for your answers.
- No code written, nothing committed.

### What I found on GitHub

I searched for Plaud projects and read the ones that use the same hosted server as us.

#### The most useful: plaud-cli (Rust, MIT licence)

A command-line tool that lists, downloads and exports Plaud recordings. It talks to https://mcp.plaud.ai/mcp directly, as NoteFetch will. What it tells us:

- **Sign-in works with a loopback address on any port.** It picks a free port each time, registers a new client for that exact address, then signs in. Registering fresh at every sign-in sidesteps the question of whether the port can change. Its comment says it does this "to dodge loopback port-mismatch issues", which suggests the port must match the registration.
- **The sign-in request carries a "resource" value** naming the server address. Easy to miss, and without it the tokens might not work.
- **After a token refresh, the server session has to be started again**, because the server ties the session to the old token. Another easy one to miss.
- **An expired sign-in shows up as "invalid_grant"**, which it turns into a plain "sign in again" message. That is what our spec asks for.
- **It does not use rmcp**, the official Rust MCP library in our plan. It talks to the server with about 250 lines of its own code, because it only needs four calls. See question 3.
- **The shape of the replies**, which we had planned to discover in milestone 1:
  - list_files returns a list under "data", with id, name, created_at (upload time), start_at (recording start), duration in milliseconds, and the device serial number.
  - It pages at 100 per page. Its documentation says the page size must be at least 10.
  - get_file returns presigned_url for the audio, which it saves as mp3.
  - get_transcript returns segments with start time, speaker and text, wrapped inside a text field that holds more JSON.
  - get_note returns several blocks, each with a tab name, a title and a type. That answers how to name extra summaries: by their tab name, for example "Meeting Minutes".
- **Its sync had a bug** that could skip recordings uploaded in the same second across two runs. Our design compares against a database of Plaud IDs rather than a "last synced" date, so it does not have this weakness. Worth a test anyway.

#### Other useful findings

- **plaud-mcp-connector** (Python, MIT) and **plaud-obsidian-sync** (JavaScript, MIT) both use Plaud's official server. Between them:
  - Plaud's times have no time zone on them. They are UTC and must be converted to local time. Getting this wrong would put the wrong time in every folder name.
  - The name search in list_files only covers the newest 500 recordings. It is unclear whether plain paging also stops at 500. That matters for the "whole history" choice on first sync, so milestone 1 should check it if your account has enough recordings.
  - Transcripts from the server have start times but no end times.
  - A recording can keep its audio with no transcript indefinitely. Our 14-day retry rule covers this.
  - One of them says the Plaud developer features need a paid plan (Pro or above), and the other says Cloud Sync must be on. See question 1.
- **plaudit** (Rust) uses a different route: Plaud's REST developer API, borrowing the sign-in registration of Plaud's own command-line tool. I would not copy that. Using another program's registration is the kind of thing Plaud's terms may forbid, and the hosted MCP server is the route Plaud invites third-party apps to use.
- **Plaud_BulkDownloader** (C#, MIT) downloads everything using your Plaud password and the unofficial web API. Not a route for us, but it makes one folder per Plaud folder (tag). See question 5.
- **Many others** are Obsidian plugins, browser extensions, AI pipelines, or Bluetooth work that talks to the recorder directly. None is relevant to a Windows app for blind users.

#### Licences

plaud-cli, plaud-mcp-connector, plaud-obsidian-sync and Plaud_BulkDownloader are MIT, like NoteFetch. We can borrow code from them, keeping their copyright notice and crediting them in the README. plaudit has no licence file, so its code cannot be reused.

### Changes I would like to make to the plan

None of these are made yet.

1. **Register a new client at every sign-in, on a free port**, as plaud-cli does. Keep the client ID with the tokens in auth.bin, because refreshing needs it. This removes an open question and a fallback case.
2. **Treat Plaud's times as UTC** and convert them to local time for folder names and the Date column, with unit tests around the clock change in March and October.
3. **Keys that only belong to the list** (Enter, Delete, F2, Ctrl+A, Ctrl+T, Ctrl+M, Ctrl+D) should be handled by the list itself, not by the window-wide accelerator table. The plan puts the accelerator table ahead of everything else, so Delete would move a recording to the Recycle Bin while you were typing in the filter box, and Ctrl+A would select all recordings instead of all the filter text. This is an accessibility fault worth fixing before it is built.
4. **Fix a mismatch between the plan documents.** PLAN.md says spoken announcements are checked in milestone 5; TASKS.md puts them in milestone 6. I suggest a tiny test in milestone 3 instead, when the main window first exists. The whole announcement design depends on whether JAWS speaks them, and neither the skill nor StableSound or TweakSound has tested it yet.
5. **Add a status for a missing summary.** The list statuses include "Waiting for transcript" but nothing for a summary that has not arrived. I suggest "Waiting for summary", and "Waiting for transcript and summary" when both are missing. See question 6.
6. **Shrink milestone 1** to confirming what plaud-cli already shows, plus the things nobody has published: token lifetimes, the audio format, whether paging stops at 500, and what your recording names look like.
7. **Update the sources in PLAN.md** with these projects, and correct one date. PLAN.md says Plaud's MCP came out on 12 May; one project says Plaud "shipped an official MCP server on 2026-08-06". Probably the npm server and hosted server launched at different times. I would check Plaud's changelog again.
8. **Never show the server's raw reply when sign-in or refresh fails.** plaud-cli prints it. We should say what happened in plain words instead, because the raw reply is unreadable and might echo back part of what was sent.

### Questions

#### Question 1: which Plaud plan do you have?

One project says Plaud's developer features, including the MCP server, need Pro or above, and another says Cloud Sync must be on. If your plan does not include it, we need to know before anything else. You can find it in the Plaud app under your account or subscription.

ANSWER:
I'm on the free plan.
#### Question 2: roughly how many recordings are in your account, and what do their names look like?

The count tells us whether milestone 1 can test paging beyond 500. The names matter for folder naming: if Plaud's default names already contain the date, such as "09-24 Lecture", our date suffix would repeat it. Two or three examples copied from the Plaud app would settle it.

ANSWER:
From the Plaud web app: recording I tweaked the name of: "Staff training 2026-09-24 09:00:26" (the name changed here to keep work details out of the repository); Recording with unaltered name: "2026-09-09 17:27:14"
#### Question 3: may I drop rmcp and write a small client of our own, as plaud-cli does?

- **For:** we only need four calls. rmcp's sign-in code has changed between versions (CLAUDE.md warns about this), and it would bring in more code than the rest of NoteFetch. A small client of our own, perhaps based on plaud-cli's, is about 250 lines that we control and can test.
- **Against:** we would have to follow any change to the MCP protocol ourselves.
- **What it adds instead:** a few small, common libraries for the web requests and the sign-in maths (reqwest, sha2, base64, rand and url). CLAUDE.md says to ask you before adding dependencies, so I am asking. rmcp needs most of these anyway, underneath.
- **My recommendation:** yes, write our own. I would record the reason in PLAN.md.

ANSWER:
Yes.
#### Question 4: may I borrow code from plaud-cli, with credit?

Mainly the sign-in and the server calls. The MIT licence allows this if we keep Lee Gonzales's copyright notice in the relevant file and credit him in the README. I would adapt it to our rules: no printing of secrets, plain-English errors, and an accessible sign-in page.

ANSWER:
Yes.
#### Question 5: do you use folders or tags in the Plaud app?

Plaud lets you sort recordings into folders. The plan ignores them and puts everything in one output folder. If you use them for courses, for example, NoteFetch could show them in the list or make one subfolder each. That would be a later version unless it matters to you now.

ANSWER:
I would like to be better organised, so please implement relevant features.
#### Question 6: wording for a missing summary

Are "Waiting for summary" and "Waiting for transcript and summary" clear to you, next to the existing "Waiting for transcript"? Or would you prefer a shorter form?

ANSWER:
Rather use "no summary" or "no transcript".
#### Question 7: happy with the eight changes above?

Say which, if any, you do not want.

ANSWER:
Happy with all of them.
## 2026-09-26: CLAUDE.md updated, and this log started

### The short version

- From now on, everything I would have told you in the terminal goes in this file instead, newest entry first, as in TweakSound. The terminal will only say "see log.md".
- CLAUDE.md now tells every future session to load the accessible Windows apps skill before any interface work, and to run its probe on each window before asking you to test.
- There is no code yet. Nothing was built or run.

### What I changed in CLAUDE.md

- **Opening line.** The standard sentence saying the file is guidance for Claude Code.
- **Reporting to you.** A new section setting out this log: level 2 heading per entry with the date, plain lists, no tables, each question just before its own ANSWER line, and a one-line terminal reply.
- **Accessibility knowledge from earlier projects.** A new section that:
  - points at the skill in your .claude skills folder, which holds the StableSound and TweakSound findings with their proven, reasoned and untested markers;
  - gives the two probe commands to run before any test round;
  - lists the proven findings that matter most here: labels immediately before their controls, group box headings not read by JAWS, multi-line edits not read on focus with the caret landing at the end, and tones only for what you did;
  - notes what the skill does not yet cover (list views, context menus in a dialog, in-place renaming, UI Automation announcements), so those get recorded as new findings and added back to the skill.
- **Current state.** Says there is no code or git repository yet, and that the milestone 1 spike must answer the open questions first.
- **Commands.** How to run one module's tests or a single test, the hidden sync flag, and the spike. The file notes that the last two do not exist yet.
- **Architecture, marked as planned.** The interface thread and worker thread, the eight engine modules in one line each, and the rules that span modules, such as sync never touching Deleted or Skipped recordings and audio links being kept out of logs.
- **Test scripts.** Now written by me as test-milestone-N.txt files, with each step just before its ANSWER line and an ANYTHING ODD line per section, the same as TweakSound. Previously CLAUDE.md said you would write the file yourself.

### An accessibility point I noticed in the spec

The Log in the main window is a multi-line read-only edit box. In TweakSound you found that JAWS does not read a multi-line edit when you Tab to it, and that the caret lands after the last line once text is set. The same will happen here unless it is designed for. I have written this into CLAUDE.md so milestone 3 deals with it. It raises one question for later, not urgent:

#### Question 1: where should the caret sit in the Log?

The Log grows as things happen, so new lines arrive at the bottom. When you Tab to it, would you rather the caret sat on the newest line (so the up arrow goes back in time) or at the very start (the oldest line)? An alternative is to put the newest entry at the top, like this file.

ANSWER:
Put the newest entry at the top.
### Other things I checked

- There are no settings files from Cursor, Copilot, Codex or Gemini in the project, so there was nothing to import.
- I saved a memory note so future NoteFetch sessions report here rather than in the terminal.
