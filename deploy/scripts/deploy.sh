#!/bin/bash
set -euo pipefail

echo "=== AmanClaw Cloud — Deploy ==="

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
K3S_DIR="$SCRIPT_DIR/../k3s"

# Apply manifests in order
kubectl apply -f "$K3S_DIR/namespace.yaml"
kubectl apply -f "$K3S_DIR/secret.yaml"
kubectl apply -f "$K3S_DIR/pvc.yaml"
kubectl apply -f "$K3S_DIR/deployment.yaml"
kubectl apply -f "$K3S_DIR/service.yaml"
kubectl apply -f "$K3S_DIR/ingress.yaml"
kubectl apply -f "$K3S_DIR/backup-cronjob.yaml"

echo "Waiting for deployment..."
kubectl rollout status deployment/cloud-server -n amanclaw-cloud --timeout=120s

echo "=== Deployment complete ==="
echo "Cloud server: https://cloud.amanclaw.my"
kubectl get pods -n amanclaw-cloud
