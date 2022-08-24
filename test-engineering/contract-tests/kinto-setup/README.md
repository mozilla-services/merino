# kinto-setup

This directory contains source code for setting up Kinto for contract tests.

## Environment variables

Please set the following environment variables in `docker-compose.yml`:

* `KINTO_URL`: The URL of the Kinto service
* `KINTO_BUCKET`: The ID of the Kinto bucket to create
* `KINTO_COLLECTION`: The ID of the Kinto collection to create

## Contributing

**mypy** is used for type checking. Execute from the root using command:

```text
mypy \
test-engineering/contract-tests/kinto-setup/ \
--config-file=test-engineering/contract-tests/kinto-setup/mypy.ini
```
