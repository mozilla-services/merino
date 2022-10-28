# Merino Contract Tests - Kinto-Setup

## Overview

This directory contains source code for setting up Kinto for contract tests.
Specifically, it is responsible for the creation of the remote settings bucket and
collection, a pre-requisite for the Merino service.

For more details on contract test design, refer to the contract-tests
[README][contract_tests_readme].

## Contributing

**mypy** is used for type checking. Execute from the root using command:

```text
mypy \
  test-engineering/contract-tests/kinto-setup/ \
  --config-file=test-engineering/contract-tests/kinto-setup/mypy.ini
```

## Local Execution

To execute the kinto-setup outside the Docker container, create a python virtual
environment, set environment variables, expose the Kinto API port in the
docker-compose.yml and use a python command. It is recommended to execute the setup
within a Python virtual environment to prevent dependency cross contamination.

1. Create a Virtual Environment

    This project uses [pyenv] for environment management.

2. Setup Environment Variables

    The following environment variables are set in `docker-compose.yml`, but will
    require local setup via command line, pytest.ini file or IDE configuration:
    * `KINTO_URL`: The URL of the Kinto service
      * Example: `KINTO_URL=http://localhost:8888`
    * `KINTO_BUCKET`: The ID of the Kinto bucket to create
      * Example: `KINTO_BUCKET=main`
    * `KINTO_COLLECTION`: The ID of the Kinto collection to create
      * Example: `KINTO_COLLECTION=quicksuggest`

3. Modify `tests/contract/docker-compose.yml`

    In the `kinto` definition, expose port 8888 by adding the following:
    ```yaml
    ports:
      - "8888:8888"
    ```

4. Run `kinto` and `kinto-attachment` docker containers.

   Execute the following from the project root:
   ```shell
    docker-compose \
      -f test-engineering/contract-tests/docker-compose.yml \
      -p merino-rs-contract-tests \
      up kinto
   ```

5. Run the kinto-setup service

    Execute the following from the project root:
    ```shell
    python test-engineering/contract-tests/kinto-setup/main.py
    ```

[contract_tests_readme]: ../README.md
[pyenv]: https://github.com/pyenv/pyenv
[merino_dev_docs]: ../../../docs/dev/index.md
