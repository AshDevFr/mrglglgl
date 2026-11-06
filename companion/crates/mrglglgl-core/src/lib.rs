//! The app's data layer: finding World of Warcraft installs, reading the SavedVariables the
//! Mrglglgl addon writes, and keeping the journal archive. It never writes under the WoW folder
//! and never executes Lua.

pub mod db;
pub mod discovery;
pub mod schema;
pub mod sv;
