#!/bin/bash
# Script untuk mengaktifkan GitHub Actions workflow
# Jalankan: bash activate-workflow.sh

WORKFLOW_FILE=".github/workflows/release.yml"

if [ -f "$WORKFLOW_FILE" ]; then
    echo "✅ Workflow sudah aktif!"
    exit 0
fi

echo "📋 Mengaktifkan workflow release..."
echo ""

# Buat direktori
mkdir -p .github/workflows

# Copy workflow dari docs
cp docs/workflow-release.yml "$WORKFLOW_FILE"

# Commit dan push
git add "$WORKFLOW_FILE"
git commit -m "Activate GitHub Actions release workflow"

# Push (catatan: butuh PAT dengan workflow scope)
echo "⚠️  Push membutuhkan PAT dengan 'workflows: write' scope"
echo "   Jalankan: git push origin master"
echo ""
echo "   Atau upload manual via GitHub Web UI:"
echo "   1. Buka https://github.com/dens4t/bitwarden-rust"
echo "   2. Klik 'Add file' → 'Upload files'"
echo "   3. Upload file docs/workflow-release.yml"
echo "   4. Rename ke .github/workflows/release.yml"
echo "   5. Commit"
