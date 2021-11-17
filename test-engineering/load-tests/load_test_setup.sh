#!/bin/bash

#Setup MacOS - Install Cloud SDK
echo -e "==================== Do you want to install GCP SDK on macOS? [yes or no]: "
read response
case $response in
    [yY] | [yY][Ee][Ss] )
        echo -e "==================== Install latest Cloud SDK version "
        /usr/local/bin/wget https://dl.google.com/dl/cloudsdk/channels/rapid/downloads/google-cloud-sdk-364.0.0-darwin-x86_64.tar.gz -P $DIRECTORY
        tar -xf $DIRECTORY/google-cloud-sdk-364.0.0-darwin-x86_64.tar.gz
        echo -e "==================== Do clean-up "
        rm -rf $DIRECTORY/google-cloud-sdk-364.0.0-darwin-x86_64.tar.gz
        echo -e "==================== Install SDK "
        ./$SDK_DIRECTORY/install.sh
        ;;
    [nN] | [n|N][O|o] )
        echo -e "==================== Skip GCP SDK Installation "
        ;;
    *) echo -e "==================== Invalid input it should be [yes or no] "
        ;;
esac

#Setup environment
GCLOUD=$(which gcloud)
SED=$(which sed)
KUBECTL=$(which kubectl)

#RED_OUPUT='\033[0;31m'
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
KINTO_SERVER_URL=firefox.settings.services.mozilla.com
KINTO_COLLECTION=quicksuggest
LOCUST_MASTER_NODE_HOST=locust-master
KINTO_BUCKET=main
CLUSTER=merino-load-test
TARGET=stage.merino.nonprod.cloudops.mozgcp.net
SCOPE="https://www.googleapis.com/auth/cloud-platform"
REGION=us-central1
ZONE=${REGION}-b
PROJECT=spheric-keel-331521

#Configure Kubernetes
echo -e "==================== Prepare environments with set of environment variables "
echo -e "==================== Set Kubernetes Cluster "
export CLUSTER=$CLUSTER
echo -e "==================== Set Kubernetes TARGET "
export TARGET=$TARGET
echo -e "==================== Set SCOPE "
export SCOPE=$SCOPE

echo -e "==================== Prepare GCP by updating default zone and project ID "
$GCLOUD config set compute/zone $ZONE
$GCLOUD config set project $PROJECT

echo -e "==================== Refresh Kubeconfig at path ~/.kube/config "
$GCLOUD container clusters get-credentials $CLUSTER --zone $ZONE --project $PROJECT

##Build Docker Images
echo -e "==================== Build the Docker image and store it in your project's container registry. Tag with the latest commit hash "
$GCLOUD builds submit --tag gcr.io/$PROJECT/locust-tasks:$LOCUST_IMAGE_TAG
echo -e "==================== Verify that the Docker image is in your project's container repository"
$GCLOUD container images list | grep locust-tasks

##Deploying the Locust master and worker nodes
echo -e "==================== Update Kubernetes Manifests "
echo -e "==================== Replace the target host and project ID with the deployed endpoint and project ID in the locust-master-controller.yaml and locust-worker-controller.yaml files"
$SED -i -e "s/\[TARGET_HOST\]/$TARGET/g" $MERINO_DIRECTORY/$MASTER_FILE
$SED -i -e "s/\[TARGET_HOST\]/$TARGET/g" $MERINO_DIRECTORY/$WORKER_FILE
$SED -i -e "s/\[PROJECT_ID\]/$PROJECT/g" $MERINO_DIRECTORY/$MASTER_FILE
$SED -i -e "s/\[PROJECT_ID\]/$PROJECT/g" $MERINO_DIRECTORY/$WORKER_FILE
$SED -i -e "s/\[LOCUST_IMAGE_TAG\]/$LOCUST_IMAGE_TAG/g" $MERINO_DIRECTORY/$MASTER_FILE

echo "==================== Clean Up "
rm MERINO_DIRECTORY/$MASTER_FILE-*

##Deploy the Locust master and worker nodes using Kubernetes Manifests
echo -e "==================== Deploy the Locust master and worker nodes"
$KUBECTL apply -f $MERINO_DIRECTORY/$MASTER_FILE
$KUBECTL apply -f $MERINO_DIRECTORY/$SERVICE_FILE
$KUBECTL apply -f $MERINO_DIRECTORY/$WORKER_FILE

echo -e "==================== Verify the Locust deployments & Services"
$KUBECTL get pods -o wide
$KUBECTL get services
sleep 10s

EXTERNAL_IP=$($KUBECTL get svc locust-master -o jsonpath="{.status.loadBalancer.ingress[0].ip}")

echo -e "==================== Get the external IP address of the system: $EXTERNAL_IP"
echo -e "==================== Open your browser and then open the Locust master web interface http://$EXTERNAL_IP:8089"