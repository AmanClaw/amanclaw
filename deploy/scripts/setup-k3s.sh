#!/bin/bash
set -euo pipefail

echo "=== AmanClaw Cloud — K3s Setup ==="

# Install K3s
curl -sfL https://get.k3s.io | sh -

# Wait for K3s to be ready
echo "Waiting for K3s..."
until kubectl get nodes >/dev/null 2>&1; do sleep 2; done
echo "K3s is ready!"

# Install cert-manager for Let's Encrypt TLS
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/latest/download/cert-manager.yaml

echo "Waiting for cert-manager..."
kubectl wait --for=condition=Available deployment --all -n cert-manager --timeout=120s

# Create Let's Encrypt ClusterIssuer
cat <<EOF | kubectl apply -f -
apiVersion: cert-manager.io/v1
kind: ClusterIssuer
metadata:
  name: letsencrypt-prod
spec:
  acme:
    server: https://acme-v02.api.letsencrypt.org/directory
    email: admin@amanclaw.my
    privateKeySecretRef:
      name: letsencrypt-prod-key
    solvers:
      - http01:
          ingress:
            class: traefik
EOF

echo "=== K3s setup complete ==="
echo "Next: Update deploy/k3s/secret.yaml with a real JWT secret"
echo "Then: ./deploy/scripts/deploy.sh"
