# Merino Contract Test Client

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

### Steps

#### Kinto Service

* The Kinto service scenario step populates the records used by the Merino service
* Kinto `request` fields:
  * `service` - Set the value to `kinto`, to direct requests to the Kinto service. 
  * `delay` - (optional) Set seconds to pause before execution of request.
  * `filename` - Set the file with records to upload. The files are located in 
                 `..\contract-tests\volumes\kinto`.
  * `data_type` - Set to `data` or `offline-expansion-data`.
* A Kinto service scenario step will not check a `response` defined in the scenario, 
  but will raise an error in the event of an unexpected HTTP response.

Example:
```yaml
- request:
    service: kinto
    filename: "data-01.json"
    data_type: "data"
```

#### Merino Service

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

[contract_tests_readme]: ../README.md
