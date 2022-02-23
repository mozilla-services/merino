# The Release Process

This project currently follows a [Continuous Delivery][continuous_delivery] process, but it's gradually moving toward [Continuous Deployment][continuous_deployment].

[continuous_delivery]: https://en.wikipedia.org/wiki/Continuous_delivery
[continuous_deployment]: https://en.wikipedia.org/wiki/Continuous_deployment

Whenever a commit is pushed to this repository's `main` branch, the deployment pipeline kicks in, deploying the changeset to the [`stage` environment](../firefox.md#stage).
After the deployment is complete, accessing the [`__version__` endpoint][stage_version] will show the commit hash of the deployed version, which will eventually match to the one of the latest commit on the `main` branch (a node with an older version might still serve the request before it is shut down).

[stage_version]: https://stage.merino.nonprod.cloudops.mozgcp.net/__version__

## Versioning
The commit hash of the deployed code is considered its version identifier. The commit hash can be retrieved locally via `git rev-parse HEAD`.

## Preventing deployment
Occasionally developers might want to prevent a commit from triggering the deployment pipeline. While this should be discouraged, there are some legitimate cases for doing so (e.g. docs only changes).
In order to prevent the deployment of the code from a PR when merging to `main`, the **title of that PR** must contain the `[do not deploy]` text. Note that, when generating the merge commit for a branch within the GitHub UI, the extened description must not be changed or care must be taken to ensure that `[do not deploy]`  is still present.
For example:

```
# PR title (NOT the commit message)
doc: Add documentation for the release process [do not deploy]
```

While the `[do not deploy]` can be anywhere in the title, it is recommended to place it at its end in order to better integrate with the current PR title practices.

The deployment pipeline will analyse the message of the merge commit (which will be contain the PR title) and make a decision based on it.

## Releasing to production
The process to promote a build from `stage` to `production` is currently manually initiated by SRE.
[This ticket](https://mozilla-hub.atlassian.net/browse/CONSVC-1566) (requires LDAP) deals with automating the process.
