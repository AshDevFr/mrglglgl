---
title: Reading your journal
description: Days, weeks, milestones, rewriting an entry and editing it by hand.
---

Each day is a page: the narrator's entry once it is written, the milestones stamped that day, any
screenshots the addon took, and **What happened**, the plain record the entry was written from (including money, the auction house, chat you chose to
record, and Legacy Challenges: see [What gets recorded](/guide/recorded/)).
Days are grouped by week; the week's chapter is written once the week is over, and follows the day
you are reading, below its page.

![A character's journal: the week's chapter and a written day with its milestone stamps.](../../../assets/screenshots/journal.png)

## Searching a journal

The field above the week finds words in a character's journal: the entries, the weeks'
chapters, and **What happened** on days the narrator has not written. Case and accents do not
matter. The days found replace the week, newest first, with the words marked; choosing one opens
that day. **⌘F** (or **Ctrl+F**) puts the cursor in the field and **Escape** empties it.

## When entries are written

A day the narrator has not written yet says so, with its place in line. The app writes finished
days one at a time in the background, oldest first. By default it waits while the game is
running, because writing uses the processor and the game may stutter. In **Settings**,
**AI**, turn on **Write while the game is running** to change that, or
**Pause background writing** to stop it.

A week's chapter is told from its days' entries, so it waits until each of its days is written,
or has given up after three tries. A day whose first entry arrives after its week's chapter was
told puts the chapter back in line, to be told again with that day in it; the earlier version stays
among the chapter's versions.

On a day still going, **Write what happened so far** writes it up to now; the full entry is written
once the day is over.

**Activity**, in the sidebar, spins and counts while the app is working. It opens a list of
everything under way, for every character: the entry being written and its words so far, the
entries waiting in the order they will be written, those that could not be written, a narrator
preview, a model download and a website build. From there you can stop what runs, try a failed
entry again, or pause and resume writing. A narrator preview waits for the entry being written,
then goes first, and the list says so.

## Writing again

Under a written entry, **Copy entry** and **Edit** are at hand; the rest is under **⋯**. Its
**Write again…** asks which narrator to use for that day, then writes the entry again; a write in
progress can be cancelled. A day with nothing written yet offers **Write now** straight away.
Earlier versions are kept: **Older** and **Newer** move between them.

## The narrator checks its own work

An entry that names a number, place, creature or item that is not in the day's notes is written
again once. If it is still wrong, the day keeps its plain notes and says why.

![One written day: the entry, the murloc's margin note, and the milestones stamped that day.](../../../assets/screenshots/entry.png)

## Editing an entry by hand

**Edit** on a finished day or week lets you write your own version, starting from the one shown
(a day set aside by the check included). It goes straight into the journal as a new version, the
earlier ones are kept, and its byline says you edited it: the app does not check your words
against the notes. The editor keeps the page's type, and a paragraph starting with `*` is the
murloc's margin note.

![Editing a day's entry by hand.](../../../assets/screenshots/edit.png)

## When an entry looks wrong

**Save a debug file…**, under **⋯** below the entry, writes the day into a folder you choose as a
JSON file:
what the addon recorded, the facts the narrator is told, the exact prompt the next write gets, and
every version written so far with its narrator, model and fact check. Attach it to a report on the
project page to show why an entry came out as it did. It holds the day's notes, so look through it
before sharing it.

## Milestones

Milestones are stamps for moments worth remembering: your first boot, your first murloc slain, a
day spent only fishing. They appear on the day they happened and together on the character's
**Milestones** tab.

![The Milestones tab: every stamp a character has earned.](../../../assets/screenshots/milestones.png)

There are 58, each with a rarity from one star to five. A day that earns several presses its three
rarest on its page and lists the others under **Also earned this day**, beside the entry;
hovering a stamp shows it large with what earned it. The narrator is told about the three rarest
only, so a big day is not a list. Every stamp, with what earns it, is in
[Every milestone](/reference/milestones/). If you turn a kind of data off for a character
(**Choose what is used**, on the character's **This character** tab, see [What gets recorded](/guide/recorded/#choosing-which-data-the-app-uses)), the
milestones it earned go with it.

## A day with nothing written

A day you did not play, or did not play with the addon, has nothing on its page yet:

![A day with nothing written: the murloc asleep on the book.](../../../assets/screenshots/empty-day.png)

## Reviewing before entries go in

Turn on **Let me review entries first** in **Settings**, **Writing** to have every new entry
wait as a draft, to accept or discard: the ones written in the background, and the ones you ask
for, from the journal or from **To review**. Off, every entry goes straight into the journal,
except a rewrite of a draft that is waiting: you asked for another version to review, so it waits
as a new draft. When a day's entry goes into the journal, older drafts of that day are set aside,
so an old draft can never replace a newer entry.

**To review**, in the sidebar, lists every character's finished days and weeks that have no entry
yet, and the drafts waiting, under each character's name: choose days across characters and
write, accept or discard them in one go. Each character's own **To review** tab shows the same
list for that character. When every finished day has its entry, the list says so:

![The To review tab with nothing waiting.](../../../assets/screenshots/review-empty.png)
