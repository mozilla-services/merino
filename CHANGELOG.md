<a name="v0.5.0"></a>

## v0.5.0 (2021-12-14)

The changes in 0.4.4 were never deployed, but probably should have been a minor
update, since it introduced a large change to the remote settings provider. As
such, this release is getting that minor version update.

#### Bug Fixes

- **adm:** Include `/v1` on the end of the remote settings URL we give to the RS
  client ([f89a3adf](f89a3adf))

#### Features

- check signature of remote-settings content ([5a0f1f17](5a0f1f17))
- don't hide inner errors in the log message ([7d50255d](7d50255d))

<a name="0.4.4"></a>

## 0.4.4 (2021-12-06) - Not deployed

#### Bug Fixes

- specify a type for info, warn and error logs ([0a5135e4](0a5135e4))
- **adm:** Integrate remote-settings-client (#232) ([385e220](385e220))

<a name="0.4.3"></a>

## 0.4.3 (2021-11-26)

#### Bug Fixes

- **adm:** Filter keywords that do not start with the partial query (#247)
  ([d166f66f](d166f66f))
- **api:** Split advertisers from the providers (#246) ([e3bdc366](e3bdc366))

<a name="0.4.2"></a>

## 0.4.2 (2021-11-24)

#### Features

- Add sub-microsecond precision using floats for timer metrics
  ([959ecd24](959ecd24))
- **adm:** Make suggestion score configurable (#235) ([6cd9f3d4](6cd9f3d4))

#### Bug Fixes

- **logs:** don't record a value for missing location in logs
  ([561c05ea](561c05ea), closes [#230](230))

<a name="0.4.1"></a>

## 0.4.1 (2021-11-24)

#### Bug Fixes

- **adm:** Use Firefox's full_keyword algorithm (#214) ([009bd1e8](009bd1e8))

<a name="v0.4.0"></a>

## v0.4.0 (2021-11-09)

#### Bug Fixes

- log sugestion request data with proper MozLog type information
  ([014f27c8](014f27c8))

#### Features

- initialize suggestion providers at start up [CONSVC-1247] (#193)
  ([e1c2561c](e1c2561c), closes [#127](127))
- instrument cache hit and misses (#208) ([3df72ed1](3df72ed1))
- Add duration-us metrics to RedisCache, MemoryCache, and RemoteSettings
  providers ([f6338aeb](f6338aeb))
- add SuggestionRequest to tracing logs ([803ff69a](803ff69a))
- implement a provider that filters results by keywords ([3ffcf693](3ffcf693))
- **suggest:**
  - Suggestion providers can their own cache keys ([2864e187](2864e187))
  - Add Stealth provider combinator ([03c66161](03c66161), closes [#181](181))

#### Performance

- Specify minimum set of cache inputs needed for every provider
  ([22784bcc](22784bcc))

<a name="0.3.0"></a>

# 0.3.0 (2021-10-15)

## Features

- added request_id (#160) ([9a77937e](9a77937e))
- Report stacktraces for errors to Sentry ([74e86b31](74e86b31))
- make metrics sink address support hostnames (#142) ([dbcd9634](dbcd9634))
- **adm:** Periodically re-sync suggestions from Remote Settings
  ([6fc67ff6](6fc67ff6))
- **suggest:** Add timeout provider ([9df0a8ad](9df0a8ad), closes [#55](55))
- **web:**
  - Allow specifying providers when requesting suggestions
    ([201d0daa](201d0daa))
  - Add endpoint that lists available providers ([5d9efb7d](5d9efb7d))

## Bug Fixes

- **settings:** Make memory cache configuration values optional
  ([e59ee421](e59ee421), closes [#136](136))

<a name="0.2"></a>

# 0.2 (2021-09-20)

## Features

- add variant fields to suggest api (#126) ([5b04053b](5b04053b))
- Add setting to configure sentry env ([10392511](10392511))
- Add memory cache locking to prevent spurious update requests. (#123)
  ([059f355a](059f355a))
- Add Sentry integration (#111) ([27e77020](27e77020))
- add locking to redis cache (#110) ([7d94f9e9](7d94f9e9), closes [#104](104))
- **settings:** Make suggestion providers much more configurable at runtime
  ([3685d700](3685d700))
- **suggest:** Add score field to suggestion (#119) ([06556d88](06556d88),
  closes [#118](118))

## Bug Fixes

- Allow overriding HTTP port of docker container ([fdc82823](fdc82823))
