#!/bin/bash
# ci.sh - Local CI testing

act \
  --container-architecture linux/amd64 \
  -W .github/workflows/ci.yaml \
  -P ubuntu-latest=catthehacker/ubuntu:act-latest \
  --artifact-server-path /tmp/artifacts \
  "$@"
