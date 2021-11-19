#!/bin/bash
set -eu

#Setup environment
GCLOUD=$(which gcloud)
SED=$(which sed)
KUBECTL=$(which kubectl)

DIRECTORY=$(pwd)
SDK_DIRECTORY=google-cloud-sdk
MERINO_DIRECTORY=$DIRECTORY/kubernetes-config
MASTER_FILE=locust-master-controller.yml
WORKER_FILE=locust-worker-controller.yml
SERVICE_FILE=locust-master-service.yml

#Declare variables
LOCUST_IMAGE_TAG=$(git log -1 --pretty=format:%h)
#LOCUST_IMAGE_TAG=qwe123
echo "Docker image tag for locust is set to: ${LOCUST_IMAGE_TAG}"

##Kubernetes Manifests variables
KINTO_SERVER_URL=https://firefox.settings.services.mozilla.com
KINTO_COLLECTION=quicksuggest
KINTO_BUCKET=main
CLUSTER=merino-load-test
TARGET=stage.merino.nonprod.cloudops.mozgcp.net
SCOPE="https://www.googleapis.com/auth/cloud-platform"
REGION=us-central1
ZONE=${REGION}-b

#Configure Kubernetes
echo -e "==================== Prepare environments with set of environment variables "
echo -e "==================== Set Kubernetes Cluster "
export CLUSTER=$CLUSTER
echo -e "==================== Set Kubernetes TARGET "
export TARGET=$TARGET
echo -e "==================== Set SCOPE "
export SCOPE=$SCOPE

echo -e "==================== Refresh Kubeconfig at path ~/.kube/config "
$GCLOUD container clusters get-credentials $CLUSTER --zone $ZONE --project $GOOGLE_CLOUD_PROJECT

##Build Docker Images
echo -e "==================== Build the Docker image and store it in your project's container registry. Tag with the latest commit hash "
$GCLOUD builds submit --tag gcr.io/$GOOGLE_CLOUD_PROJECT/locust-merino:$LOCUST_IMAGE_TAG
echo -e "==================== Verify that the Docker image is in your project's container repository"
$GCLOUD container images list | grep locust-merino

##Deploying the Locust master and worker nodes
echo -e "==================== Update Kubernetes Manifests "
echo -e "==================== Replace the target host and project ID with the deployed endpoint and project ID in the locust-master-controller.yml and locust-worker-controller.yml files"

FILES=($MASTER_FILE $WORKER_FILE)
for file in "${FILES[@]}"
do
    $SED -i -e "s/\[TARGET_HOST\]/$TARGET/g" $MERINO_DIRECTORY/$file
    $SED -i -e "s/\[PROJECT_ID\]/$GOOGLE_CLOUD_PROJECT/g" $MERINO_DIRECTORY/$file
    $SED -i -e "s/\[LOCUST_IMAGE_TAG\]/$LOCUST_IMAGE_TAG/g" $MERINO_DIRECTORY/$file
    $SED -i -e "s/\[KINTO_BUCKET\]/$KINTO_BUCKET/g" $MERINO_DIRECTORY/$file
    $SED -i -e "s/\[KINTO_COLLECTION\]/$KINTO_COLLECTION/g" $MERINO_DIRECTORY/$file
    $SED -i -e "s/\[KINTO_SERVER_URL\]/$KINTO_SERVER_URL/g" $MERINO_DIRECTORY/$file
done

$KUBECTL delete pods --all
$KUBECTL delete svc locust-master

##Deploy the Locust master and worker nodes using Kubernetes Manifests
echo -e "==================== Deploy the Locust master and worker nodes"
$KUBECTL apply -f $MERINO_DIRECTORY/$MASTER_FILE
$KUBECTL apply -f $MERINO_DIRECTORY/$SERVICE_FILE
$KUBECTL apply -f $MERINO_DIRECTORY/$WORKER_FILE

echo -e "==================== Verify the Locust deployments & Services"
$KUBECTL get pods -o wide
$KUBECTL get services
