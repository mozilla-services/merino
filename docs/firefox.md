# Configuring Firefox and Merino Environments

As of Firefox 93.0, Merino is not enabled by default. To enable it, set the
Firefox preference `browser.urlbar.merino.enabled` to `true`. By default Merino
will connect to the production environments. This is controlled with the
`browser.urlbar.merino.endpointURL` preference. See below for other options.

You can also query any of the endpoint URLs below with something like

```sh
curl 'https://stage.merino.nonprod.cloudops.mozgcp.net/api/v1/suggest?q=your query'
```

## Environments

### Production

*Endpoint URL*: <https://merino.services.mozilla.com/api/v1/suggest>

The primary environment for end users. Firefox is configured to use this by
default. As of 2021-10-25, this server is not active yet.

This environment only deploys manually as a result of operations triggering
deploys.

### Stage

*Endpoint URL*: <https://stage.merino.nonprod.cloudops.mozgcp.net/api/v1/suggest>

This environment is used for manual and load testing of the server. It is not
guaranteed to be stable or available. It is used as a part of the deploy process
to verify new releases before they got to production.

This environment automatically deploys new tags on the Merino repository.

### Dev

*Endpoint URL*: <https://dev.merino.nonprod.cloudops.mozgcp.net/api/v1/suggest>

This environment is unstable and is not guaranteed to work. It's primary use is
as a development area for operations.

This environment automatically deploys the latest commit to the `main` branch of
the repository.
