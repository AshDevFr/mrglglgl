use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use mrglglgl_core::discovery::{
    self, CharacterSource, DiscoveryOverrides, RootSource, WowRoot, account_settings_path,
    characters, discover_from, flavors, normalize_root, roots_from_battlenet_config,
    roots_from_registry,
};

fn touch(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// A WoW install laid out like a real one: three flavors, Forever's numeric realm folder and
/// `Firstname-Surname` character folders, an account-level SavedVariables folder, a character
/// without the addon, and folders that are not flavors.
fn install() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("World of Warcraft");
    let journal = "\nMrglglglCharDB = {\n[\"schema\"] = 1,\n}\n";
    let forever = root.join("_forever_/WTF/Account/ACCOUNT1");
    touch(
        &forever.join("93/Wren-Hollowbrook/SavedVariables/Mrglglgl.lua"),
        journal,
    );
    touch(
        &forever.join("93/Tamsin-Reedwater/SavedVariables/Mrglglgl.lua"),
        journal,
    );
    touch(
        &forever.join("93/Brin-Stonehoof/SavedVariables/Questie.lua"),
        "Q = {}",
    );
    touch(
        &forever.join("SavedVariables/Mrglglgl.lua"),
        "\nMrglglglDB = {\n}\n",
    );
    touch(
        &root.join("_classic_era_/WTF/Account/ACCOUNT1/Old Realm/Wren/SavedVariables/Mrglglgl.lua"),
        journal,
    );
    touch(
        &root.join("_retail_/WTF/Account/OTHER/Some Realm/Hero/SavedVariables/Mrglglgl.lua"),
        journal,
    );
    fs::create_dir_all(root.join("_no_wtf_here_/Interface")).unwrap();
    fs::create_dir_all(root.join("Data")).unwrap();
    fs::create_dir_all(root.join("Utils")).unwrap();
    dir
}

fn root_of(dir: &tempfile::TempDir) -> PathBuf {
    dir.path().join("World of Warcraft")
}

fn all_characters(root: &WowRoot) -> Vec<CharacterSource> {
    flavors(root).iter().flat_map(characters).collect()
}

#[test]
fn scans_every_flavor_folder_without_knowing_their_names() {
    let dir = install();
    let root = WowRoot {
        path: root_of(&dir),
        source: RootSource::DefaultPath,
    };
    let names: Vec<_> = flavors(&root).into_iter().map(|f| f.name).collect();
    assert_eq!(names, ["_classic_era_", "_forever_", "_retail_"]);

    let found: Vec<_> = all_characters(&root)
        .into_iter()
        .map(|c| {
            format!(
                "{}/{}/{}/{}",
                c.flavor, c.account, c.realm_folder, c.character_folder
            )
        })
        .collect();
    assert_eq!(
        found,
        [
            "_classic_era_/ACCOUNT1/Old Realm/Wren",
            "_forever_/ACCOUNT1/93/Tamsin-Reedwater",
            "_forever_/ACCOUNT1/93/Wren-Hollowbrook",
            "_retail_/OTHER/Some Realm/Hero",
        ]
    );
}

#[test]
fn points_each_character_at_its_journal_file() {
    let dir = install();
    let root = WowRoot {
        path: root_of(&dir),
        source: RootSource::DefaultPath,
    };
    for character in all_characters(&root) {
        assert!(character.journal_path.is_file());
        assert!(
            character
                .journal_path
                .ends_with("SavedVariables/Mrglglgl.lua")
        );
    }
}

#[test]
fn finds_the_account_settings_file() {
    let dir = install();
    let root = WowRoot {
        path: root_of(&dir),
        source: RootSource::DefaultPath,
    };
    let forever = flavors(&root)
        .into_iter()
        .find(|f| f.name == "_forever_")
        .unwrap();
    assert!(account_settings_path(&forever, "ACCOUNT1").is_some());
    assert!(account_settings_path(&forever, "NOPE").is_none());
}

#[test]
fn accepts_the_game_folder_or_a_flavor_folder_and_nothing_else() {
    let dir = install();
    let root = root_of(&dir);
    assert_eq!(normalize_root(&root), Some(root.clone()));
    assert_eq!(normalize_root(&root.join("_forever_")), Some(root.clone()));
    assert_eq!(normalize_root(&root.join("Data")), None);
    assert_eq!(normalize_root(&dir.path().join("missing")), None);
}

#[test]
fn an_override_always_wins() {
    let dir = install();
    let other = install();
    let found = discover_from(
        &DiscoveryOverrides {
            wow_root: Some(root_of(&dir).join("_forever_")),
        },
        vec![(root_of(&other), RootSource::DefaultPath)],
    );
    assert_eq!(
        found,
        [WowRoot {
            path: root_of(&dir),
            source: RootSource::Override
        }]
    );

    let none = discover_from(
        &DiscoveryOverrides {
            wow_root: Some(dir.path().join("not a wow folder")),
        },
        vec![(root_of(&other), RootSource::DefaultPath)],
    );
    assert!(
        none.is_empty(),
        "a bad override is reported, not silently replaced"
    );
}

#[test]
fn keeps_real_candidates_once_each_in_order() {
    let dir = install();
    let found = discover_from(
        &DiscoveryOverrides::default(),
        vec![
            (dir.path().join("nope"), RootSource::Registry),
            (root_of(&dir).join("_retail_"), RootSource::Registry),
            (root_of(&dir), RootSource::DefaultPath),
        ],
    );
    assert_eq!(
        found,
        [WowRoot {
            path: root_of(&dir),
            source: RootSource::Registry
        }]
    );
}

#[test]
fn reads_install_paths_from_a_fake_registry() {
    let registry = BTreeMap::from([(
        (
            r"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Blizzard Entertainment\World of Warcraft",
            "InstallPath",
        ),
        r"D:\Games\World of Warcraft\_retail_\".to_string(),
    )]);
    let found = roots_from_registry(|key, value| registry.get(&(key, value)).cloned());
    assert_eq!(
        found,
        [PathBuf::from(r"D:\Games\World of Warcraft\_retail_")]
    );
}

#[test]
fn reads_paths_from_a_battlenet_config_whatever_its_layout() {
    let config = r#"{ "Client": { "Install": { "DefaultInstallPath": "D:/Games" } },
                      "Games": { "wow": { "Path": "E:/WoW Forever" } }, "Volume": 3 }"#;
    let found = roots_from_battlenet_config(config);
    assert!(found.contains(&PathBuf::from("D:/Games/World of Warcraft")));
    assert!(found.contains(&PathBuf::from("E:/WoW Forever")));
    assert!(roots_from_battlenet_config("not json").is_empty());
}

fn snapshot(path: &Path) -> Vec<(PathBuf, u64, Option<SystemTime>)> {
    let mut out = Vec::new();
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap().flatten() {
            let meta = entry.metadata().unwrap();
            if meta.is_dir() {
                stack.push(entry.path());
            }
            out.push((entry.path(), meta.len(), meta.modified().ok()));
        }
    }
    out.sort();
    out
}

#[test]
fn reading_an_install_changes_nothing_in_it() {
    let dir = install();
    let before = snapshot(dir.path());
    let roots = discover_from(
        &DiscoveryOverrides::default(),
        vec![(root_of(&dir), RootSource::DefaultPath)],
    );
    for character in all_characters(&roots[0]) {
        let text = fs::read_to_string(&character.journal_path).unwrap();
        let _ = mrglglgl_core::schema::journal_from_text(&text);
    }
    assert_eq!(snapshot(dir.path()), before);
}

/// The owner's real install, when present: found with no settings. Prints and asserts no
/// character or account names, since those are real people's.
#[test]
fn finds_the_local_macos_install_with_no_settings() {
    let local = Path::new("/Applications/World of Warcraft/_forever_");
    if !cfg!(target_os = "macos") || !local.is_dir() {
        eprintln!("no local Forever install; skipped");
        return;
    }
    let roots = discovery::discover(&DiscoveryOverrides::default());
    let root = roots
        .iter()
        .find(|r| r.path == Path::new("/Applications/World of Warcraft"))
        .expect("the default macOS install");
    assert_eq!(root.source, RootSource::DefaultPath);
    let forever = flavors(root).into_iter().find(|f| f.name == "_forever_");
    assert!(forever.is_some(), "the _forever_ flavor");
    for character in characters(&forever.unwrap()) {
        assert!(character.journal_path.is_file());
    }
}
