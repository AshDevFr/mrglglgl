---
title: Your data and privacy
description: What the app keeps, what it sends, and what to do when something goes wrong.
---

## Your data

The app writes nothing into the World of Warcraft folder except the addon itself, and only when
you ask it to install, update or remove it in **Settings**, **Addon** (never while the game runs).
Removing the addon removes only its folder: what it recorded stays in the game's files and in the
app, and a copy is kept so you can put it back. A copy installed by an addon manager is left
alone unless you choose to replace it.

Each time the game saves a journal, the app keeps a copy in its own data folder;
**Keep backups for** chooses how long those copies are kept (30 days, 90 days, 1 year or
forever). Your journal's full history stays in the app whatever you choose: backups are only a
safety net.

To keep a kind of data out of what narrators, the website and exports see (or out of the app's
view), without deleting it, use **Choose what is used** on the character's **This character** tab: see
[What gets recorded](/guide/recorded/#choosing-which-data-the-app-uses). Other players' chat lines
are never sent to a narrator, never go on the website and are never exported.

Forgetting data in the game, with the addon's "Forget recorded" buttons, changes only the game's
files. To remove a character's history from the app, remove the character in the app.

## Privacy

With **This computer**, nothing leaves your computer.

With an endpoint, what is sent depends on where it is. An address on your own computer (such as
Ollama at `http://localhost:11434/v1`) stays on it. For any other address, the app shows this before
you use it, and it is exactly what happens:

> This address is not on this computer. When writing, the app sends that address: your character's name, the names of people you grouped with, and the text of quests you touched that day, along with what happened that day.

Turn on **Send stand-in names for the people you grouped with** to send invented names instead;
the real names are put back in the entry when it comes back. Your endpoint key, if it needs one, is
kept in your system's keychain, never in the app's files.

## When something goes wrong

If macOS says the app "is damaged and can't be opened", see the tip in
[Getting started](/getting-started/#1-install-the-app).

The app keeps a log of what it does, a file a day for the last week, in the `logs` folder of its
data folder. Settings, Logs, **Show the log file** opens it in your file manager: attach it
when you ask for help. If some settings could not be read, Settings says which ones are back to their
defaults; the values it could not read are kept aside rather than overwritten.
