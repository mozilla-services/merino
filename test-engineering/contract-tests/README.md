# Merino Contract Tests 🤖

This directory contains source code for automated contract tests for Merino.

## Overview

The contract test suite is designed to be set up as a docker-compose CI workflow.
To simulate common use cases, the suite utilizes 5 docker containers: `client`,
`merino`, `kinto-setup`, `kinto` & `kinto-attachments`.

The following sequence diagram depicts container interactions during the
`remote_settings__coffee` test scenario.

**Test Scenario: remote_settings__coffee**
![Sequence diagram of the integration tests][sequence_diagram]
**Notes:**
* The interactions between `kinto` and `kinto-attachments` are not depicted.
* The diagram was composed using Miro

### client

The `client` container consists of a Python-based test framework that executes the
contract tests. The HTTP client used in the framework can be instructed to prepare
Remote Settings data through requests to kinto and can verify merino functionality
through requests to the merino service.

For more details see the client [README][client_readme]

### merino

The `merino` container encapsulates the merino service under test.

For more details, see the merino [README][merino_readme] or project
[documentation][merino_docs].

### kinto-setup

The `kinto-setup` container consists of a Python-based program responsible for
defining the Remote Settings bucket, "main", and collection, "quicksuggest", prior
to the `merino` container startup, a pre-requisite.

For more details see the kinto-setup [README][kinto_setup_readme]

### kinto & kinto-attachments

The `kinto` container holds a minimalist storage service with synchronisation and
sharing abilities. It uses the `kinto-attachments` container to store data locally.

For more details see the Remote Settings [documentation][kinto_docs]

## Local Execution

To run the contract tests locally, execute the following from the repository root:

```shell
docker-compose \
  -f test-engineering/contract-tests/docker-compose.yml \
  -p merino-rs-contract-tests \
  up --abort-on-container-exit --build
```

To remove contract test containers and network artifacts, execute the following from
the repository root:

```shell
docker-compose \
  -f test-engineering/contract-tests/docker-compose.yml \
  -p merino-rs-contract-tests \
  down
```

[client_readme]: ./client/README.md
[kinto_docs]: https://remote-settings.readthedocs.io/en/latest/
[kinto_setup_readme]: ./kinto-setup/README.md
[merino_docs]: ../../docs/SUMMARY.md
[merino_readme]: ../../README.md
[sequence_diagram]: sequence_diagram.jpg
