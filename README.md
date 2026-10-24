# NoteFetch

Download your Plaud recordings, transcripts and summaries to your own computer,
with a window built for screen reader users first.

A single Windows program. No installer, no runtime to install, no Node.js, and
no administrator rights.

**[Read the NoteFetch guide][guide]** online: the same guide, with tutorials,
that `F1` opens in the program.

[guide]: https://ldefontenay.github.io/notefetch/userguide/

## Download

Two files, always the newest version, direct. They contain exactly the same
program. Take the zip if you use Microsoft Edge, and the exe otherwise.

**[Download notefetch.zip][zip]**: for Edge, or any browser that objects to a
downloaded program.

**[Download notefetch.exe][latest]**: the program on its own, nothing to unzip.

[zip]: https://github.com/ldefontenay/notefetch/releases/latest/download/notefetch.zip
[latest]: https://github.com/ldefontenay/notefetch/releases/latest/download/notefetch.exe

The zip exists because Edge will not keep a downloaded unsigned program unless
you open its downloads flyout, find *Keep*, and confirm a second time: fiddly
with a screen reader, and alarming if you have not met it before. Edge has no
such objection to a zip. The zip holds one file and nothing else, so extracting
it gives you `notefetch.exe`.

That is the whole program. Put it in a folder of its own and run it. Its
settings are kept beside it.

The [releases page][releases] has both files with their release notes, and
every earlier version. Please read [Antivirus and SmartScreen](#antivirus-and-smartscreen)
below before the first run: the program is unsigned, and Windows will say so
once, whichever file you took.

[releases]: https://github.com/ldefontenay/notefetch/releases

Windows 10 or 11, 64-bit. A Plaud account; the free Starter plan works.

## The problem

A Plaud recorder gives you the audio of a lecture or a meeting, a transcript
and an AI summary. All of it lives in Plaud's app and website, which are not
always easy to use with a screen reader, and getting recordings out one at a
time, to keep them, search them or upload them elsewhere, is slow.

NoteFetch fetches everything in one go into ordinary folders: one folder per
recording, named after it, holding the audio, the transcript and the summary as
ordinary files. Next time it fetches only what is new, and anything Plaud has
finished since, such as a summary that was not ready yet.

It talks to Plaud's own hosted MCP server, which Plaud offers to any app, with
your permission given in your browser. It only reads: nothing in your Plaud
account is ever changed or deleted.

## Using it

Run `notefetch.exe`. The first time, it offers to sign you in to Plaud in your
browser, then asks how much of your history to download, saying how many
recordings and hours each choice means, so a long history never arrives by
surprise.

After that it syncs when it starts. The main window is a list of your
recordings, newest first, with each one's folder, date, length and status in
words. In the list:

- `Enter` plays a recording, or downloads one not yet downloaded.
- `Ctrl+M` opens the summary and `Ctrl+T` the transcript.
- `F2` renames a recording: its folder and every file in it, all or nothing.
- `Ctrl+Shift+M` moves recordings into a folder, such as one per course. The
  folders are real folders, so File Explorer shows exactly what NoteFetch does.
- `Delete` sends a recording to the Recycle Bin, and NoteFetch remembers not
  to download it again.
- The Applications key opens a menu of everything else.

`Ctrl+S` syncs, `Escape` cancels whatever is running, and `F1` opens the full
guide, with tutorials, in your browser. The guide is also
[online][guide].

## Accessibility

This is the point of the project, not a feature of it. NoteFetch was built with
and for a JAWS user, and every window has been tested by one, and checked with
NVDA.

- **Real Win32 controls** from resource templates: a genuine list view, combo
  boxes, edit boxes and buttons that a screen reader already knows. No
  drawn-UI toolkit is used, and none will be.
- **Every control has a label and an Alt key**, and the tab order follows the
  window from top to bottom.
- **Nothing is carried by colour or icons.** A recording's status is a word:
  Downloaded, No summary, Deleted, and so on.
- **NoteFetch speaks without moving your focus**, through UI Automation
  notifications, so you can carry on in the list while it works. Progress is
  spoken at most every few seconds, and there are three levels, down to off.
- **Everything spoken is also in the Log**, a box you can read back with the
  arrow keys, newest first, so nothing is heard once and then lost.
- **Four small sounds answer what you did**: done, finished, refused, and a
  question for you. Work NoteFetch does by itself is silent.
- **Names are built to be heard.** A folder is "Memory, week 4, 22 Sep 2026,
  1000": the name first for first-letter navigation, the date with the month in
  letters so it is not read as a string of numbers. Every file inside starts
  with what it is: "Transcript, Memory, week 4, ...".
- **Message boxes only for questions** you must answer. Everything routine goes
  to the Log.

## Antivirus and SmartScreen

NoteFetch is not signed with a code-signing certificate. The first time you run
it, Windows SmartScreen may show **Windows protected your PC**; *More info* then
*Run anyway* gets past it, once per machine.

Defender may then say it is running a cloud scan, which can take about ten
seconds, and nothing announces the end of it. NoteFetch's window opens when it
has passed.

Microsoft Edge objects earlier, at the download itself, and will not keep an
unsigned program without the two-step *Keep* described under
[Download](#download). The [zip][zip] avoids that. It does not avoid the
SmartScreen prompt: Windows carries the mark through extraction, so that one
still happens once.

Download NoteFetch only from [this repository's releases page][releases].

## Code signing policy

NoteFetch releases are not yet signed. NoteFetch is applying to
[SignPath Foundation](https://signpath.org) for free code signing for
open-source projects. If it is accepted, this section will read: free code
signing provided by [SignPath.io](https://about.signpath.io), certificate by
[SignPath Foundation](https://signpath.org).

How releases are made, and will be signed:

- Every release is built by the [build workflow](.github/workflows/build.yml)
  on GitHub's own Windows runners, from the tagged source in this repository.
  Nothing is built or signed on a personal computer.
- Only `notefetch.exe`, built from this repository, is signed. No third-party
  program is signed.
- Each signing request is approved by hand before it is signed.

Team roles:

- Committer and reviewer: [Laurent Cadet de Fontenay](https://github.com/ldefontenay)
- Approver: [Laurent Cadet de Fontenay](https://github.com/ldefontenay)

Privacy: this program will not transfer any information to other networked
systems unless specifically requested by the user or the person installing or
operating it. NoteFetch talks only to Plaud, to fetch your own recordings, when
you sign in or sync. See [Privacy](#privacy).

## What has actually been tested

- **Screen reader:** JAWS on Windows 11, through ten rounds of testing by its
  developer, who is blind: every window, menu, key, announcement and sound.
- **Plaud:** one account on the free Starter plan, with about 80 recordings
  from a Plaud Note Pro.
- **Uploading the audio** (Ogg Opus, `.ogg`) to Heidi Health worked.
- **NVDA** (2026.2): every announcement, the list, menus, the rename box, About
  and the guide, in two rounds.
- **Narrator:** announcements are spoken, in a brief check only.
- **Not yet tested:** Windows 10, and an account with more than 500
  recordings.

## Files

Only `notefetch.exe` is distributed. The dialogs, the guide and the sounds are
compiled into it. What NoteFetch writes:

- Your recordings, in the output folder: `Documents\NoteFetch` at first,
  changeable in Settings.
- `notefetch.conf`: the settings, beside the exe, or in `%APPDATA%\NoteFetch` if
  that folder cannot be written to. Safe to edit by hand.
- `auth.bin`: your Plaud sign-in, encrypted with Windows DPAPI for your Windows
  account, in `%APPDATA%\NoteFetch`. Signing out deletes it.
- `notefetch.db`: the list of recordings and what has been fetched of each, in
  `%LOCALAPPDATA%\NoteFetch`.
- `notefetch.log`: a copy of the Log, only if you turn it on. It never holds
  your sign-in, the words of a transcript or summary, or a download link.
- `notefetch-help.html`: the guide, written out each time you press `F1`, so it
  can never be older than the program, in `%LOCALAPPDATA%\NoteFetch`.

Deleting the exe, `notefetch.conf` and the two NoteFetch folders removes
NoteFetch completely. Your recordings stay until you delete them.

## Privacy

No telemetry. NoteFetch talks only to Plaud's server, its sign-in service, and
the storage serving your audio.

## Building

Rust stable-msvc and the Visual Studio Build Tools with the C++ workload.

```
cargo build --release
```

`build.rs` runs `rc.exe` over `notefetch.rc`, which compiles the dialog
templates, menu, accelerators, version resource and application manifest into
the binary.

```
cargo test                                   # unit tests
cargo clippy --all-targets -- -D warnings    # kept clean
cargo test -- --ignored                      # DPAPI and the Recycle Bin, on the real machine
```

`PLAN.md` is the design record: what was decided, on what evidence, and the
tester's own words for each decision. Read it before changing anything.
`SPEC.md` says what NoteFetch does, and the `test-milestone-*.txt` files are the
raw results of each round of testing with JAWS.

## Known issues

- **Plaud's sign-in lapses after a couple of weeks unused.** NoteFetch then
  says so and offers to sign in again in the browser.
- **Renaming in NoteFetch does not rename in Plaud.** Plaud's server is
  read-only. NoteFetch keeps your name and never overwrites it.
- **During an in-place rename**, JAWS reads the rest of the list row after the
  edit box.
- **After Ctrl+A**, "All N recordings selected" is spoken only after JAWS has
  read the name of every recording selected.

## Credits

The sign-in and MCP client were adapted from
[plaud-cli](https://github.com/leegonzales/plaud-cli) by Lee Gonzales, under
the MIT licence. Its notice is kept in the files that use its code.

Plaud is a trademark of its owner. NoteFetch is an independent program, not
made, endorsed or supported by Plaud.

## Licence

MIT. See [LICENSE](LICENSE).
