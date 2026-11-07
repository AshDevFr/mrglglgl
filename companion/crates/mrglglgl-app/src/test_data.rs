//! Writes the data the frontend's tests render: the shared synthetic journal exactly as the
//! `character_journal` command would serve it, its milestones, and the milestone ids the
//! catalogues must name. Generated like the TypeScript bindings, so the frontend is tested
//! against what the backend really sends.

use std::path::PathBuf;

use mrglglgl_core::milestones::{all, compute};
use mrglglgl_core::schema;

fn ui_test_data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../ui/src/test/data")
}

fn write(name: &str, value: &impl serde::Serialize) {
    let dir = ui_test_data();
    std::fs::create_dir_all(&dir).unwrap();
    let json = serde_json::to_string_pretty(value).unwrap() + "\n";
    std::fs::write(dir.join(name), json).unwrap();
}

#[test]
fn write_frontend_test_data() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/savedvariables/synthetic-character.lua");
    let journal = schema::journal_from_text(&std::fs::read_to_string(fixture).unwrap()).unwrap();
    write("synthetic-journal.json", &journal);
    write("synthetic-awards.json", &compute(&journal));
    let ids: Vec<&str> = all().iter().map(|m| m.id.as_str()).collect();
    write("milestone-ids.json", &ids);
}
