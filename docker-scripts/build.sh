#!/bin/bash

# Docker build script for tobelog
# Usage: ./docker-scripts/build.sh [environment]
# Environments: dev, staging, production (default: dev)

set -e

ENVIRONMENT=${1:-dev}
PROJECT_NAME="tobelog"
DOCKER_REPO="localhost/${PROJECT_NAME}"
VERSION=$(grep '^version' Cargo.toml | sed 's/version = "\(.*\)"/\1/')

echo "Building ${PROJECT_NAME} Docker image for ${ENVIRONMENT} environment..."
echo "Version: ${VERSION}"

# Build the Docker image
docker build \
  --tag "${DOCKER_REPO}:${VERSION}" \
  --tag "${DOCKER_REPO}:${ENVIRONMENT}" \
  --tag "${DOCKER_REPO}:latest" \
  .

echo "Build completed successfully!"
echo "Tagged images:"
echo "  ${DOCKER_REPO}:${VERSION}"
echo "  ${DOCKER_REPO}:${ENVIRONMENT}"
echo "  ${DOCKER_REPO}:latest"

# Optional: Run basic smoke test
if [ "${ENVIRONMENT}" = "dev" ]; then
  echo ""
  echo "Running basic smoke test..."
  docker run --rm "${DOCKER_REPO}:${VERSION}" /usr/local/bin/tobelog --help
fi