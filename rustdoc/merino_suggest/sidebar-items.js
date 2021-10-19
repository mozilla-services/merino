initSidebarItems({"constant":[["FIREFOX_TEST_VERSIONS","The range of major Firefox version numbers to use for testing."]],"enum":[["CacheStatus","The relation between an object and a cache."],["LanguageIdentifier","An enum used to signify whether a `Language` refers to a specific language or a wildcard."],["SetupError","Errors that may occur while setting up the provider."],["SuggestError","Errors that may occur while querying for suggestions."]],"fn":[["fake_example_url","Helper to generate a URL to use for testing, of the form “https://example.com/fake#some-random-words”."]],"mod":[["debug","A suggestion provider that provides debug responses."],["device_info","Data structures that model information about a requester’s device. Device form factor, operating system, and browser are captured."],["domain","Datatypes to better represent the domain of Merino."],["fixed","A suggestion provider that provides a fixed response with a customizable title."],["id_multi","Provides a provider-combinator that contains a set of named providers. It can list these providers by name, and serve suggestions using only a partial set of them."],["keyword_filter","A suggestion provider that filters suggestions from a subprovider."],["multi","Provides a provider-combinator that provides suggestions from multiple sub-providers."],["timeout","Tools to make sure providers don’t  cache_status: todo!(), cache_ttl: todo!(), suggestions: todo!() take excessive amounts of time."],["wikifruit","A suggestion provider that provides toy responses."]],"struct":[["DebugProvider","A toy suggester to test the system."],["FixedProvider","A suggester that always provides the same suggestion, with a configurable title."],["IdMulti","A provider that aggregates suggestions from suggesters that tracks an ID per suggester (or suggester tree)."],["KeywordFilterProvider","A combinator provider that filters the results from the wrapped provider using a blocklist from the settings."],["Language","A representation of a language, as given in the Accept-Language HTTP header."],["Multi","A provider that aggregates suggestions from multiple suggesters."],["NullProvider","A provider that never provides any suggestions"],["Proportion","Represents a value from 0.0 to 1.0, inclusive. That is: a portion of something that cannot be negative or exceed 100%."],["ProviderDetails","Metadata about a provider contained in [`NamedMulti`];"],["Suggestion","A suggestion to provide to a user."],["SuggestionRequest","A request for suggestions."],["SuggestionResponse","A response of suggestions, along with related metadata."],["SupportedLanguages","Languages supported by the client."],["TimeoutProvider","A combinator provider that returns an empty set of suggestions if the wrapped provider takes too long."],["WikiFruit","A toy suggester to test the system."]],"trait":[["SuggestionProvider","A backend that can provide suggestions for queries."]]});