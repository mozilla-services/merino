<a name="0.3.0"></a>

# 0.3.0 (2021-10-15)

## Features

- added request_id (#160) ([9a77937e](9a77937e))
- Report stacktraces for errors to Sentry ([74e86b31](74e86b31))
- make metrics sink address support hostnames (#142) ([dbcd9634](dbcd9634))
- **adm:**  Periodically re-sync suggestions from Remote Settings ([6fc67ff6](6fc67ff6))
- **suggest:**  Add timeout provider ([9df0a8ad](9df0a8ad), closes [#55](55))
- **web:**
  - Allow specifying providers when requesting suggestions ([201d0daa](201d0daa))
  - Add endpoint that lists available providers ([5d9efb7d](5d9efb7d))

## Bug Fixes

- **settings:**  Make memory cache configuration values optional ([e59ee421](e59ee421), closes [#136](136))

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
