//! The app's data layer: finding World of Warcraft installs, reading the SavedVariables the
//! Mrglglgl addon writes, and keeping the journal archive. It never writes under the WoW folder
//! and never executes Lua.

pub mod archive;
pub mod beats;
pub mod continuity;
pub mod db;
pub mod discovery;
pub mod factcheck;
pub mod milestones;
pub mod narrators;
pub mod prompt;
pub mod schema;
pub mod screenshots;
pub mod sv;
pub mod watch;
pub mod writer;
