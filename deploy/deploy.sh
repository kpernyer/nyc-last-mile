#!/bin/bash
# Deploy Last-Mile MCP Server to Google Cloud Run
#
# Prerequisites:
#   - Google Cloud CLI installed and authenticated
#   - Docker installed
#   - Domain verified in Google Search Console
#
# Usage:
#   ./deploy/deploy.sh <PROJECT_ID> [REGION]
#
# Example:
#   ./deploy/deploy.sh my-gcp-project us-central1

set -e

PROJECT_ID=${1:?'Usage: deploy.sh <PROJECT_ID> [REGION]'}
REGION=${2:-us-central1}
SERVICE_NAME="lastmile-mcp"
IMAGE_NAME="gcr.io/${PROJECT_ID}/${SERVICE_NAME}"
DOMAIN="logistic.hey.sh"

echo "=============================================="
echo "Last-Mile MCP Server Deployment"
echo "=============================================="
echo "Project:  ${PROJECT_ID}"
echo "Region:   ${REGION}"
echo "Service:  ${SERVICE_NAME}"
echo "Domain:   ${DOMAIN}"
echo ""

# Set project
echo ">>> Setting GCP project..."
gcloud config set project ${PROJECT_ID}

# Enable required APIs
echo ">>> Enabling required APIs..."
gcloud services enable \
    cloudbuild.googleapis.com \
    run.googleapis.com \
    containerregistry.googleapis.com

# Build and push container image
echo ">>> Building container image..."
gcloud builds submit \
    --tag ${IMAGE_NAME}:latest \
    --timeout=30m

# Deploy to Cloud Run
echo ">>> Deploying to Cloud Run..."
gcloud run deploy ${SERVICE_NAME} \
    --image ${IMAGE_NAME}:latest \
    --region ${REGION} \
    --platform managed \
    --allow-unauthenticated \
    --port 8080 \
    --memory 2Gi \
    --cpu 2 \
    --min-instances 0 \
    --max-instances 10 \
    --timeout 300

# Get service URL
SERVICE_URL=$(gcloud run services describe ${SERVICE_NAME} \
    --region ${REGION} \
    --format 'value(status.url)')

echo ""
echo ">>> Service deployed at: ${SERVICE_URL}"
echo ""

# Map custom domain
echo ">>> Mapping custom domain: ${DOMAIN}"
echo ""
echo "NOTE: Before running domain mapping, ensure you have:"
echo "  1. Verified domain ownership in Google Search Console"
echo "  2. Access to update DNS records for ${DOMAIN}"
echo ""
read -p "Continue with domain mapping? (y/n) " -n 1 -r
echo ""

if [[ $REPLY =~ ^[Yy]$ ]]; then
    gcloud run domain-mappings create \
        --service ${SERVICE_NAME} \
        --domain ${DOMAIN} \
        --region ${REGION} \
        || echo "Domain mapping may already exist or need verification"

    echo ""
    echo ">>> DNS Configuration Required:"
    echo ""
    echo "Add the following DNS record to your domain registrar:"
    echo ""
    echo "  Type:  CNAME"
    echo "  Name:  logistic (or @ for apex)"
    echo "  Value: ghs.googlehosted.com."
    echo ""
    echo "Or for apex domain, add these A records:"
    echo "  216.239.32.21"
    echo "  216.239.34.21"
    echo "  216.239.36.21"
    echo "  216.239.38.21"
    echo ""
fi

echo "=============================================="
echo "Deployment Complete!"
echo "=============================================="
echo ""
echo "Service URL: ${SERVICE_URL}"
echo "Custom URL:  https://${DOMAIN} (after DNS propagation)"
echo ""
echo "Test the service:"
echo "  curl ${SERVICE_URL}/health"
echo ""
echo "MCP endpoint:"
echo "  POST ${SERVICE_URL}/mcp"
echo ""
