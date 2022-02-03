//! Helpers to reconfigure a provider and it's inner providers.

use cadence::StatsdClient;
use merino_settings::{Settings, SuggestionProviderConfig};
use merino_suggest_traits::{MakeFreshType, SetupError, SuggestionProvider};

use crate::make_provider_tree;

/// Reconfigure providers in place.

pub async fn reconfigure_provider_tree(
    provider: &mut dyn SuggestionProvider,
    new_settings: Settings,
    new_config: serde_json::Value,
    metrics_client: StatsdClient,
) -> Result<(), SetupError> {
    // let make_fresh: Box<
    //     (dyn Fn(
    //         SuggestionProviderConfig,
    //     ) -> Pin<
    //         Box<(dyn Future<Output = Result<Box<dyn SuggestionProvider>, SetupError>> + 'static)>,
    //     > + 'static),
    // > = Box::new(move |fresh_config: SuggestionProviderConfig| {
    //     make_provider_tree(new_settings.clone(), fresh_config, metrics_client.clone())
    // });

    let make_fresh: MakeFreshType = Box::new(move |fresh_config: SuggestionProviderConfig| {
        make_provider_tree(new_settings.clone(), fresh_config, metrics_client.clone())
    });

    provider.reconfigure(new_config, make_fresh)
}
