# Data collection

This page should list all metrics and logs that Merino is expected to emit in
production, including what should be done about them, if anything.

## Logs

This list does not include any `DEBUG` or `TRACE` level events, since those are
not logged by default in production. The events below are grouped by crate, and
the level and type of the log is listed.

Any log containing sensitive data must include a boolean field `sensitive`
that is set to `true` to exempt it from flowing to the generally accessible
log inspection interfaces.

### `merino-adm`

- `INFO adm.remote-settings.sync-start` - The Remote Settings provider has
  started syncing records.

- `WARN adm.remote-settings.empty` - After syncing no records were found in the
  Remote Settings collection.

### `merino-cache`

- `INFO cache.redis.save-error` - There was an error while saving a cached
  suggestion to the Redis server.

### `merino-web`

- `INFO web.suggest.request` - A suggestion request is being processed. This
  event will include fields for all relevant details of the request. **Fields:**

  - `sensitive` - Always set to true to ensure proper routing.
  - `query` - If query logging is enabled, the text the user typed. Otherwise an
    empty string.
  - `country` - The country the request came from.
  - `region` - The first country subdivision the request came from.
  - `city` - The city the request came from.
  - `dma` - A US-only location description that is larger than city and smaller
    than states, but does not align to political borders.
  - `agent` - The original user agent.
  - `os_family` - Parsed from the user agent. One of "windows", "macos",
    "linux", "ios", "android", "chrome os", "blackberry", or "other".
  - `form_factor` - Parsed from the user agent. One of "desktop", "phone",
    "tablet", or "other"
  - `browser` - The browser and possibly version detected. Either "Firefox(XX)"
    where XX is the version, or "Other".
  - `rid` - The request ID.
  - `accepts_english` - True if the user's Accept-Language header includes an
    English locale, false otherwise.
  - `requested_providers` - A comma separated list of providers requested via
    the query string, or an empty string if none were requested (in which case
    the default values would be used).
  - `client_variants` - Any client variants sent to Merino in the query string.

- `INFO web.configuring-suggesters` - A web worker is starting to configure
  local suggesters, which may take some seconds and require network traffic to
  synchronize data.

- `ERROR web.suggest.setup-error` - There was an error while setting up
  configuration providers. This may be temporary, and future requests will
  attempt to configure providers again.

- `ERROR web.suggest.error` - There was an error while providing suggestions
  from an otherwise set-up provider. This may represent a network or
  configuration error.

- `ERROR dockerflow.error_endpoint` - The `__error__` endpoint of the server was
  called. This is used to test our error reporting system. It is not a cause for
  concern, unless we receive a large amount of these records, in which case some
  outside service is likely malicious or misconfigured.

## Metrics

> A note on timers: Statsd timers are measured in milliseconds, and are reported
> as integers (at least in Cadence). Milliseconds are often not precise enough
> for the tasks we want to measure in Merino. Instead we use generic histograms
> to record microsecond times. Metrics recorded in this way should have `-us`
> appended to their name, to mark the units used (since we shouldn't put the
> proper unit μs in metric names).

- `startup` - A counter incremented at startup, right after metrics are
  initialized, to signal a successful metrics system initialization.

- `client_variants.<variant_name>` - A counter incremented for each client
  variant present in a query request, incremented when the response is assembled
  with the suggestions.

- `request.suggestion-per` - A histogram that reports the number of suggestions
  in a response for a given query.

- `keywordfilter.match` - Report the number of suggestions filtered by the
  filter with the given ID.

  **Tags:**

  - `id` - The filter that was matched.

- `adm.rs.provider.duration-us` - A histogram that records the amount of time,
  in microseconds, that the adM Remote Settings provider took to generate
  suggestions.

  **Tags:**

  - `accepts-english` - If the request included an `Accept-Language` header that
    accepted any `en-*` locale. Only requests that do are provided with
    suggestions.

- `cache.memory.duration-us` - A histogram that records the amount of time, in
  microseconds, that the memory cache took to provide a suggestion. Includes the
  time it takes to fallback to the inner provider for cache misses and errors.

  **Tags:**

  - `cache-status` - If the response was pulled from the cache or regenerated.
    `"hit"`, `"miss"`, `"error"`, or `"none"`.

- `cache.memory.hit` - A counter that is incremented every time the in-memory
  cache is queried and a cached suggestion is found.

- `cache.memory.miss` - A counter that is incremented every time the in-memory
  cache is queried and a cached suggestion is not found.

- `cache.memory.pointers-len` - A gauge representing the number of entries in
  the first level of hashing in the in-memory deduped hashmap.

- `cache.memory.storage-len` - A gauge representing the number of entries in the
  second level of hashing in the in-memory deduped hashmap.

- `cache.redis.duration-us` - A histogram that records the amount of time, in
  microseconds, that the Redis cache took to provide a suggestion. Includes the
  time it takes to fallback to the inner provider for cache misses and errors.

  **Tags:**

  - `cache-status` - If the response was pulled from the cache or regenerated.
    `"hit"`, `"miss"`, `"error"`, or `"none"`.

- `cache.redis.hit` - A counter that is incremented every time the redis cache
  is queried and a cached suggestion is found.

- `cache.redis.miss` - A counter that is incremented every time the redis cache
  is queried and a cached suggestion is not found.
