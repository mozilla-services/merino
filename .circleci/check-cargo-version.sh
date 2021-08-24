#!/usr/bin/env bash

if [[ $# -ne 1 ]]; then
  echo USAGE $0 cargo-version.txt
  exit 1
fi

CARGO_VERSION_FILE=$1

EXPECTED_CARGO_VERSION=$(cat $CARGO_VERSION_FILE)
ACTUAL_CARGO_VERSION=$(cargo version)

if [[ "$EXPECTED_CARGO_VERSION" = "$ACTUAL_CARGO_VERSION" ]]; then
  echo "Cargo version is the expected value of ${EXPECTED_CARGO_VERSION}"
else
  echo "Error: Expected cargo version ${EXPECTED_CARGO_VERSION} but found ${ACTUAL_CARGO_VERSION}"
  exit 1
fi
