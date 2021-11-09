# load-tests

This directory contains source code for load tests for Merino.

**Please note that this is work in progress.** 🚧

## Run tests locally

You can run the load tests locally from the repository root directory using:

```text
docker-compose -f test-engineering/load-tests/docker-compose.yml up --scale locust_worker=4
```

## Environment variables

Please set the following environment variables when running these load tests.

### For locust master and worker nodes

* `LOAD_TESTS__LOGGING_LEVEL`: Level for the logger in the load tests as an int (`10` for `DEBUG`, `20` for `INFO` etc.)
* `KINTO__SERVER_URL`: Server URL of the Kinto instance to download suggestions from
* `KINTO__BUCKET`: Kinto bucket with the suggestions
* `KINTO__COLLECTION`: Kinto collection with the suggestions

### For the locust master node

* `LOCUST_HOST`: Server URL of the Merino deployment to load test

### For locust worker nodes

* `LOCUST_MASTER_NODE_HOST`: Server URL of locust master for distributed load testing
