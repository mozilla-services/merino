initSidebarItems({"constant":[["FIREFOX_TEST_VERSIONS","The range of major Firefox version numbers to use for testing."]],"enum":[["CacheStatus","The relation between an object and a cache."],["LanguageIdentifier","An enum used to signify whether a `Language` refers to a specific language or a wildcard."],["SetupError","Errors that may occur while setting up the provider."],["SuggestError","Errors that may occur while querying for suggestions."]],"fn":[["fake_example_url","Helper to generate a URL to use for testing, of the form “https://example.com/fake#some-random-words”."]],"mod":[["debug","A suggestion provider that provides debug responses."],["device_info","Data structures that model information about a requester’s device. Device form factor, operating system, and browser are captured."],["domain","Datatypes to better represent the domain of Merino."],["multi","Provides a provider-combinator that provides suggestions from multiple sub-providers."],["timeout","Tools to make sure providers don’t  cache_status: todo!(), cache_ttl: todo!(), suggestions: todo!() take excessive amounts of time."],["wikifruit","A suggestion provider that provides toy responses."]],"struct":[["DebugProvider","A toy suggester to test the system."],["Language","A representation of a language, as given in the Accept-Language HTTP header."],["Multi","Type alias for the contained suggestion type to save some typing. A provider that aggregates suggestions from multiple suggesters."],["NullProvider","A provider that never provides any suggestions"],["Proportion","Represents a value from 0.0 to 1.0, inclusive. That is: a portion of something that cannot be negative or exceed 100%."],["Suggestion","A suggestion to provide to a user."],["SuggestionRequest","A request for suggestions."],["SuggestionResponse","A response of suggestions, along with related metadata."],["SupportedLanguages","Languages supported by the client."],["TimeoutProvider","A combinator provider that returns an empty set of suggestions if the wrapped provider takes too long."],["WikiFruit","A toy suggester to test the system."]],"trait":[["SuggestionProvider","A backend that can provide suggestions for queries."]]});