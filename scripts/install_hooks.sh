#!/bin/bash
# install_hooks.sh

HOOKS_DIR=".git/hooks"
PRE_PUSH_HOOK="$HOOKS_DIR/pre-push"

echo "Installing pre-push hook for ARCHITECTURE.md auto-update..."

cat << 'EOF' > "$PRE_PUSH_HOOK"
#!/bin/bash
# Aiome: Auto-update ARCHITECTURE.md before pushing

echo "📐 Updating ARCHITECTURE.md (Knowledge Map)..."
python3 scripts/generate_architecture.py

git diff --quiet ARCHITECTURE.md
if [ $? -ne 0 ]; then
    echo "⚠️ ARCHITECTURE.md has changed. Automatically committing updates."
    git add ARCHITECTURE.md
    
    # We amend the current commit so we don't spam the commit history with "update docs"
    git commit --amend --no-edit

    echo "✅ ARCHITECTURE.md committed."
fi

exit 0
EOF

chmod +x "$PRE_PUSH_HOOK"
echo "✅ Git pre-push hook installed successfully."
