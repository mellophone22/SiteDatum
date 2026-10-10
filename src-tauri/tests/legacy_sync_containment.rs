//! Regression checks for the disabled legacy Sync production boundary.
//!
//! These checks intentionally inspect the small command boundary so a future
//! refactor cannot quietly reconnect the legacy provider while MetadataSync is
//! still denied. Disconnect remains callable as a privacy cleanup action.

const LIB_SOURCE: &str = include_str!("../src/lib.rs");
const AUTH_SOURCE: &str = include_str!("../src/cloud_auth.rs");
const SYNC_SOURCE: &str = include_str!("../src/cloud_sync.rs");

fn function_body<'a>(source: &'a str, name: &str) -> &'a str {
    let marker = format!("fn {name}");
    let start = source.find(&marker).expect("named function exists");
    let remainder = &source[start..];
    let end = remainder
        .find("\n#[tauri::command]")
        .unwrap_or(remainder.len());
    &remainder[..end]
}

#[test]
fn every_legacy_network_or_data_command_uses_the_shared_kill_switch() {
    let shared_boundary = function_body(LIB_SOURCE, "require_legacy_cloud_sync");
    assert!(shared_boundary.contains("CommercialFeature::MetadataSync"));
    assert!(shared_boundary.contains("require_feature("));

    for command in [
        "sign_in_cloud_with_password",
        "sync_cloud_workspace",
        "list_cloud_conflicts",
        "resolve_cloud_conflict",
    ] {
        assert!(
            function_body(LIB_SOURCE, command).contains("require_legacy_cloud_sync(&state)?"),
            "{command} must enforce the shared legacy Sync boundary"
        );
    }

    let availability = function_body(LIB_SOURCE, "get_cloud_sync_availability");
    assert!(availability.contains("legacy_cloud_sync_available(&state)?"));

    let disconnect = function_body(LIB_SOURCE, "disconnect_cloud");
    assert!(disconnect.contains("cloud_auth::disconnect()"));
    assert!(disconnect.contains("disable_legacy_cloud_sync_access()"));
}

#[test]
fn legacy_release_sources_contain_no_live_provider_coordinates_or_raw_payload_logs() {
    const RETIRED_PROJECT_ID: &str = "jblmxjowguphehuozfsg";
    const RETIRED_PUBLISHABLE_FRAGMENT: &str = "2ZDEAuL6CSD6SK5omAi";

    for source in [AUTH_SOURCE, SYNC_SOURCE] {
        assert!(!source.contains(RETIRED_PROJECT_ID));
        assert!(!source.contains(RETIRED_PUBLISHABLE_FRAGMENT));
        assert!(!source.contains("response.text()"));
        assert!(source.contains("legacy-sync-disabled.invalid"));
    }
}

#[test]
fn dormant_sync_v2_modules_have_no_command_or_external_lifecycle_caller() {
    // Source-surface regression, not a general Rust call-graph proof. Paired with
    // the behavioral all-access-mode MetadataSync denial test in enforcement.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let lifecycle = [
        "KeyRing::initial",
        "KeyRing::load_protected",
        ".save_protected(",
        ".rotate_sync_v2_protected_keys(",
        ".create_sync_v2_recovery_file(",
        ".install_sync_v2_recovery_floor(",
        ".stage_sync_v2_rotation(",
        "open_recovery_file(",
        "write_recovery_file(",
        "RecoveryCode::generate",
    ];
    for entry in std::fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        let source = std::fs::read_to_string(&path)
            .unwrap()
            .replace("\r\n", "\n");
        let production = source.split("#[cfg(test)]\nmod tests").next().unwrap();
        let name = path.file_name().unwrap().to_str().unwrap();
        if name.starts_with("sync_v2_") {
            assert!(
                !production.contains("#[tauri::command]"),
                "{name} cannot expose Sync commands"
            );
        } else {
            for line in production.lines().filter(|line| {
                let line = line.trim_start();
                !line.starts_with("mod sync_v2_")
                    && !(line.starts_with("include_str!(\"../migrations/")
                        && line.ends_with(".sql\"),"))
            }) {
                assert!(
                    !line.contains("sync_v2_"),
                    "{name} cannot call dormant Sync modules"
                );
            }
        }
        if name != "sync_v2_key_recovery.rs" {
            for call in lifecycle {
                assert!(
                    !production.contains(call),
                    "{name} cannot call dormant key lifecycle"
                );
            }
        }
    }
}
