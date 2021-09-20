<a name="0.2"></a>

## 0.2 (2021-09-20)

#### Features

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

#### Bug Fixes

- Allow overriding HTTP port of docker container ([fdc82823](fdc82823))
