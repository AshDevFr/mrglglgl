//! The app's data layer: finding World of Warcraft installs, reading the SavedVariables the
//! Mrglglgl addon writes, and keeping the journal archive. It never executes Lua, and never writes
//! under the WoW folder except for one exception, in `addon_install`: installing, updating or
//! removing the addon's own folder under `Interface/AddOns`, never while the game runs.

pub mod addon_install;
pub mod addon_status;
pub mod archive;
pub mod beats;
mod beats_fr;
pub mod continuity;
pub mod db;
pub mod discovery;
pub mod export;
pub mod factcheck;
pub mod game;
pub mod milestones;
pub mod narrators;
pub mod prompt;
pub mod queue;
pub mod review;
pub mod schema;
pub mod screenshots;
pub mod site;
pub mod stories;
pub mod sv;
pub mod usage;
pub mod watch;
pub mod writer;
