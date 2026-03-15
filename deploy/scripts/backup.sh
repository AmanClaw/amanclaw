#!/bin/bash
set -euo pipefail
echo "=== Manual Backup ==="
kubectl create job --from=cronjob/backup manual-backup-$(date +%s) -n amanclaw-cloud
echo "Backup job created. Check with: kubectl get jobs -n amanclaw-cloud"
