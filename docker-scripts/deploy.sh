#!/bin/bash

# Docker deployment script for tobelog
# Usage: ./docker-scripts/deploy.sh [environment]
# Environments: dev, staging, production (default: dev)

set -e

ENVIRONMENT=${1:-dev}
PROJECT_NAME="tobelog"
COMPOSE_FILE="docker-compose.yml"
OVERRIDE_FILE="docker-compose.${ENVIRONMENT}.yml"

echo "Deploying ${PROJECT_NAME} to ${ENVIRONMENT} environment..."

# Check if environment-specific compose file exists
if [ ! -f "${OVERRIDE_FILE}" ]; then
  echo "Error: ${OVERRIDE_FILE} not found!"
  echo "Available environments:"
  ls docker-compose.*.yml | sed 's/docker-compose\.\(.*\)\.yml/  \1/'
  exit 1
fi

# Check if .env file exists
if [ ! -f ".env" ]; then
  echo "Warning: .env file not found. Using environment variables."
  echo "Consider copying .env.example to .env and filling in values."
fi

# Stop existing containers
echo "Stopping existing containers..."
docker-compose -f "${COMPOSE_FILE}" -f "${OVERRIDE_FILE}" down

# Pull/build latest images
echo "Building/pulling latest images..."
docker-compose -f "${COMPOSE_FILE}" -f "${OVERRIDE_FILE}" build

# Start services
echo "Starting services..."
docker-compose -f "${COMPOSE_FILE}" -f "${OVERRIDE_FILE}" up -d

# Wait for health check
echo "Waiting for services to be healthy..."
TIMEOUT=60
ELAPSED=0
while [ $ELAPSED -lt $TIMEOUT ]; do
  if docker-compose -f "${COMPOSE_FILE}" -f "${OVERRIDE_FILE}" ps | grep -q "healthy"; then
    echo "Service is healthy!"
    break
  fi
  echo "Waiting for health check... (${ELAPSED}s/${TIMEOUT}s)"
  sleep 5
  ELAPSED=$((ELAPSED + 5))
done

if [ $ELAPSED -ge $TIMEOUT ]; then
  echo "Warning: Service did not become healthy within ${TIMEOUT} seconds"
fi

# Check service status
echo "Service status:"
docker-compose -f "${COMPOSE_FILE}" -f "${OVERRIDE_FILE}" ps

# Show logs
echo ""
echo "Recent logs:"
docker-compose -f "${COMPOSE_FILE}" -f "${OVERRIDE_FILE}" logs --tail=20

echo ""
echo "Deployment completed!"
echo "To follow logs: docker-compose -f ${COMPOSE_FILE} -f ${OVERRIDE_FILE} logs -f"
echo "To stop: docker-compose -f ${COMPOSE_FILE} -f ${OVERRIDE_FILE} down"