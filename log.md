# Project log

Newest entry first. Each entry starts with a level 2 heading holding the date and a short title, so you can jump between entries with your screen reader's heading keys.

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
