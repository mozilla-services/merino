# Data collection

This page should list all metrics and logs that Merino is expected to emit in
production, including what should be done about them, if anything.

## Logs

This list does not include any `DEBUG` or `TRACE` level events, since those are
not logged by default in production. The events below are grouped by crate, and
the level and type of the log is listed.

### `merino-adm`

- `INFO adm.remote-settings.sync-start` - The Remote Settings provider has
  started syncing records.

- `WARN adm.remote-settings.empty` - After syncing no records were found in the
  Remote Settings collection.

### `merino-cache`

- `INFO cache.redis.save-error` - There was an error while saving a cached
  suggestion to the Redis server.

### `merino-web`

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
