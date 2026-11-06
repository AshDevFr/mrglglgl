use std::fs;
use std::path::Path;

use mrglglgl_core::addon_status::{
    AddonProblem, AddonState, CharacterRun, InstalledAddon, assess, client_version,
    display_version, installed, is_forever,
};
use mrglglgl_core::discovery::Flavor;
use mrglglgl_core::schema::{AddonErrors, AddonHeartbeat, SUPPORTED_SCHEMA};

const NOW: i64 = 1_794_000_000;
const DAY: i64 = 24 * 60 * 60;

fn touch(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn flavor(root: &Path, name: &str) -> Flavor {
    Flavor {
        name: name.to_string(),
        path: root.join(name),
    }
}

/// `.build.info` as the launcher writes it, trimmed to the columns that matter and two products.
const BUILD_INFO: &str = "Branch!STRING:0|Active!DEC:1|Build Key!HEX:16|Tags!STRING:0|Version!STRING:0|Product!STRING:0\n\
eu|1|9006ba|OSX x86_64 EU? enUS speech?|1.60.1.70790|wowf\n\
eu|1|1234ab|OSX x86_64 EU?|1.15.7.61000|wow_classic_era\n";

const RELEASED_TOC: &str = "## Interface: 16001\n## Title: Mrglglgl\n## Version: addon-v0.2.0\n\
## X-Mrglglgl-Schema: 1\n## SavedVariables: MrglglglDB\n\nCore.lua\n";

fn heartbeat(version: &str, interface: i64, login: i64) -> AddonHeartbeat {
    AddonHeartbeat {
        version: Some(version.to_string()),
        build: Some("70790".to_string()),
        interface: Some(interface),
        login: Some(login),
    }
}

fn released() -> InstalledAddon {
    InstalledAddon {
        version: Some("0.2.0".to_string()),
        interface: Some(16001),
        schema: Some(1),
        development: false,
    }
}

#[test]
fn versions_are_shown_without_the_tag_prefix_and_a_development_copy_has_none() {
    assert_eq!(display_version("addon-v0.1.0").as_deref(), Some("0.1.0"));
    assert_eq!(display_version("0.3.1").as_deref(), Some("0.3.1"));
    assert_eq!(display_version("@project-version@"), None);
    assert_eq!(display_version(" "), None);
}

#[test]
fn reads_the_installed_addon_from_its_toc() {
    let dir = tempfile::tempdir().unwrap();
    let forever = flavor(dir.path(), "_forever_");
    assert_eq!(installed(&forever), None);
    touch(
        &forever.path.join("Interface/AddOns/Mrglglgl/Mrglglgl.toc"),
        RELEASED_TOC,
    );
    assert_eq!(installed(&forever), Some(released()));
}

#[test]
fn an_unpackaged_or_linked_copy_is_a_development_copy() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("addon");
    touch(
        &source.join("Mrglglgl.toc"),
        "## Interface: 16001\n## Version: @project-version@\n",
    );
    let forever = flavor(dir.path(), "_forever_");
    let addons = forever.path.join("Interface/AddOns");
    fs::create_dir_all(&addons).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&source, addons.join("Mrglglgl")).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&source, addons.join("Mrglglgl")).unwrap();
    let found = installed(&forever).unwrap();
    assert!(found.development);
    assert_eq!(found.version, None);
    assert_eq!(found.schema, None);
}

#[test]
fn tells_forever_from_other_flavors_and_reads_its_client_version() {
    let dir = tempfile::tempdir().unwrap();
    let forever = flavor(dir.path(), "_forever_");
    let era = flavor(dir.path(), "_classic_era_");
    let renamed = flavor(dir.path(), "_mine_");
    touch(
        &forever.path.join(".flavor.info"),
        "Product Flavor!STRING:0\nwowf\n",
    );
    touch(
        &era.path.join(".flavor.info"),
        "Product Flavor!STRING:0\nwow_classic_era\n",
    );
    touch(
        &renamed.path.join(".flavor.info"),
        "Product Flavor!STRING:0\nwowf\n",
    );
    assert!(is_forever(&forever));
    assert!(!is_forever(&era));
    assert!(is_forever(&renamed));
    // Without the file, the usual folder name decides.
    let bare = dir.path().join("bare");
    assert!(is_forever(&flavor(&bare, "_forever_")));
    assert!(!is_forever(&flavor(&bare, "_classic_")));

    touch(&dir.path().join(".build.info"), BUILD_INFO);
    assert_eq!(
        client_version(dir.path(), "wowf").as_deref(),
        Some("1.60.1.70790")
    );
    assert_eq!(
        client_version(dir.path(), "wow_classic_era").as_deref(),
        Some("1.15.7.61000")
    );
    assert_eq!(client_version(dir.path(), "wow"), None);
}

#[test]
fn the_state_follows_what_is_installed_and_the_last_login() {
    let dir = tempfile::tempdir().unwrap();
    let forever = flavor(dir.path(), "_forever_");
    let recent = heartbeat("addon-v0.2.0", 16001, NOW - DAY);
    let old = heartbeat("addon-v0.2.0", 16001, NOW - 9 * DAY);
    let run = |heartbeat| CharacterRun {
        character: "Wren",
        heartbeat,
        errors: None,
    };

    let none = assess(&forever, None, None, &[run(Some(&recent))], NOW);
    assert_eq!(none.state, AddonState::NotInstalled);
    assert!(none.folder.ends_with("Interface/AddOns/Mrglglgl"));

    let waiting = assess(&forever, None, Some(released()), &[run(None)], NOW);
    assert_eq!(waiting.state, AddonState::Waiting);
    assert_eq!(waiting.last_run, None);

    let recording = assess(&forever, None, Some(released()), &[run(Some(&recent))], NOW);
    assert_eq!(recording.state, AddonState::Recording);
    let last = recording.last_run.unwrap();
    assert_eq!(
        (last.character.as_str(), last.version.as_deref(), last.login),
        ("Wren", Some("0.2.0"), NOW - DAY)
    );
    assert!(recording.problems.is_empty());

    let quiet = assess(&forever, None, Some(released()), &[run(Some(&old))], NOW);
    assert_eq!(quiet.state, AddonState::Quiet);
}

#[test]
fn the_latest_login_of_any_character_counts() {
    let dir = tempfile::tempdir().unwrap();
    let forever = flavor(dir.path(), "_forever_");
    let old = heartbeat("addon-v0.1.0", 16001, NOW - 20 * DAY);
    let recent = heartbeat("addon-v0.2.0", 16001, NOW - DAY);
    let status = assess(
        &forever,
        None,
        Some(released()),
        &[
            CharacterRun {
                character: "Wren",
                heartbeat: Some(&old),
                errors: None,
            },
            CharacterRun {
                character: "Tamsin",
                heartbeat: Some(&recent),
                errors: None,
            },
        ],
        NOW,
    );
    assert_eq!(status.state, AddonState::Recording);
    assert_eq!(status.last_run.unwrap().character, "Tamsin");
    // The old character's older version is not an update waiting to load.
    assert!(status.problems.is_empty());
}

#[test]
fn problems_each_have_their_own_kind() {
    let dir = tempfile::tempdir().unwrap();
    let forever = flavor(dir.path(), "_forever_");
    let ran_older = heartbeat("addon-v0.1.0", 16002, NOW - DAY);
    let errors = AddonErrors {
        count: 3,
        last_message: Some("Loot.lua:40: attempt to index a nil value".to_string()),
        last_event: Some("LOOT_OPENED".to_string()),
        last_time: Some(NOW - 2 * DAY),
    };
    let newer_schema = InstalledAddon {
        schema: Some(SUPPORTED_SCHEMA + 1),
        ..released()
    };
    let status = assess(
        &forever,
        None,
        Some(newer_schema),
        &[CharacterRun {
            character: "Wren",
            heartbeat: Some(&ran_older),
            errors: Some(&errors),
        }],
        NOW,
    );
    assert_eq!(
        status.problems,
        vec![
            AddonProblem::NewerSchema {
                schema: SUPPORTED_SCHEMA + 1,
                supported: SUPPORTED_SCHEMA,
            },
            AddonProblem::UpdatePending {
                running: "0.1.0".to_string(),
                installed: "0.2.0".to_string(),
            },
            AddonProblem::OutOfDate {
                addon: 16001,
                client: 16002,
            },
            AddonProblem::Errors {
                count: 3,
                message: Some("Loot.lua:40: attempt to index a nil value".to_string()),
                time: Some(NOW - 2 * DAY),
            },
        ]
    );
}

#[test]
fn old_errors_and_development_copies_raise_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let forever = flavor(dir.path(), "_forever_");
    let ran = heartbeat("@project-version@", 16001, NOW - DAY);
    let errors = AddonErrors {
        count: 1,
        last_message: Some("old".to_string()),
        last_event: None,
        last_time: Some(NOW - 30 * DAY),
    };
    let development = InstalledAddon {
        version: None,
        development: true,
        schema: None,
        interface: Some(16001),
    };
    let status = assess(
        &forever,
        None,
        Some(development),
        &[CharacterRun {
            character: "Wren",
            heartbeat: Some(&ran),
            errors: Some(&errors),
        }],
        NOW,
    );
    assert_eq!(status.state, AddonState::Recording);
    assert!(status.problems.is_empty());
}
