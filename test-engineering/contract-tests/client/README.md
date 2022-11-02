# Merino Contract Tests - Client

## Overview

This directory contains a Python-based test framework for the contract tests. 
The HTTP client used in the framework supports:

* Requests to Kinto (Remote Settings) for record population.
* Requests for suggestions from Merino, with response checks.

For more details on contract test design, refer to the contract-tests 
[README][contract_tests_readme].

## Contributing

**mypy** is used for type checking. Execute from the root using command:

```text
mypy \
test-engineering/contract-tests/client/ \
--config-file=test-engineering/contract-tests/client/mypy.ini
```

## Scenarios

The client is instructed on request and response check actions via scenarios, 
recorded in the `scenarios.yml` file. A scenario is defined by a name, a description 
and steps.

#### Name

A test name should identify the use case under test. Names are written in snake case
with double-underscores `__` for scenario and behavior delineation.

Example:
`remote_settings__refresh`

#### Description

A test description should outline, in greater detail, the purpose of a test, meaning
the feature or common user interaction being verified.

Example:
_Test that Merino successfully returns refreshed output in the cases of
suggestion content updates and additions_

#### Steps

##### Kinto Service

* The Kinto service scenario step populates the records used by the Merino service
* Kinto `request` fields:
  * `service` - Set the value to `kinto`, to direct requests to the Kinto service. 
  * `delay` - (optional) Set seconds to pause before execution of request.
  * `record_id` - Set the ID that will correspond with the records specified in
                  `filename`
  * `filename` - Set the file with records to upload. The files are located in 
                 `..\volumes\kinto`.
  * `data_type` - Set to `data` or `offline-expansion-data`.
* A Kinto service scenario step will not check a `response` defined in the scenario, 
  but will raise an error in the event of an unexpected HTTP response.
* All records populated by Kinto service scenario steps are deleted as part of the 
  scenario teardown.

Example:
```yaml
- request:
    service: kinto
    record_id: "data-01"
    filename: "data-01.json"
    data_type: "data"
```

##### Merino Service

* The Merino service scenario step sends queries to merino and checks the validity of 
  the responses.
* Merino `request` fields:
  * `service` - Set the value to `merino`, to direct requests to the Merino service. 
  * `delay` - (optional) Set seconds to pause before execution of request.
  * `method` - Set the HTTP request method.
  * `path` - Set the query and parameters for the request method.
  * `headers` - Set a list of HTTP request headers.
* Merino `response` fields:
  * `status_code` - Set the expected HTTP response status code.
  * `content` - Set a list of expected merino suggestion content.
    * The `request_id` is excluded from verification and can be set to `null` in the 
    scenario.

Example:
```yaml
- name: wiki_fruit__apple
  description: Test that Merino successfully returns a WikiFruit suggestion
  steps:
    - request:
        service: merino
        method: GET
        path: '/api/v1/suggest?q=apple'
        headers:
          - name: User-Agent
            value: 'Mozilla/5.0 (Windows NT 10.0; rv:10.0) Gecko/20100101 Firefox/91.0'
          - name: Accept-Language
            value: 'en-US'
      response:
        status_code: 200
        content:
          client_variants: []
          server_variants: []
          request_id: null
          suggestions:
            - block_id: 1
              full_keyword: 'apple'
              title: 'Wikipedia - Apple'
              url: 'https://en.wikipedia.org/wiki/Apple'
              impression_url: 'https://127.0.0.1/'
              click_url: 'https://127.0.0.1/'
              provider: 'test_wiki_fruit'
              advertiser: 'test_advertiser'
              is_sponsored: false
              icon: 'https://en.wikipedia.org/favicon.ico'
              score: 0.0
```

## Local Execution

To execute the test scenarios outside the client Docker container, create a Python
virtual environment, set environment variables, expose the Merino and Kinto API ports
in the `docker-compose.yml` and use a pytest command. It is recommended to execute the
tests within a Python virtual environment to prevent dependency cross contamination.

1. Create a Virtual Environment

    This project uses [pyenv] for environment management.

2. Setup Environment Variables

    The following environment variables are set in `docker-compose.yml`, but will
    require local setup via command line, pytest.ini file or IDE configuration:
    * `MERION_URL`: The URL of the Merino service
      * Example: `MERINO_URL=http://localhost:8000`
    * `KINTO_URL`: The URL of the Kinto service
      * Example: `KINTO_URL=http://localhost:8888`
    * `KINTO_ATTACHMENTS_URL`: The URL of the Kinto Attachments service
      * Example: `KINTO_ATTACHMENTS_URL=http://localhost:80`
    * `KINTO_BUCKET`: The ID of the Kinto bucket to create
      * Example: `KINTO_BUCKET=main`
    * `KINTO_COLLECTION`: The ID of the Kinto collection to create
      * Example: `KINTO_COLLECTION=quicksuggest`
    * `KINTO_DATA_DIR`: The directory containing advertiser data
      * Example: `KINTO_DATA_DIR=tests/contract/volumes/kinto`
    * `SCENARIOS_FILE`: The directory containing contract test scenario files
      * Example: `SCENARIOS_FILE=tests/contract/volumes/client/scenarios.yml`

3. Modify `tests/contract/docker-compose.yml`

    In the `merino` definition, expose port 8000 by adding the following:
    ```yaml
    ports:
      - "8000:8000"
    ```

    In the `kinto` definition, expose port 8888 by adding the following:
    ```yaml
    ports:
      - "8888:8888"
    ```

    In the `kinto-attachments` definition, expose port 80 by adding the following:
    ```yaml
    ports:
      - "80:80"
    ```

4. Run `merino`, `kinto-setup`, `kinto` and `kinto-attachment` docker containers.

   Execute the following from the project root:
   ```shell
    docker-compose \
      -f test-engineering/contract-tests/docker-compose.yml \
      -p merino-rs-contract-tests \
      up merino
   ```

5. Run the contract tests

    Execute the following from the project root:
    ```shell
    pytest test-engineering/contract-tests/client/tests/test_merino.py -vv
    ```
    * Tests can be run individually using [-k _expr_][pytest-k].

      Example executing the `remote_settings__refresh` scenario:
      ```shell
      pytest test-engineering/contract-tests/client/tests/test_merino.py -vv \
        -k remote_settings__refresh
      ```

[contract_tests_readme]: ../README.md
[pyenv]: https://github.com/pyenv/pyenv
[pytest-k]: https://docs.pytest.org/en/latest/example/markers.html#using-k-expr-to-select-tests-based-on-their-name

