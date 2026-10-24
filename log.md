# Project log

Newest entry first. Each entry starts with a level 2 heading holding the date and a short title, so you can jump between entries with your screen reader's heading keys.

## 2026-10-24: Milestone 6 is signed off; the release is waiting on your decisions

### The short version

- **Round 2 passed, every step, with JAWS and NVDA.** Thank you. Milestone 6 is signed off, with your words in PLAN.md. Good news that Narrator speaks too.
- **"Test a spoken announcement" is gone** from the Help menu, the guide and SPEC.md, as you asked. The guide's troubleshooting now says to press Ctrl+S and listen for "Syncing with Plaud." instead.
- **The skill is updated:** JAWS and Narrator do hear the new announcement method, so other apps can use it for every screen reader, with the live region added for NVDA.
- **The README** now says NVDA was tested in two rounds and Narrator speaks announcements.
- **The history scan is done again:** 19 commits, no secrets, no test data, no recording names. One thing to decide about your email, below.
- **No test this time.** What is left of milestone 7 is yours to decide: four questions below.
- 94 tests pass, clippy is clean. All committed.

### What is left before the release

- Writing to Plaud. A draft is below. I have not sent anything or contacted anyone.
- The trademark registers. Web searches find no product called NoteFetch (only Fetchnotes, a notes app from 2012 to 2013, already known). The UK IPO and EUIPO search pages do not work for me, so the registers themselves are still unchecked. A search for "NoteFetch" at trademarks.ipo.gov.uk and at the EUIPO's eSearch takes a couple of minutes.
- Making the repository public, then the first release (tag v0.1.0; the build attaches the exe and zip to a draft release for you to publish).

### Your email in the history

- Your Gmail address is not in any file, but it is the author address on all 19 commits, and it becomes visible once the repository is public. That is normal on GitHub, and your public projects may already show it. If you would rather not, GitHub can give you a private "noreply" address, and I can rewrite the history to use it before going public. Rewriting is safe now, while nobody else has a copy.

Shall I rewrite the author address to your GitHub noreply address before the repository goes public?

ANSWER:
No, ok to use laurentd@gmail.com, same as for PhotoScribe.
### Draft letter to Plaud

Subject: NoteFetch, a free accessible Windows app for the Plaud MCP server

Hello,

I am a blind developer and a Plaud Note Pro user. I have written NoteFetch, a free, open-source Windows app (MIT licence) that lets screen reader users download their own Plaud recordings, transcripts and summaries to their computer, through your hosted MCP server at mcp.plaud.ai. Plaud's app and website are hard to use with a screen reader, and NoteFetch makes the recordings fully usable by keyboard with JAWS and NVDA.

It signs each user in with their own Plaud account through your OAuth flow, using a fresh client registration and a loopback redirect, the same way as the open-source plaud-cli. It only reads: it uses list_files, get_file and get_transcript, never changes anything in Plaud, and sends nothing anywhere else. Sign-in tokens are encrypted on the user's computer.

Before I publish it on GitHub, I would like to check two things:

1. Is this use of the MCP server within your terms, including for other people who download the app?
2. The app's name, NoteFetch, does not use the Plaud name, and its documentation says it is not made, endorsed or supported by Plaud. Is it all right to say that it works with Plaud, and to name Plaud in its description?

I would be glad to share the code or answer any questions.

Kind regards,
Laurent Cadet de Fontenay

Is the letter right, and shall it go through Plaud's developer contact page under your name? (You would send it yourself, or tell me to fill in the form.)

ANSWER:
I'm not going to proactively contact Plaud. I'll react if approached.
### Going public

When the letter is answered, or before? Shall I make the repository public once you say so, and then tag v0.1.0 for the first draft release?

ANSWER:
Make it public now. Would it be appropriate to change the version number to v1?
Do you want to check the UK IPO and EUIPO registers yourself, or shall I try again another way?

ANSWER:
I'll approach any concerns reactively.
## 2026-10-24: NVDA now speaks, and About is a window

### The short version

- **Thank you for the round.** The guide, the error messages, the announcement levels and F1 are signed off, with your words in PLAN.md.
- **NVDA now speaks NoteFetch's announcements.** I found out why it was silent without needing you: NVDA never receives the kind of announcement JAWS uses from a window like NoteFetch's. When NVDA is running, NoteFetch now uses the method NVDA does listen for. JAWS keeps the old method, slightly changed.
- **The recording is no longer read two or three times.** After a sync, NoteFetch emptied and refilled the list, and NVDA re-read the selected row each time, cutting off "Sync finished." Now only what changed is changed. At start-up the list now fills before the window appears.
- **Tab in the rename box** finishes the rename, as Enter does, and stays on the same recording.
- **About NoteFetch** is a small window: a read-only box with the version and "By Laurent Cadet de Fontenay, with assistance from Claude", and OK.
- **Please run test-milestone-6-round-2.txt.** 15 steps, about 30 minutes. JAWS first, because that is the part I could not check.
- 94 tests pass, clippy is clean, and the probe finds no faults in the main window or About. All committed.

### What I did with NVDA on your computer

- NVDA was running when I started. To see exactly what it said, I restarted it twice with its logging turned up, ran NoteFetch's test copy and pressed keys in it, and read NVDA's log. NVDA will have spoken aloud while I did. I also opened Calculator once, to check that NVDA hears other programs' announcements (it does).
- Afterwards I put NVDA's settings file back exactly as it was and started NVDA normally. It is running now, as you left it.

### What was wrong with NVDA, briefly

- JAWS and NVDA listen for announcements in different ways. NoteFetch's way reached JAWS but never reached NVDA at all: NVDA's own log showed nothing arriving, while Calculator's arrived fine.
- When NVDA is running, NoteFetch now writes what it wants said into a hidden text box in whichever NoteFetch window is in front, and tells NVDA it changed. NVDA reads it. Lines said together are joined, and wait about a seventh of a second, so a menu closing does not cut them off.
- The rest is in PLAN.md, "Milestone 6 results, and NVDA", and in the accessible apps skill, so other apps can be fixed the same way.

### For other apps

- The skill now has the NVDA method, with code, the list lesson ("change only what differs"), the Tab-in-rename fix, and two small tools I wrote to check announcements and focus without a screen reader. Any of your other apps that announce the way NoteFetch did will likely be silent with NVDA too, and can be fixed the same way.

### One small change you should know about

- The new JAWS method needs a part of the Windows library to be named in the project's list of dependencies. It is not a new dependency: it was already built into NoteFetch as part of the Windows library, and nothing new is downloaded or shipped. I hope that is all right.

### The test announcement item

- You would like "Test a spoken announcement" removed once NVDA works. I have left it in for this round, since steps 3 and 10 use it. The test asks whether to remove it now.

## 2026-10-24: The user guide is written, and milestone 6 is ready to test

### The short version

- **Sounds round 2 passed,** all seven steps. Thank you. The sounds are signed off, with your words in PLAN.md.
- **The user guide is written** and opens with F1, from anywhere in NoteFetch, including Settings. It follows your Clipman and Photo Scribe guides: eleven little tutorials with worked examples, then troubleshooting, the keys and the files.
- **Every error message now says what to do next.** I read every one in the code and fixed about a dozen that did not.
- **The release groundwork from milestone 7 is done too:** an automatic build on GitHub, a first README, a scan of the whole history for secrets (clean), and SignPath's free code signing checked (NoteFetch looks eligible once public).
- **Please run test-milestone-6.txt.** 19 steps, about 40 minutes, plus NVDA if you have it, plus reading the guide and the README at your own pace. Your verdict on the guide is the main thing.
- 94 tests pass and clippy is clean. All committed.

### Before you start

- A copy of NoteFetch from your last test was still open. I did not close it. I moved its file aside so I could build the new version, which Windows allows while it runs, and it carries on unharmed. Please close it with Alt+F4 before the test; the new version starts only once it is closed. I will tidy the old file away afterwards.

### The guide

- Parts are level 2 headings and tutorials level 3, as in Photo Scribe. Each tutorial has "What you will learn", "Worked example" and "What just happened" as level 4 headings. The test asks whether those are helpful or too many.
- The quick reference is a set of lists, not a table, because a list item reads the task and its key in one line. The test asks which you prefer.
- The worked examples are a term of lectures: renaming "Untitled" to "Memory, week 4", a folder for Cognitive neuroscience, a cancelled lecture to delete, and so on.
- The byline is "By Laurent Cadet de Fontenay, with assistance from Claude", as in your other guides.
- The Help menu now has "NoteFetch guide", "Test a spoken announcement" and "About NoteFetch", which says the version in the Log rather than in a message box.

### Error messages

What changed, for example:

- A reply from Plaud that NoteFetch cannot read used to name an internal tool, such as "list_files". It now says: "NoteFetch could not understand Plaud's reply. Try again later. If it keeps happening, Plaud may have changed how it answers, and a newer version of NoteFetch may be needed."
- "There is already a folder called ..." now adds "Choose a different name."
- A folder or recording that has vanished now says to press F5 to refresh the list.

### The release work

- **The automatic build:** every time I push, GitHub builds NoteFetch, runs the tests and clippy on a fresh Windows machine. When we tag a version, it also makes a draft release with the exe and the zip, for you to check and publish by hand. Nothing is published by itself.
- **Its first run found a real bug in the tests,** now fixed: they assumed the computer was on UK time, and GitHub's machines are on UTC, so every expected folder name was an hour out. NoteFetch itself was fine; the tests now always use UK time. The second run passed.
- **The README** is in README.md, in StableSound's style, without its em dashes. Section 6 of the test asks you to read it.
- **Still yours to decide, later:** writing to Plaud about the app and the name, checking the trademark registers, and making the repository public. I have not contacted anyone.

### What I could not check

- I did not open the new NoteFetch on your desktop, because your copy was open and you may have been using the computer. So F1 opening the browser, and F1 from Settings, are untested until your round. The menu letters and the guide's links, keys and spelling are checked by tests.

## 2026-10-24: The sounds passed, and the missing ones are fixed

### The short version

- **The four sounds passed as they are.** Thank you. Your words are in PLAN.md.
- **The missing sounds were a bug,** now fixed: renames, moves, deletes and folder changes report back to the window in a different way from a sync, and that way never reached the sounds.
- **Your changes are made:** Quiet is the starting volume; a rename that works is silent; Attention plays only before a question NoteFetch asks by itself (signing in, the first sync), not before Delete or the others you ask for.
- **Please run test-milestone-6-sounds-round-2.txt.** 7 steps, about 10 minutes.
- 88 tests pass and clippy is clean. All committed.

### When the sounds play now

- **Done:** a move, delete, skip, new folder, folder rename or folder delete that worked. Not a rename of a recording.
- **Finished:** a sync, download, sign-in or move to a new output folder that you started, and that went well.
- **Refused:** straight away when something you pressed is refused, including a change. During a sync, if the sync itself hits a problem, Refused plays once at its end in place of Finished.
- **Attention:** before "Sign in now?" and before the first-sync question. Nothing else.

### One more fix found on the way

- A refusal you pressed during a sync, such as starting a second sync, would have turned that sync's Finished into Refused at the end. Now the refusal sounds at once, and the sync still ends with Finished.

### What I could not check

- I cannot hear the sounds, so whether each now plays when it should is for round 2. I did not play any sound on your machine.

## 2026-10-24: Milestone 5 signed off, and the first draft of the sounds

### The short version

- **Milestone 5 is signed off.** Thank you. Your words are in PLAN.md.
- **Your last three changes are made:** Delete folder now defaults to No (deleting recordings stays Yes); Sign in is unavailable in the File menu while you are signed in, and Sign out while you are not; and UK spelling stays throughout, as you agreed.
- **Milestone 6 has begun with the sound palette.** NoteFetch now has four little sounds, wired in. Please hear them with test-milestone-6-sounds.txt: 9 steps, about 20 minutes.
- I also fixed a bug I found while checking: a settings file saved by Notepad or PowerShell could lose its first setting.
- 88 tests pass, clippy is clean, and the probe found no defects in Settings. All committed.

### Sign in in the File menu

- With no sign-in, Sign in is available and Sign out is not. Signed in, the reverse. I checked both by reading the menu on a scratch copy.
- The one case where Sign in stays available while a sign-in is stored: when Plaud has refused it as expired. Then you need Sign in to get going again, so NoteFetch offers it until a new sign-in is saved. The Settings button follows the same rule.

### The four sounds

Made exactly as TweakSound's are, keeping what you settled there: the lead-in of a twelfth of a second against Bluetooth clipping, and its volume as "Normal". The notes are new: only C sharp, D, F sharp and G, none of which StableSound or TweakSound plays, and a test makes sure it stays so.

- **Done:** two quick notes rising. A rename, move, delete, skip or folder change worked.
- **Finished:** Done's two notes and then a higher one. A sync, download, sign-in or move of your recordings that you started has finished.
- **Refused:** two notes falling to an unsettled note. Something could not be done, or went wrong.
- **Attention:** a note, a short gap, then the same note higher, like a call. Just before a question.

When they play:

- Only for what you did. The sync on start is silent unless something goes wrong.
- A refusal of something you just pressed sounds at once. During a sync, Refused plays once at the end, however many things went wrong, so a bad network does not make it beep over and over.
- Nothing plays for something you cancelled with Escape.

### Settings

- **Play sounds for what you do** (Alt+P), on to begin with.
- **Sound volume** (Alt+V): Quiet, Normal or Loud. Arrowing to a volume plays Done at it, so you can choose by ear.

### What I did on your desktop

- A scratch copy of NoteFetch opened a few times for the checks, with its sounds and announcements off, so nothing played or spoke. I closed it each time and deleted the copy. No keys were pressed on your behalf.
- I made a new file, "Play NoteFetch sounds.cmd", in test-data.

## 2026-10-24: Milestone 5 passed, your changes are made

### The short version

- **Milestone 5 passed.** Thank you, and thank you for the kind words. Your own words are in PLAN.md.
- **All seven of your changes are made,** listed below. 81 tests pass, clippy is clean, and the probe found no defects in the new first-sync wording.
- **Milestone 5 is committed,** with your results and these changes.
- **Please run test-milestone-5-round-2.txt.** 6 steps, about 10 minutes, only on what changed.
- **Two questions below:** UK or American spelling, and whether the Delete questions should also default to No.
- Step 15 (moving the recordings back) had no answer, but section 3 said "No, excellent", so I took it as working. Say if not.
- The earlier question about a recording put straight into the output folder was left blank. It stays as built, moved into Uncategorised at the next refresh, which you heard in section 1.

### Your changes

- **"Uncategorized" with a z** is now refused as a folder name too, and typing it in Move to folder means the Uncategorised folder.
- **Settings** now says "Download audio", "Download transcripts", "Download summaries" and "Download other parts".
- **The folder picker** opens one level up, in the folder holding the output folder, so you see the output folder beside its neighbours.
- **Moving recordings to a new output folder** now has No as the default, so Enter does nothing by accident.
- **Every sign-in hint** now says "from the File menu or from Settings".
- **The first sync** asks "How much of your Plaud audio, transcripts and summaries should it download now?"
- **The middle choice** is now "Specified duration, the last 30 days: 12 recordings, about 9 hours", with Alt+S, and the box below it is "Specified duration, number of days", so the two read as a pair.

### Question: UK or American spelling

You asked for my steer. I recommend keeping UK spelling everywhere NoteFetch speaks: the interface, messages, help and the folder name "Uncategorised". The reasons:

- It is your rule for all your projects, and it is how you and most of your likely first users write.
- JAWS says both spellings the same way, so a listener never hears a difference.
- Mixing the two would look careless in the help and the readme, which sighted readers will see.
- Where someone types a name, NoteFetch should accept both spellings, as it now does for Uncategorised. That covers the only place where an American user could trip over it.

The one place Windows itself is American, such as "Select Folder" in the folder picker, is outside NoteFetch's control and reads naturally anyway.

Do you agree: UK spelling throughout, both spellings accepted when typed?

ANSWER:
Yes.
### Question: should the Delete questions also default to No?

Your change makes No the default for moving recordings. The question before deleting a recording, and the one before deleting a folder, still have Yes as the default. A deleted recording goes to the Recycle Bin and can be brought back, so it is less risky than a move, but the same caution could apply. Should those two default to No as well?

ANSWER:
Deleting folders should default to no. Deleting recordings to yes, as it could get frustrated when deleting multiple recordings and there is a good safeguard.
## 2026-10-23: Milestone 5 built, ready for you to test

### The short version

- **Milestone 5 is built:** the Uncategorised folder, the Settings window, moving your recordings when you change the output folder, signing in and out from Settings, and the first-sync question.
- **Please run test-milestone-5.txt.** 22 steps, about 40 minutes. Section 5 needs one more sign-in in your browser, on a new empty test copy.
- 81 tests pass and clippy is clean. Not committed yet; I will commit with your results.
- The question in the entry below, about a recording put straight into the output folder, still stands. I built it the way I suggested, so section 1 shows you how it behaves.

### What is new

- **Uncategorised.** At the first start, NoteFetch moves every recording not in a category into a folder called Uncategorised. After that, new downloads go there. The Folder box, the Folder column and Move to folder all say "Uncategorised". It cannot be renamed or deleted, and no category can take its name.
- **Settings, with Ctrl+Comma** or File menu, Settings. Everything from the settings file is there, in the order of the spec, and each control reads on its own. OK saves at once; Escape leaves everything as it was.
- **Changing the output folder** opens the Windows folder picker. If the old folder holds recordings, NoteFetch asks whether to move them. Yes moves them all, keeping their categories. No takes you back to Settings. A move cannot be stopped half way, so nothing is left behind by accident.
- **Sign in or Sign out** is one button in Settings, whose name says which.
- **The first sync** now asks how much to download: everything, only the last so many days (30 to begin with), or nothing old. Each choice says how many recordings and roughly how many hours. Typing a number of days changes that choice's count as you type.

### What I checked

- I ran the accessibility probe on Settings and on the first-sync question. Every control has its label as its name, and every Alt letter is unique.
- On a separate copy of the test data, with no sign-in, no audio and announcements off, I started NoteFetch: it moved the 50 test recordings into Uncategorised. I opened Settings, ticked a box and pressed OK: the settings file changed and the Log said "Settings saved."
- I opened the first-sync question on its own, typed 200 days (the choice then said "Only the last 200 days: 24 recordings, about 10 hours"), tried OK with the box empty (it said what to type and kept the window open), and chose OK.
- What I could not check: the folder picker with JAWS, moving your real test recordings, signing in from Settings, and a real first sync. Those are sections 3 to 5.

### What I did on your desktop

- The separate copy of NoteFetch opened a few times for the probe, without taking the foreground. I closed it each time. No keys were pressed on your behalf, and your clipboard was not touched.
- I made a new file, "Start NoteFetch first sync test.cmd", in test-data, for section 5.

## 2026-10-23: Milestone 4 finished, and your three ideas are in the plan

### The short version

- **Round 2 passed.** Everything you could test worked. Thank you. Your words are in PLAN.md.
- **Milestone 4 is committed,** with round 2's work.
- **Your three ideas are planned:**
  - the Uncategorised folder, in milestone 5;
  - a sound palette in StableSound's style, in milestone 6;
  - a user guide with little tutorials, like your Clipman and Photo Scribe guides, in milestone 6.
- **Next is milestone 5:** settings, the first-sync question, sync on start, and the Uncategorised folder.
- **One small question below,** about recordings put straight into the output folder.
- 76 tests pass and clippy is clean.

### Round 2

- The menus: Open transcript and Open summary unavailable when there is none, and available when there is one. Confirmed.
- A recording being downloaded is refused, and finishes normally. Confirmed by your longer download.
- Changes to a recording waiting its turn were too quick to catch. As you said, that stays as it is. The unit tests cover it.
- The new Delete setting worked both ways.

### The sound palette

I looked at StableSound's tones: short notes with soft ends so they never click, rising for on and falling for off. NoteFetch would build its sounds the same way, in code, and play them through Windows itself, so no new library is needed.

- **A first draft of four sounds:** done (a rename, move or delete worked), finished (a download or sync you started), refused (something could not be done), and attention (a question, or a sign-in is needed).
- **Pitches of their own,** so NoteFetch cannot be mistaken for StableSound or TweakSound when they run together.
- **Only for what you did,** the rule StableSound taught: the sync on start stays silent unless something goes wrong.
- **You hear them before they go in.** A hidden command will play each one, so you can judge them in one sitting, as you did StableSound's.
- A setting for sounds on or off, and the volume.

### The user guide

I read your Getting Started with Clipman guide (on the getting-started-guide branch of your fork) and Photo Scribe's help. NoteFetch's guide will follow the same shape:

- About this guide, Why use NoteFetch, What NoteFetch does, and Words you will meet.
- Parts as level 2 headings, tutorials as level 3, each with "What you will learn", a worked example, numbered steps saying what to press and what you will hear, and "What just happened".
- The sounds NoteFetch makes, troubleshooting, a quick reference of keys, the files it keeps, and credits.
- Example tutorials: sign in and fetch your first recordings; play a lecture and open its summary; rename a recording; sort a term's lectures into a category folder; delete one and bring it back.

It is written in milestone 6, once everything it describes is built, so the steps are exactly what you will hear.

### Question: a recording put straight into the output folder

With the Uncategorised folder, the output folder should hold only folders of folders. But you could still move a recording folder straight into the output folder in File Explorer. My plan is that the next refresh moves it into Uncategorised, and the Log says so, so the app and File Explorer always agree. The other way would be to leave it there and show it as Uncategorised in NoteFetch, which means the two disagree. Is moving it into Uncategorised what you would expect?

ANSWER:
Yes.
## 2026-10-19: Milestone 4 passed, and your requests are done

### The short version

- **Milestone 4 passed in full.** All 29 steps worked. It is committed.
- **Your three requests are built:** changes during a sync, Open transcript and Open summary shown as unavailable when there is none, and a setting for what Delete does on recordings never downloaded.
- **Please run test-milestone-4-round-2.txt.** 11 steps, about 15 minutes. Section 2 needs quick hands during a download.
- **One question below,** about telling recording folders from category folders in File Explorer, with my ideas.
- 76 tests pass and clippy is clean. The new work is not committed yet; I will commit it with your round 2 results.

### What changed

- **Renaming, moving and deleting now work during a sync or download.** The only things refused are a recording being downloaded at that moment, and a folder holding one. The Log names the recording and says to try again when the download finishes. A recording only waiting its turn is not refused: if you delete it, or choose Don't download, the download leaves it out, and the Log says so.
- **Open transcript and Open summary** are unavailable in the context menu and the Recording menu when the recording has none. Ctrl+T and Ctrl+M still give the message you liked.
- **New setting, delete_marks_dont_download,** on by default. Off, Delete on a recording never downloaded does nothing and says to use Don't download instead. It goes into the Settings window in milestone 5; round 2 changes it by hand in the settings file.
- **The F2 edit box** stays as it is. JAWS reading the rest of the row after the edit box is JAWS reading the list behind it, and I know of no way to stop that from inside NoteFetch.
- **Your findings are in the accessibility skill:** Enter in a list, the context menu, the greyed items, in-place editing and focus after delete, each with your words, so the next project starts from them.

### What I checked

- The unit tests cover the new guard that keeps changes away from downloads.
- On a separate copy of the test data with no sign-in, I renamed a recording with F2 through the rebuilt background worker, and pressed Delete on a recording never downloaded with the new setting off. Both behaved as described. I then deleted that copy.
- What I could not check: a real download alongside a change, because that needs your sign-in. That is section 2 of round 2.

### Question: recording folders and category folders

You asked how File Explorer could tell a folder holding one recording from a category folder like Neuroscience. Here are the options I see.

- **Option 1, your idea: an "Uncategorised" folder.** Recordings not in a category go into a real folder called Uncategorised, so the output folder holds only category folders, and each of those holds only recordings. In NoteFetch, "Not in a folder" would become "Uncategorised" everywhere, so the app and File Explorer use the same word. The cost: one more level for everyone, a one-off move of your existing recordings into it (done safely, one recording at a time, the same way as Move to folder), and "Uncategorised" becomes a reserved name you cannot give a category.
- **Option 2: leave it as it is.** Every recording folder ends with a date and time, and a category folder never does. No work, but you have to listen to the end of each name to know.
- **Option 3: a marker in category names,** for example "Neuroscience (folder)". Easy to build, but it adds noise to every category name, and anything renamed in File Explorer would lose it.
- I would not use icons or colours: they tell a screen reader nothing.

My recommendation is option 1, always on rather than a setting, because the layout is then the same for everyone and the help can describe it simply. If you agree, I would build it in milestone 5, since it changes how the output folder is laid out and needs a careful one-off move.

Which option would you like? If option 1, is "Uncategorised" the right name?

ANSWER:
Yes, go with option 1, and "uncategorised" is the right name.
## 2026-10-18: Milestone 4 built, ready for you to test

### The short version

- **Milestone 4 is built:** Enter to play, the context menu, renaming with F2, deleting, the Deleted view, Don't download, Copy audio file path, Move to folder, and the Folder menu.
- **Please run test-milestone-4.txt.** 29 steps, about 40 minutes. It also checks your round 3 request that recordings stay in the list until a download finishes.
- **Four questions below.** Each has its own ANSWER line.
- All automatic checks pass: 74 tests, and clippy is clean. Not committed yet; I will commit once your results are in, unless you would like it sooner.

### What is new

- **Enter** plays a downloaded recording in your default player, and downloads one that is not downloaded, or that is in the Deleted view.
- **Ctrl+T and Ctrl+M** open the transcript and the summary in your own programs. If there is none, the Log says so and why.
- **Ctrl+Shift+C** copies the full path of the audio file.
- **The Applications key and Shift+F10** open a context menu for the focused recording, at that recording, not at the mouse. Its items depend on the status, and its default item is what Enter does. With several recordings selected, the items that only make sense for one are unavailable, so the menu keeps the same shape.
- **The Recording menu** (Alt+R) has the same commands, set up the same way each time it opens.
- **F2** edits the name in place, as in File Explorer. Only the name is in the edit box; the date and any "(2)" are kept for you. Enter renames the folder and every file in it; Escape leaves it as it was.
- **Delete** asks first, then moves the recording's folder to the Recycle Bin and lists it in the Deleted view, where Enter downloads it again. Nothing is deleted in Plaud.
- **Don't download** is in the context menu for recordings not yet downloaded.
- **Ctrl+Shift+M** opens "Move to folder": a list of your folders, and a box for a new folder name.
- **The Folder menu** (Alt+L) has New folder, Rename folder and Delete folder. Rename and Delete act on the folder chosen in the Folder box, and the menu says which: "Rename the folder "Neuroscience"...". When none is chosen, they say "choose one in the Folder box first".
- None of these need a Plaud sign-in: they only change your disk.

### What I checked before handing it over

- I ran the accessibility probe on the main window, the Move to folder window, the folder name window and both questions. Every control has its label as its name, and the Alt letters are right.
- I drove a separate copy of NoteFetch with simulated keys: Enter, F2 then Enter, F2 then Escape, Delete with No and with Yes, Move to folder with a new folder, rename and delete a folder, and the context menu. All worked.
- **One real problem found and fixed:** after deleting, focus moved to the next recording but nothing was selected, so pressing Delete again said "No recordings are selected". Now the next recording is focused and selected.
- **Another fixed:** if a player has the audio open and a rename is refused, the message could have told you to restart Windows. It now says a file is open in another program, such as a media player, and to close it and try again.
- What only you can tell: how JAWS reads the context menu and whether it says which item is the default, how it reads the F2 edit box, and where focus lands after a delete.

### What I did on your desktop

- I opened the separate copy of NoteFetch a few times, with announcements off. Once, for about three seconds, it took the foreground so I could press Shift+F10, Ctrl+T and Ctrl+Shift+M for real; then I gave the foreground back to whatever had it.
- For the Copy audio file path check, I saved your clipboard first and put it back afterwards. If anything you had copied is missing, that is why, and I am sorry.
- The delete check sent two small test folders to your Recycle Bin, each holding only a metadata.json file from the test copy, no audio or text. You can empty them away.

### Questions

#### Question 1: renaming a recording with "(2)"

Two recordings started in the same minute are told apart by "(2)". If you rename the second one to a name nothing else has, for example "Short test", should the "(2)" go? I have made it go, because it no longer tells anything apart: you get "Short test, 15 Oct 2026, 1229". If the new name does clash, for example you name both "Lecture", the second keeps its "(2)". Step 16 shows it. Is that right?

ANSWER:
Yes, that's right.
#### Question 2: changes wait for a sync or download

Renaming, moving and deleting wait until a sync or download has finished. Renaming a folder while a download is writing into it would break the download, so this keeps things safe and simple. The cost: NoteFetch syncs as it opens, so for the first moments after opening, F2 or Delete just says it cannot start yet. Is that acceptable, or would you like these to work during a sync (which I can do, with care, for recordings the sync is not touching)?

ANSWER:
If possible, allow actions on recordings unaffected by the sync.
#### Question 3: the default answer when deleting

The delete question has Yes as its default button, as File Explorer's does, since the Recycle Bin can undo it. So Delete then Enter deletes. Would you rather No were the default, so Delete then Enter does nothing until you press Y?

ANSWER:
Keep the Windows Explorer convention.
#### Question 4: Delete on a recording that was never downloaded

There is nothing on disk to delete, so Delete marks it "Don't download" instead, without asking, and it moves to the Deleted view, where Enter downloads it after all. Is that what you would expect, or should Delete do nothing for those and say to use Don't download?

ANSWER:
Delete moves to "don't download" on such recordings by default, but make that a setting so that optionally delete does nothing.
## 2026-10-18: Milestone 3 finished, committed and pushed

### The short version

- **Round 3 passed.** Escape in the Log cancels and NoteFetch stays open, and the sync Log makes sense. Thank you.
- **Your two answers are built:**
  - Downloading new recordings when syncing is now a setting, on by default, as you chose. Until the Settings window arrives in milestone 5, it is the line `sync_downloads_new` in the settings file.
  - Recordings stay in the list until the whole download finishes, then the view catches up. So "Not yet downloaded" no longer shrinks under you while it runs. If you change the Show or Folder box or the filter during a download, the list follows your choice at once.
- **Your round 3 notes are fixed too:**
  - The list now shows "Untitled (2)" for the second of two recordings started in the same minute.
  - A sync's progress says "Fetched 1 of 6." rather than "Downloaded", since a sync also counts recordings it only checked. Download selected still says "Downloaded".
- **Milestone 3 is committed and pushed.** All automatic checks pass: 67 tests, and clippy is clean.
- **Next is milestone 4:** Enter to play, the context menu, renaming, deleting and folders. Its test round will also check the two list changes above with JAWS, since you have not heard them yet.
- **One small edit to your round 1 answers:** the JAWS speech you pasted into test-milestone-3.txt included the name of one of your real recordings. The repository will become public, so I replaced that name with "[name of a real recording, removed before committing]". Nothing else in your answers was changed.
- No questions this time.

## 2026-10-18: Round 2 results, Escape fixed, a clearer Log, and Plaud checked again

### The short version

- **Thank you for round 2.** Signing in, signing out, downloading, progress and closing all worked as designed. Your words are recorded in PLAN.md.
- **Escape in the Log closed NoteFetch.** Fixed: it now cancels, as everywhere else.
- **The Log after a sync is clearer.** It now says what it is about to fetch and why, and the "Checked ... again" lines are folded into one.
- **Plaud still cannot rename recordings.** I asked Plaud's server today: same five read-only tools as a month ago.
- **Please run round 3:** test-milestone-3-round-3.txt, 5 steps, perhaps 10 minutes. No password needed.
- **Two questions below**, about what a sync should download, and the close question.
- All automatic checks pass: 67 tests, and clippy is clean.

### Escape in the Log

The Log is a multi-line edit box, and Windows lets that kind of box take Escape for itself and close the window it sits in. Closing does not ask while NoteFetch is only waiting for a sign-in, so it just quit. The Log now leaves Escape to the window, which treats it as Cancel.

I checked it by sending Escape to the Log myself: without the fix NoteFetch closed, with it the Log said "Nothing is running, so there is nothing to cancel." Steps 2 to 4 check it with JAWS. I have added this to the accessibility skill, since any window with a multi-line box would have the same trap.

### Why the Log you pasted was confusing

- **"None are new", then 2 downloaded.** Those two, 2 June and 23 July, were downloads your Escape had stopped half way in step 10 or 11. The sync finished them, which is right, but the Log never said so. Now a line just after "Plaud has 79 recordings" says, for example: "Fetching 8 recordings: 2 not finished last time, and 6 to check again for a transcript or summary."
- **The six "Checked ... again" lines.** These are recordings from the last 14 days that Plaud has not transcribed yet. NoteFetch looks again at each sync. They are now one line: "Checked 6 recordings again for a transcript or summary: Plaud has nothing new for them yet." Download selected still gives each recording its own line, since you chose them.
- **"Untitled, 15 Oct 2026, 1229" twice.** Not a mistake, as it turns out: you have two recordings that both started at 12:29 on 15 October, one an hour long and one 20 seconds. Their folders differ by "(2)", but the Log left that off. Messages now use the folder's own name, "(2)" included. Step 5 checks it.

### Ctrl+A

I tried two ways of getting "All 79 recordings selected." heard first, and measured what Windows tells JAWS each time. Both still send JAWS one "selected" message per recording, 78 of them, so neither would help, and I took them out. It stays as you heard it in round 2, which you said is fine. What I learned is in PLAN.md and the skill. There is one more idea, a different kind of list, which I would only try if the list needs rebuilding for another reason.

### Plaud's tools, checked again

- I added a hidden developer command that asks Plaud's server for its tool list. It still has the same five: list recordings, get a recording, get a note, get a transcript, and get the current user. All read-only, so renaming in Plaud, or moving recordings into Plaud's folders, is still not possible.
- One small improvement: listing recordings by date now works on Plaud's server, exactly and with no limit. NoteFetch does not need it yet.
- Searching the web, I found Plaud's own documentation (https://docs.plaud.ai/documentation/personal_access/mcp) still describes the same server. I also found an unofficial project (https://github.com/rggnkmp/plaud-connector) that can rename, by using Plaud's private app interface without Plaud's say-so. I would not build on that: it could break or be blocked at any time, and it may not be within Plaud's terms. NoteFetch keeps to the published server. Renaming in NoteFetch stays local, as planned.

### Questions

#### Question 1: should a sync download new recordings?

You wrote that all new recordings were actually downloaded, not just synchronised, and you were still thinking about it. At the moment Sync new lists what Plaud has and then downloads every new recording: audio, transcript and summary. The choices I can see:

- A: keep it as it is. Sync new means "get everything new".
- B: Sync new only lists. New recordings appear in the list as "Not yet downloaded", and you choose what to download with Ctrl+D.
- C: a setting, "Download new recordings when syncing", on or off, with one of those as the default. Settings arrive in milestone 5.

My suggestion is C with downloading on, so it works without thought for most people, but B suits anyone whose Plaud records a lot they never want on disk. Which do you prefer?

ANSWER:
Yes, go with C with downloading on by default.
#### Question 2: what changed after you said No to closing?

In step 11 you wrote that the total of recordings to be downloaded seemed to refresh after you said No. NoteFetch's own progress keeps the same total all through a download, so I think what you heard may be JAWS's count of rows in the list, "3 of 60" and so on. In the "Not yet downloaded" view, each recording leaves the list as soon as it is downloaded, so that number shrinks while a download runs. Does that sound like what you heard? And if so, would you rather downloaded recordings stayed in that view until the download finishes?

ANSWER:
Yes, that's probably what I heard. And it would make sense for the downloading recording to remain in the view until its download is complete.
### What I did on your desktop

To check the Escape fix and the Ctrl+A events, I opened NoteFetch briefly a few times, on a separate copy of the test data with announcements off. If a NoteFetch window flashed up or took the focus while you were working, that was me. I closed each one straight away.

## 2026-10-18: Sign in from the window, and round 2 of the milestone 3 test

### The short version

- **You can now sign in from NoteFetch.** The File menu has "Sign in to Plaud..." and "Sign out of Plaud". And when NoteFetch finds your sign-in has expired, it now asks "Sign in again now?", with Yes already chosen, so you no longer have to go looking for the command.
- **Please run round 2:** test-milestone-3-round-2.txt, 12 steps, perhaps 20 minutes. It covers signing in and out, cancelling a sign-in, Ctrl+A again, and the three download steps you could not do last time. Have your Plaud password to hand.
- **Thank you for round 1.** The rest of the window worked as designed, and your words are recorded in PLAN.md.
- All automatic checks pass: 65 tests, and clippy is clean.

### What your round 1 results settled

- **Announcements work with JAWS.** You heard the "sign-in has expired" message at start-up and the test announcement. This was the biggest open question in the design, so progress and completion can be spoken as planned, with no extra software.
- **The test announcement coming after the focused row** is JAWS finishing its own speech first, then ours. I have left it, as you suggested it is fine.
- **Ctrl+A:** JAWS read every name and "All 71 recordings selected." was lost. NoteFetch now waits a third of a second before saying it, in the hope that it gets through. Step 8 checks whether it does. If not, the Log still has it.
- **Your sign-in expired** because it was not used for three weeks: Plaud refused the one from 27 September. Plaud does not say how long a sign-in lasts, so I have noted it in PLAN.md. Anyone who uses NoteFetch only now and then will see the same, which is why NoteFetch now asks rather than just saying what to do.

### How signing in behaves

- NoteFetch opens Plaud's sign-in page in your browser and says so in the Log. Once you have signed in, close the tab and come back. NoteFetch says "Signed in to Plaud." and then syncs.
- While it waits, Escape cancels. It gives up by itself after five minutes.
- Sign out does not ask first, because it deletes nothing but the sign-in, and signing in again undoes it.
- Closing NoteFetch now asks only during a download or a sync. While it is only waiting for you to sign in, Alt+F4 just closes it.

### What I could not check myself

I have not opened the window this time, so I could not have taken the focus while you were working. Signing in also needs your Plaud account in the browser, so it has only been checked by the automatic tests. Round 2 is its first real run.

And please do not apologise for exploring the window while I was testing. It was my test window that took the focus. Nothing was harmed.

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

Sorry about that. Normally I take my hands off the keyboard when I notice you're running tests, didn't notice in time this time around, and explored the app a bit with the tab key and arrowed through the list, which lead to confusion.
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
