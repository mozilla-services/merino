# load-tests

This directory contains source code for load tests for Merino.

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

## Run tests locally

You can run the load tests locally from the repository root directory using:

```text
docker-compose -f test-engineering/load-tests/docker-compose.yml up --scale locust_worker=4
```

## Run distributed load tests on GCP

You can run the distributed load tests on Cloud Shell from the `load-tests`
directory using the bash script `load_test_setup.sh`.

Make the file executable by:

```text
chmod +x setup_k8.sh
```

Then, run the file:

```text
./setup_k8.sh
```
Select the option `create`, it goes through the process of creating a cluster, setting up the env variables
and building the docker image.

*Note*: Be sure to delete the cluster(steps mentioned at the end of this doc) after you are done running load tests and copying the logs.

Run a watch loop while an external IP address is assigned to the `locust-master` service, which should take a few seconds:

```text
kubectl get svc locust-master --watch
```

Press Ctrl+C to exit the watch loop and then run the following command to note the external IP address:

```bash
EXTERNAL_IP=$(kubectl get svc locust-master -o jsonpath="{.status.loadBalancer.ingress[0].ip}")
```

```text
echo $EXTERNAL_IP
```

Open your browser and then open the Locust master web interface `http://$EXTERNAL_IP:8089`.

To modify the number of workers to say `20`, run:

```text
kubectl scale deployment/locust-worker --replicas=20
```

Currently, it's set to run `10` workers but it can be changed by running above command.


# How to grab the logs

Get the master pod name by running:

```text
kubectl get pods -o wide
```

then run:

```text
kubectl cp <master-pod-name>:/home/locust/merino_stats.csv merino_stats.csv
kubectl cp <master-pod-name>:/home/locust/merino_exceptions.csv merino_exceptions.csv
kubectl cp <master-pod-name>:/home/locust/merino_failures.csv merino_failures.csv
```

To remove all the `GET` requests and retain only the aggregate summary use:

```text
cat merino_stats.csv | grep -Ev "^GET," > merino_stats.csv.tmp
mv merino_stats.csv.tmp merino_stats.csv
```

Thereafter, copy these 3 files (merino_stats.csv, merino_exceptions.csv, merino_failures.csv) to [gist](https://gist.github.com/new) and provide a link to it in the [Merino load test history doc](https://docs.google.com/document/d/1BGNhKuclUH40Bit9KxYWLiv_N_VnE66uxi9pBFbRWbg/edit)

# Deleting the cluster

After you are done with load testing delete the cluster by running the same bash script and choosing the `delete` option

```text
./setup_k8.sh
```
Select `delete`

