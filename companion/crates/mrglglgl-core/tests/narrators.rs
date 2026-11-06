use mrglglgl_core::narrators::{
    self, DEFAULT_NARRATOR, Invalid, Library, MAX_FIELD_CHARS, NarratorError, Problem,
};

fn dwarf_copy() -> narrators::Narrator {
    narrators::copy_of("grumpy_dwarf", "en")
}

#[test]
fn a_copy_of_a_built_in_has_every_field_and_no_id_yet() {
    let copy = dwarf_copy();
    let original = narrators::get("grumpy_dwarf", "en");
    assert_eq!(copy.name, "The grumpy dwarf (mine)");
    assert!(copy.id.is_empty(), "an id is given when it is saved");
    assert_eq!(copy.persona, original.persona);
    assert_eq!(copy.example, original.example);
    assert_eq!(copy.hints, original.hints);
    assert!(!copy.opt_in);
}

#[test]
fn a_saved_narrator_is_listed_and_used_in_any_prose_language() {
    let dir = tempfile::tempdir().unwrap();
    let mut copy = dwarf_copy();
    copy.voice = "Kind and slow, like moss.".into();
    let saved = narrators::save(dir.path(), copy).unwrap();
    assert_eq!(saved.id, "custom_the_grumpy_dwarf_mine");
    assert!(
        dir.path()
            .join("custom_the_grumpy_dwarf_mine.toml")
            .is_file()
    );

    let library = Library::load(dir.path());
    let listed = library.list();
    let mine = listed.iter().find(|n| n.id == saved.id).unwrap();
    assert!(mine.custom);
    assert!(
        listed.iter().filter(|n| !n.custom).count() >= 5,
        "built-ins stay"
    );
    assert_eq!(
        library.get(&saved.id, "fr").voice,
        "Kind and slow, like moss."
    );

    // Saving it again keeps its id and its file.
    let mut again = saved.clone();
    again.voice = "Kinder still.".into();
    assert_eq!(narrators::save(dir.path(), again).unwrap().id, saved.id);
    assert_eq!(
        Library::load(dir.path()).get(&saved.id, "en").voice,
        "Kinder still."
    );
}

#[test]
fn built_ins_cannot_be_overwritten_and_names_that_clash_get_their_own_id() {
    let dir = tempfile::tempdir().unwrap();
    let first = narrators::save(dir.path(), dwarf_copy()).unwrap();
    let second = narrators::save(dir.path(), dwarf_copy()).unwrap();
    assert_eq!(second.id, format!("{}_2", first.id));

    let mut built_in = narrators::get("grumpy_dwarf", "en").clone();
    built_in.voice = "Changed.".into();
    let saved = narrators::save(dir.path(), built_in).unwrap();
    assert!(
        saved.id.starts_with("custom_"),
        "a built-in id becomes a copy"
    );
    assert_ne!(
        Library::load(dir.path()).get("grumpy_dwarf", "en").voice,
        "Changed."
    );
}

#[test]
fn an_invalid_profile_is_refused_naming_each_field() {
    let dir = tempfile::tempdir().unwrap();
    let mut bad = dwarf_copy();
    bad.persona = "  ".into();
    bad.voice = "x".repeat(MAX_FIELD_CHARS + 1);
    bad.example.margin_note = "Mrgl, no star.".into();
    bad.hints.insert("flying".into(), "Wings.".into());
    let problems = narrators::validate(&bad);
    let has = |field: &str, problem: Problem| {
        problems.contains(&Invalid {
            field: field.into(),
            problem,
        })
    };
    assert!(has("persona", Problem::Missing));
    assert!(has("voice", Problem::TooLong));
    assert!(has("example.margin_note", Problem::NoStar));
    assert!(has("hints.flying", Problem::UnknownHint));
    match narrators::save(dir.path(), bad) {
        Err(NarratorError::Invalid(list)) => assert_eq!(list, problems),
        other => panic!("expected invalid, got {other:?}"),
    }
    assert!(Library::load(dir.path()).list().iter().all(|n| !n.custom));
    assert!(narrators::validate(&dwarf_copy()).is_empty());
}

#[test]
fn import_and_export_share_a_narrator_as_one_file() {
    let dir = tempfile::tempdir().unwrap();
    let saved = narrators::save(dir.path(), dwarf_copy()).unwrap();
    let text = narrators::to_toml(&saved);
    assert_eq!(
        narrators::parse(&text).unwrap(),
        saved,
        "the file reads back the same"
    );

    // Importing a file whose id is taken gives it a fresh one; the name stays.
    let imported = narrators::import(dir.path(), &text).unwrap();
    assert_ne!(imported.id, saved.id);
    assert!(imported.id.starts_with("custom_"));
    assert_eq!(imported.name, saved.name);

    // A built-in's own file imports as a copy.
    let built_in = narrators::to_toml(narrators::get("pompous_noble", "en"));
    assert!(
        narrators::import(dir.path(), &built_in)
            .unwrap()
            .id
            .starts_with("custom_")
    );

    assert!(matches!(
        narrators::import(dir.path(), "not = [a narrator"),
        Err(NarratorError::Unreadable(_))
    ));
}

#[test]
fn a_deleted_narrator_falls_back_to_the_scribe() {
    let dir = tempfile::tempdir().unwrap();
    let saved = narrators::save(dir.path(), dwarf_copy()).unwrap();
    narrators::delete(dir.path(), &saved.id).unwrap();
    let library = Library::load(dir.path());
    assert!(library.list().iter().all(|n| n.id != saved.id));
    assert_eq!(library.get(&saved.id, "en").id, DEFAULT_NARRATOR);
    assert!(
        matches!(
            narrators::delete(dir.path(), "grumpy_dwarf"),
            Err(NarratorError::BuiltIn)
        ),
        "a built-in is never deleted"
    );
}

#[test]
fn a_missing_folder_or_a_broken_file_leaves_the_built_ins() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        Library::load(&dir.path().join("nowhere")).list(),
        narrators::list()
    );
    std::fs::write(dir.path().join("custom_broken.toml"), "garbage = [").unwrap();
    assert_eq!(Library::load(dir.path()).list(), narrators::list());
}
