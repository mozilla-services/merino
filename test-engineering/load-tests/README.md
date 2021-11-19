# load-tests

This directory contains source code for load tests for Merino.

**Please note that this is work in progress.** 🚧

## Run tests locally

You can run the load tests locally from the repository root directory using:

```text
docker-compose -f test-engineering/load-tests/docker-compose.yml up --scale locust_worker=4
```

# Run distributed load tests on GCP

You can run the distributed load tests from the repository root directory using the bash script ```load_test_setup.sh```
Follow the below steps:

Make the file executable by:
```chmod +X load_test_setup.sh```

then, run the file:
```load_test_setup.sh```

Wait for the External_IP to be generated, it takes a few seconds, run a watch loop while an external IP address is assigned to the Locust master service:
```kubectl get svc locust-master --watch```

Press Ctrl+C to exit the watch loop and then run the following command to note the external IP address:
```EXTERNAL_IP=$(kubectl get svc locust-master -o jsonpath="{.status.loadBalancer.ingress[0].ip}")```

```echo $EXTERNAL_IP```

Open your browser and then open the Locust master web interface http://$EXTERNAL_IP:8089"


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
