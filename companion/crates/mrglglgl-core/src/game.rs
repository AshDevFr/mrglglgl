//! Recognising the game among running processes, so background writing can wait until the
//! player stops playing: local inference takes the CPU the game needs.
//!
//! The executable names are `Wow.exe`, `WowClassic.exe` and their test-realm variants on
//! Windows (unverified on real hardware), and "World of Warcraft" or "World of Warcraft Classic"
//! on macOS. Under Wine the Windows names show up too.

/// Whether a process name is the game's.
pub fn is_game_process(name: &str) -> bool {
    let lower = name.to_lowercase();
    let stem = lower
        .trim_end_matches(".exe")
        .trim_end_matches("-64")
        .trim_end_matches("-arm64");
    if stem.starts_with("world of warcraft") {
        return true;
    }
    // "wow", "wowclassic", "wowclassict", "wowt", "wowb", and any flavor built the same way.
    stem.strip_prefix("wow")
        .is_some_and(|rest| rest.chars().all(|c| c.is_ascii_lowercase()) && rest.len() <= 12)
}
