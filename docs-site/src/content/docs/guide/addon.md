---
title: The addon
description: What the addon records, its settings in game, and managing it from the app.
---

The addon quietly records your character's day: sessions, levels, quests, zones and places
discovered, kills, deaths and what killed you, fishing (boots included), notable loot, the people
you grouped with, professions, money and the auction house, and the Legacy Challenges you
complete. Chat, for role play, can be recorded too but is off by default. It shows nothing unless
you ask, except one line the first time it records on your account, to say it is working and that
`/mrgl` shows what it noted. The newer kinds are described in [What gets recorded](/guide/recorded/).

## In game

- `/mrgl status` shows what has been recorded today.
- `/mrgl settings` opens its panel, also under Options, AddOns, Mrglglgl.

The settings are spread over pages under **Mrglglgl** in Options, AddOns:

- **Mrglglgl** (the main page):
  - **What gets recorded:** each category can be turned off. Quest descriptions are most of the
    journal's size; the names of people you group with are other people's names; **Chat** is off
    by default.
  - **Notable loot quality:** the lowest item quality recorded as notable loot.
  - **Storage**, **Keep full detail for:** 14, 30 (the default), 60 or 90 days. Older days keep
    their key moments and a daily summary; the app keeps the full history once it has seen it.
- **Chat:** under **Chat channels**, two switches per channel, "your lines" and "others' lines".
  See [What gets recorded](/guide/recorded/#chat-for-role-play).
- **Screenshots:** take a game screenshot on a level up, a death, a boss kill and a few firsts
  (your first boot, first murloc slain, first death by murloc), for the app to show with the day.
  Off by default.
- **Forget:** **Forget recorded data** deletes what this character has recorded in a category.
  Press **Forget** twice to confirm. Copies the app already has are not affected.

The addon speaks the language of your game client: English, or French on a French client. The
app's own language is chosen separately (see [Settings](/guide/settings/#language)).

## From the app

**Settings**, **Addon** shows, for each WoW Forever folder, whether the addon is installed, its
version, and whether it has been recording, with what to do about anything wrong.

![The Addon tab of Settings: installed, recording, and the newest release.](../../../assets/screenshots/settings-addon.png)

From there you can install the addon, update it, put back the previous version, or remove it.

- The app never changes the addon while the game is running.
- It writes only the addon's own folder, `Interface/AddOns/Mrglglgl`, and checks every download
  before anything is written.
- Removing the addon removes only its folder: everything it recorded stays, in the game's files
  and in the app. A copy is kept so you can put it back.
- A copy installed by an addon manager such as CurseForge, or by hand, is left alone unless you
  choose to replace it.
- **Offer test versions** also offers versions before they are released.
