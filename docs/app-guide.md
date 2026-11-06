# Using the Mrglglgl app

The app reads the journal the Mrglglgl addon keeps in game, keeps its full history, and has the
murloc scribe write each day and each week as a short journal entry.

## First run

1. Install the addon (Settings, The addon, **Install**, or from CurseForge, Wago or the
   releases page) and play at least once. The game saves the addon's journal when you log out
   or type `/reload`, and only then can the app see it.
2. Open the app. It looks for World of Warcraft in the usual places. If it does not find it, open
   Settings and enter the game folder (the one containing `_forever_`) under
   **World of Warcraft folder**.
3. Your characters appear in the sidebar, grouped by account and realm. Each shows when the game
   last saved its journal.
4. To have entries written, open Settings and choose who writes, under **Who writes**:
   - **This computer:** the app downloads a writing model once (a few gigabytes; the size is shown
     before anything downloads). Nothing you write leaves your computer. This needs at least 8 GB
     of memory; the app tells you if your computer has less and suggests a model that fits.
   - **An endpoint you run or pay for:** any OpenAI-compatible address, such as Ollama or
     LM Studio on your own machine, or a cloud provider. See Privacy below.

## Reading your journal

Each day is a page: what happened, written as plain sentences, the murloc's entry once it is
written, the milestones stamped that day, and any screenshots the addon took. Days are grouped by
week; the week's chapter is written once the week is over.

A day the murloc has not written yet says so, with its place in line. The app writes finished days
one at a time in the background, oldest first. By default it waits while the game is running,
because writing uses the processor and the game may stutter; turn on
**Write while the game is running** to change that, or **Pause background writing** to stop it.

On a day you can write the entry again, pick another narrator for that day, or cancel a write in
progress. Earlier versions are kept.

The murloc checks its own work: an entry that names a number, place, creature or item that is not in
the day's notes is written again once, and if it is still wrong, the day keeps its plain notes and
says why.

## Narrators and margin notes

**Narrator** chooses who writes your journal. The murloc scribe is the default; the others are
styles you can choose, and the murloc writing entirely in its own language is only used if you pick
it. **Murloc margin notes** sets how often the murloc adds a note in the margin, from never to about
one day in three. **Language of the journal** writes entries in English or French, independently of
the app's own language.

## Sharing

Copy an entry to paste it into Discord or a forum. The copy ends with "via Mrglglgl" unless you turn
that off in Settings.

Export a character's whole journal from its page: as one web page laid out like the journal
(stamps and captioned screenshots inside), as a PDF through the print dialog, or as Markdown.

To publish your journals, turn on Settings, Website: the app keeps a finished site up to date in a
folder (a post per week, an archive, milestones, a feed), which you put online with Cloudflare
Pages, Netlify or GitHub Pages. Other players appear under stand-in names unless you turn that off.
To change how the site looks, see [Theming the journal website](site-themes.md).

## Your data

The app writes nothing into the World of Warcraft folder except the addon itself, and only when
you ask it to install, update or remove it in Settings, The addon (never while the game runs).
Removing the addon removes only its folder: what it recorded stays in the game's files and in the
app, and a copy is kept so you can put it back. A copy installed by an addon manager is left
alone unless you choose to replace it. Each time the game saves a
journal, the app keeps a copy in its own data folder; **Keep backups for** chooses how long those
copies are kept (30 days, 90 days, 1 year or forever). Your journal's full history stays in the app
whatever you choose: backups are only a safety net.

Forgetting data in the game, with the addon's "Forget recorded" buttons, changes only the game's
files. To remove a character's history from the app, remove the character in the app.

## When something goes wrong

The app keeps a log of what it does, a file a day for the last week, in the `logs` folder of its
data folder. Settings, Log, **Show the log file** opens it in your file manager: attach it when you
ask for help. If some settings could not be read, Settings says which ones are back to their
defaults; the values it could not read are kept aside rather than overwritten.

## Privacy

With **This computer**, nothing leaves your computer.

With an endpoint, what is sent depends on where it is. An address on your own computer (such as
Ollama at `http://localhost:11434/v1`) stays on it. For any other address, the app shows this before
you use it, and it is exactly what happens:

> This address is not on this computer. When writing, the app sends that address: your character's name, the names of people you grouped with, and the text of quests you touched that day, along with what happened that day.

Turn on **Send stand-in names for the people you grouped with** to send invented names instead;
the real names are put back in the entry when it comes back. Your endpoint key, if it needs one, is
kept in your system's keychain, never in the app's files.
