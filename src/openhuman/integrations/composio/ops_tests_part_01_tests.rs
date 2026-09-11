use std::collections::BTreeSet;

use super::*;
use crate::openhuman::integrations::composio::providers::{
    profile_md::merge_provider_into_profile_md, ProviderUserProfile,
};

#[tokio::test]
async fn composio_delete_connection_direct_refuses_before_accessing_local_state() {
    for mode in [COMPOSIO_MODE_DIRECT, "  direct\t"] {
        for api_key in [None, Some("ck_test_direct_key".to_string())] {
            for clear_memory in [false, true] {
                let tmp = tempfile::tempdir().unwrap();
                let mut config = Config::default();
                config.workspace_dir = tmp.path().join("workspace");
                config.config_path = tmp.path().join("config.toml");
                config.composio.mode = mode.to_string();
                config.composio.api_key = api_key.clone();
                config.save().await.unwrap();

                // No backend session or memory driver is installed. Refusal must
                // precede both toolkit lookup and memory target discovery.
                merge_provider_into_profile_md(
                    &config.workspace_dir,
                    &ProviderUserProfile {
                        toolkit: "gmail".to_string(),
                        connection_id: Some("c1".to_string()),
                        email: Some("user@example.com".to_string()),
                        ..Default::default()
                    },
                )
                .unwrap();
                let profile_path = config.workspace_dir.join("PROFILE.md");
                let profile = std::fs::read_to_string(&profile_path).unwrap();
                assert!(profile.contains("user@example.com"));
                let config_before = std::fs::read(&config.config_path).unwrap();
                let workspace_before: BTreeSet<_> = std::fs::read_dir(&config.workspace_dir)
                    .unwrap()
                    .map(|entry| entry.unwrap().file_name())
                    .collect();

                let error = composio_delete_connection(&config, "c1", clear_memory)
                    .await
                    .unwrap_err();

                assert!(error.starts_with("COMPOSIO_DIRECT_DISCONNECT_UNSUPPORTED:"));
                assert!(error.contains("your own Composio dashboard at app.composio.dev"));
                assert!(
                    error.contains("has not disconnected the account or cleared its synced memory")
                );
                assert!(!error.contains("DeleteConnection is not available over the direct route"));
                assert_eq!(std::fs::read_to_string(&profile_path).unwrap(), profile);
                assert_eq!(std::fs::read(&config.config_path).unwrap(), config_before);
                let workspace_after: BTreeSet<_> = std::fs::read_dir(&config.workspace_dir)
                    .unwrap()
                    .map(|entry| entry.unwrap().file_name())
                    .collect();
                assert_eq!(workspace_after, workspace_before);
            }
        }
    }
}

#[tokio::test]
async fn composio_delete_connection_managed_preserves_missing_session_error() {
    let tmp = tempfile::tempdir().unwrap();
    let mut config = Config::default();
    config.workspace_dir = tmp.path().join("workspace");
    config.config_path = tmp.path().join("config.toml");

    let error = composio_delete_connection(&config, "c1", false)
        .await
        .unwrap_err();

    assert!(error.contains("composio unavailable: no backend session token"));
    assert!(!error.contains("COMPOSIO_DIRECT_DISCONNECT_UNSUPPORTED"));
}
