# Configuring Merino (Operations)

## Settings

Merino's settings can be specified in two ways: a YAML file placed in a specific
location, or via environment variables. Not all settings can be set with
environment variables, however. Notably, provider configuration must be done
with its own YAML file.

### [File organization](#file-organization)

These are the settings sources, with later sources overriding earlier ones.

- A base configuration checked into the repository, in `config/base.yaml`. This
  provides the default values for most settings.

- Per-environment configuration files in the `config` directory. The environment
  is selected using the environment variable `MERINO_ENV`. The settings for that
  environment are then loaded from `config/${env}.yaml`, if it exists. The
  default environment is "development". A "production" environment is also
  provided.

- A local configuration file not checked into the repository, at
  `config/local.yaml`. This file is in `.gitignore` and is safe to use for local
  configuration and secrets if desired.

- Environment variables that begin with `MERINO_` and use `__` (a double
  underscore) as a level separator. For example, `Settings::http::workers` can
  be controlled from the environment variable `MERINO_HTTP__WORKERS`.

The names given below are of the form "`yaml.path` (`ENVIRONMENT_VAR`)"

### General

- `env` (`MERINO_ENV`) - Only settable from environment variables. Controls
  which environment configuration is loaded, as described above.

- `debug` (`MERINO_DEBUG`) - Boolean that enables additional features to debug
  the application. This should not be set to true in public environments, as it
  reveals all configuration, including any configured secrets.

- `public_documentation` (`MERINO_PUBLIC_DOCUMENTATION`) - When users visit the
  root of the server, they will be redirected to this URL. Preferable a public
  wiki page that explains what the server is and does.

- `log_full_request` (`MERINO_LOG_FULL_REQUEST`) - Boolean that enables logging
  the entire suggestion request object as a part of the tracing log, including
  the search query. When the setting is false (default), the suggest request
  object should be logged, but the search query should be blank. Note that
  access to the collected query logs is restricted.

### HTTP

Settings for the HTTP server.

- `http.listen` (`MERINO_HTTP__LISTEN`) - An IP and port to listen on, such as
  `127.0.0.1:8080` or `0.0.0.0:80`.
- `http.workers` (`MERINO_HTTP__WORKERS`) - Optional. The number of worker
  threads that should be spawned to handle tasks. If not provided will default
  to the number of logical CPU cores available.

### Logging

Settings to control the format and amount of logs generated.

- `logging.format` - The format to emit logs in. One of

  - `pretty` (default in development) - Multiple lines per event, human-oriented
    formatting and color.
  - `compact`- A single line per event, with formatting and colors.
  - `mozlog` (default in production) - A single line per event, formatted as
    JSON in [MozLog](https://wiki.mozilla.org/Firefox/Services/Logging) format.

- `logging.info` (`MERINO_LOGGING__LEVELS`) - Minimum level of logs that should
  be reported. This should be a number of _entries_ separated by commas (for
  environment variables) or specified as list (YAML).

  This will be combined with the contents of the `RUST_LOG` environment variable
  for compatibility. `RUST_LOG` will take precedence over this setting. If the
  environment variable `MERINO_LOGGING__LEVELS` is specified, all the settings
  in the YAML file will be ignored.

  Each entry can be one of `ERROR`, `WARN`, `INFO`, `DEBUG`, or `TRACE` (in
  increasing verbosity), with an optional component that specifies the source of
  the logs. For example `INFO,merino_web=DEBUG,reqwest=WARN` would set the
  default log level to INFO, but would lower the level to `DEBUG` for the
  `merino-web` crate and raise it to `WARN` for the reqwest crate.

### Metrics

Settings for Statsd/Datadog style metrics reporting.

- `metrics.sink_host` (`MERINO_METRICS__SINK_ADDRESS`) - The IP or hostname to
  send metrics to over UDP. Defaults to `0.0.0.0`.

- `metrics.sink_port` (`MERINO_METRICS__SINK_PORT`) - The port to send metrics
  to over UDP. Defaults to 8125.

- `max_queue_size_kb` (`MERINO_METRICS__MAX_QUEUE_SIZE_KB`) - The maximum size
  of the buffer that holds events waiting to be sent. If unsent events rise
  above this, then metrics will be lost. Defaults to 32KB.

### Sentry

Error reporting via Sentry.

- `sentry.mode` (`MERINO_SENTRY__MODE`) - The type of Sentry integration to
  enable. One of `release`, `server_debug`, `local_debug`, or `disabled`. The
  two `debug` settings should only be used for local development.

If `sentry.mode` is set to `release`, then the following two settings are
required:

- `sentry.dsn` - Configuration to connect to the Sentry project.
- `sentry.env` - The environment to report to Sentry. Probably "production",
  "stage", or "dev".

If `sentry.mode` is set to `disabled`, no Sentry integration will be activated.
If it is set to `local_debug`, the DSN will be set to a testing value
recommended by Sentry, and extra output will be included in the logs.

The mode can be set to `server_debug`, which will allow testing real integration
with Sentry. Sentry integration and debug logging will be activated. It is
recommended to use the [merino-local][sentry-merino-local] sentry environment.
See that page for DSN information. The following two settings are required:

[sentry-merino-local]: https://sentry.prod.mozaws.net/operations/merino-local

- `sentry.dsn` - Configuration to connect to the Sentry project. A testing
  project should be used.
- `sentry.who` - Your username, which will be used as the environment, so that
  you can filter your results out in Sentry's web interface.

### Redis

Connection to Redis. This is used by the Redis provider cache below.

- `redis.url` (`MERINO_REDIS__URL`) - The URL to connect Redis at. Example:
  `redis://127.0.0.1/0`.

### Remote_settings

Connection to Remote Settings. This is used by the Remote Settings suggestion
provider below.

- `remote_settings.server` (`MERINO_REMOTE_SETTINGS__SERVER`) - The server to
  sync from. Example: `https://firefox.settings.services.mozilla.com`.

- `remote_settings.default_bucket` (`MERINO_REMOTE_SETTINGS__DEFAULT_BUCKET`) -
  The bucket to use for Remote Settings providers if not specified in the
  provider config. Example: "main".

- `remote_settings.default_collection`
  (`MERINO_REMOTE_SETTINGS__DEFAULT_COLLECTION`) - The collection to use for
  Remote Settings providers if not specified in the provider config. Example:
  "quicksuggest".

### Location

Configuration for determining the location of users.

- `location.maxmind_database` (`MERINO_LOCATION__MAXMIND_DATABASE`) - Path to a
  MaxMind GeoIP database file. Optional. If not specified, geolocation will be
  disabled.

### Provider Configuration

The configuration for suggestion providers.

Note that the provider settings are configured by separate YAML files located in
`config/providers`. These settings cannot be configured via environment variables.
The file organization is identical to the [top level settings](#file-organization)
with the same source overriding rule.

#### Configuration Object

Each provider configuration has a `type`, listed below, and it's own individual
settings.

_Example_:

```yaml
wiki_fruit:
  type: wiki_fruit
```

#### Configuration File

Each configuration file should be a map where the keys are provider IDs will be
used in the API to enable and disable providers per request. The values are
provider configuration objects, detailed below. Some providers can takes other
providers as children. Because of this, each key in this config is referred to
as a "provider tree".

_Example_:

```yaml
adm:
  type: memory_cache
  inner:
    type: remote_settings
    collection: "quicksuggest"
wiki_fruit:
  type: wiki_fruit
debug:
  type: debug
```

#### Leaf Providers

These are production providers that generate suggestions.

- Remote Settings - Provides suggestions from a RS collection, such as the
  suggestions provided by adM. See also the top level configuration for Remote
  Settings, below.
  - `type=remote_settings`
  - `bucket` - Optional. The name of the Remote Settings collection to pull
    suggestions from. If not specified, the global default will be used.
  - `collection` - Optional. The name of the Remote Settings collection to pull
    suggestions from. If not specifeid, the global default will be used.
  - `resync_interval_sec` - Optional. The time between re-syncs of Remote
    Settings data, in seconds. Defaults to 3 hours.

#### Combinators

These are providers that extend, combine, or otherwise modify other providers.

- Multiplexer - Combines providers from multiple sub-providers.

  - `type=multiplexer`
  - `providers` - A list of other provider configs to draw suggestions from.

  _Example_:

  ```yaml
  sample_multi:
    type: multiplexer
    providers:
      - fixed:
        type: fixed
        value: I'm a banana
      - debug:
        type: debug
  ```

- Timeout - Returns an empty response if the wrapped provider takes too long to
  respond.

  - `type=timeout`
  - `inner` - Another provider configuration to generate suggestions with.
  - `max_time_ms` - The time, in milliseconds, that a provider has to respond
    before an empty result is returned.

- KeywordFilter - Filters the suggestions coming from the wrapped provider with
  the given blocklist.

  - `type=keyword_filter`
  - `suggestion_blocklist` - The map used to define the blocklist rules. Each
    entry contains a rule id and an associated regular expression that
    recommended titles are matched against.
  - `inner` - The wrapped provider to draw suggestions from.

  _Example_:

  ```yaml
  filtered:
    type: keyword_filter
    suggestion_blocklist:
      no_banana: "(Banana|banana|plant)"
    inner:
      type: multiplexer
      providers:
        - fixed:
          type: fixed
          value: I'm a banana
        - debug:
          type: debug
  ```

- Stealth - Runs another provider, but hides the results. Useful for load
  testing of new behavior.

  - `type=stealth`
  - `inner` - Another provider configuration to run.

#### Caches

These providers take suggestions from their children and cache them for future
use.

- Memory Cache - An in-memory, per process cache.

  - `type=memory_cache`
  - `default_ttl_sec` - The time to store suggestions before, if the inner
    provider does not specify a time.
  - `cleanup_interval_sec` - The cache will automatically remove expired entries
    with this period. Note that expired entries are also removed dynamically if
    a matching request is processed.
  - `max_removed_entries` - While running the cleanup task, at most this many
    entries will be removed before cancelling the task. This should be used to
    limit the maximum amount of time the cleanup task takes. Defaults to
    100_000.
  - `default_lock_timeout_sec` - The amount of time a cache entry can be locked
    for writing.
  - `inner` - Another provider configuration to generate suggestions with.

- Redis Cache - A remote cache that can be shared between processes.
  - `type=redis_cache`
  - `default_ttl_sec` - The time to store suggestions before, if the inner
    provider does not specify a time.
  - `default_lock_timeout_sec` - The amount of time a cache entry can be locked
    for writing.
  - `inner` - Another provider configuration to generate suggestions with.

#### Development providers

These should not be used in production, but are useful for development and
testing.

- Debug - Echos back the suggestion request that it receives formatted as JSON
  in the `title` field of a suggestion.

  - `type=debug`

- WikiFruit - A very basic provider that suggests Wikipedia articles for the
  exact phrases "apple", "banana", and "cherry".

  - `type=wiki_fruit`

- Null - A provider that never suggests anything. Useful to fill in combinators
  and caches for testing.
  - `type="null"` - Note that `null` in YAML is an actual null value, so this
    must be specified as the string `"null"`.
