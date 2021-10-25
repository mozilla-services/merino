# load-tests

This directory contains source code for load tests for Merino.

**Please note that this is work in progress.** 🚧

## Run tests locally

You can run the load tests locally from the repository root directory using:

```text
docker-compose -f test-engineering/load-tests/docker-compose.yml up --scale locust_worker=4
```

## RS_QUERIES file in the docker-compose.yml

This expects an `InstantSuggest_Queries_*.json` file from the source-data directory in the `quicksuggest-rs` repo for the path in RS_QUERIES_FILE in the `docker-compose.yml` file.


