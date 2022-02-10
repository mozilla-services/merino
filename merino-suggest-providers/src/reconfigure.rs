//! Helpers to reconfigure a provider and it's inner providers.

use cadence::StatsdClient;
use merino_settings::{Settings, SuggestionProviderConfig};
use merino_suggest_traits::{MakeFreshType, SetupError, SuggestionProvider};

use crate::make_provider_tree;

/// Reconfigure providers in place.
/// # Errors
/// If there is an unrecoverable error processing the new config.
pub async fn reconfigure_provider_tree(
    provider: &mut dyn SuggestionProvider,
    new_settings: Settings,
    new_config: serde_json::Value,
    metrics_client: StatsdClient,
) -> Result<(), SetupError> {
    let make_fresh: MakeFreshType = Box::new(move |fresh_config: SuggestionProviderConfig| {
        make_provider_tree(new_settings.clone(), fresh_config, metrics_client.clone())
    });
    provider.reconfigure(new_config, &make_fresh).await?;
    Ok(())
}
