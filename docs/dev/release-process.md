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
Developers with write access to the Merino repository can initiate a deployment to production after a Pull-Request on the Merino GitHub repository is merged to the `main` branch.
While any developer with write access can trigger the deployment to production, the _expectation_ is that individual(s) who authored and merged the Pull-Request should do so, as they are the ones most familiar with their changes and who can tell, by looking at the data, if anything looks anomalous.
In general authors should feel _responsible_ for the changes they make and shepherd throught their deployment.

Releasing to production can be done by:

1. opening the [CircleCI dashboard][circleci_dashboard];
2. looking up the pipeline named `merino <PR NUMBER>` running in the `main-workflow`; this pipeline should either be in a running status (if the required test jobs are still running) or in the "on hold" status, with the `unhold-to-deploy-to-prod` being held;
3. once in the "on hold" status, with all the other jobs successfully completed, clicking on the "thumbs up" action on the `unhold-to-deploy-to-prod` job row will approve it and trigger the deployment, unblocking the `deploy-to-prod` job;
4. developers **must** monitor the [Merino Application & Infrastructure][merino_app_info] dashboard for any anomaly, for example significant changes in HTTP response codes, increase in latency, cpu/memory usage (most things under the infrastructure heading).

[circleci_dashboard]: https://app.circleci.com/pipelines/github/mozilla-services/merino?branch=main&filter=all
[merino_app_info]: https://earthangel-b40313e5.influxcloud.net/d/Cm83vS57z/merino-application-and-infrastructure?orgId=1&refresh=1m

## What to do if production is broken?
Don't panic and follow the instructions below:

- depending on the severity of the problem, decide if this warrants [kicking off an incident][incident_docs];
- if the root cause of the problem can be identified in a relatively small time, create a PR for the fix.
    - verify the fix locally;
    - verify the fix on `stage`, after it is reviewed by a Merino developer and merged;
    - [deploy it to production](#releasing-to-production).

**OR**

- if the root cause of the problem is harder to track down, revert the last commit and then [deploy the revert commit to production](#releasing-to-production).

[incident_docs]: https://mana.mozilla.org/wiki/pages/viewpage.action?pageId=150549987
