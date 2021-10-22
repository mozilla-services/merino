# load-tests

This directory contains source code for load tests for Merino.

**Please note that this is work in progress.** 🚧

## Run tests locally

You can run the load tests locally from the repository root directory using:

```text
docker-compose -f test-engineering/load-tests/docker-compose.yml up --scale locust_worker=4
```
