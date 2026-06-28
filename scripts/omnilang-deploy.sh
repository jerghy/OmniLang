#!/bin/bash
# OmniLang Deploy — auto-detect deploy tool and publish to hosting platform.
#
# Supports: Vercel (via npx), Netlify (via npx), manual (zip download)
# Usage:   bash scripts/omnilang-deploy.sh [--platform vercel|netlify|manual]

set -e

PLATFORM="${1:-auto}"
PROJECT_DIR="${2:-.}"
DEPLOY_URL=""

# ── Tool Detection ────────────────────────────────────────

detect_tool() {
    local tool="$1"
    if command -v "$tool" &>/dev/null; then
        echo "$tool"
        return 0
    fi
    return 1
}

detect_npx() {
    if detect_tool "npx"; then
        echo "npx"
    elif detect_tool "npm"; then
        echo "npm exec"
    elif detect_tool "yarn"; then
        echo "yarn dlx"
    elif detect_tool "pnpm"; then
        echo "pnpm dlx"
    else
        echo ""
    fi
}

# ── Platform Deploy ────────────────────────────────────────

deploy_vercel() {
    local npx_cmd
    npx_cmd=$(detect_npx)
    if [ -z "$npx_cmd" ]; then
        echo "[FAIL] No Node.js package manager found (npx/npm/yarn/pnpm)."
        echo "       Install Node.js: https://nodejs.org"
        return 1
    fi

    echo "[...] Deploying to Vercel via $npx_cmd..."
    cd "$PROJECT_DIR"

    if [ "$npx_cmd" = "npx" ]; then
        $npx_cmd vercel --prod --confirm --no-clipboard 2>&1 | tee /tmp/omnilang-vercel.log
    else
        $npx_cmd vercel@latest --prod --confirm --no-clipboard 2>&1 | tee /tmp/omnilang-vercel.log
    fi

    # Extract URL from output
    DEPLOY_URL=$(grep -oP 'https://[^\s]+\.vercel\.app' /tmp/omnilang-vercel.log | head -1)
    if [ -n "$DEPLOY_URL" ]; then
        echo "[OK] Deployed: $DEPLOY_URL"
    fi
}

deploy_netlify() {
    local npx_cmd
    npx_cmd=$(detect_npx)
    if [ -z "$npx_cmd" ]; then
        echo "[FAIL] No Node.js package manager found."
        return 1
    fi

    echo "[...] Deploying to Netlify via $npx_cmd..."
    cd "$PROJECT_DIR"

    # Create netlify.toml if not exists
    if [ ! -f netlify.toml ]; then
        cat > netlify.toml <<'EOF'
[build]
  publish = "dist"
  command = "npm run build || true"
EOF
    fi

    if [ "$npx_cmd" = "npx" ]; then
        $npx_cmd netlify-cli deploy --prod --dir=dist 2>&1 | tee /tmp/omnilang-netlify.log
    else
        $npx_cmd netlify-cli@latest deploy --prod --dir=dist 2>&1 | tee /tmp/omnilang-netlify.log
    fi
}

deploy_manual() {
    echo "============================================"
    echo "  Manual Deploy — Your project is ready."
    echo "============================================"
    echo ""
    echo "  Project dir: $PROJECT_DIR"
    echo ""
    echo "  To deploy, upload these files to any static host:"
    echo ""
    if [ -d "$PROJECT_DIR/dist" ]; then
        ls -la "$PROJECT_DIR/dist/"
        echo ""
        echo "  Example: npx serve $PROJECT_DIR/dist"
        echo "  Example: rsync -av $PROJECT_DIR/dist/ user@host:/var/www/"
    elif [ -d "$PROJECT_DIR/build_output" ]; then
        ls -la "$PROJECT_DIR/build_output/"
        echo ""
        echo "  Build output is in: $PROJECT_DIR/build_output/"
    else
        echo "  (No build output found. Run the compiler first.)"
    fi
    echo ""
    echo "  Supported free static hosts:"
    echo "    - Vercel:    drag-and-drop at vercel.com/new"
    echo "    - Netlify:   drag-and-drop at app.netlify.com/drop"
    echo "    - Cloudflare Pages:  git push to deploy"
    echo "    - GitHub Pages:      push to gh-pages branch"
    echo "    - Surge:     npx surge ./dist"
    echo ""

    # Offer surge as a quick option
    local npx_cmd
    npx_cmd=$(detect_npx)
    if [ -n "$npx_cmd" ] && [ -d "$PROJECT_DIR/dist" ]; then
        echo "  Quick deploy with Surge:"
        echo "    cd $PROJECT_DIR && $npx_cmd surge dist"
    fi
}

# ── Main ───────────────────────────────────────────────────

echo "OmniLang Deploy"
echo "  Platform: ${PLATFORM}"
echo "  Project:  ${PROJECT_DIR}"
echo ""

case "$PLATFORM" in
    vercel)
        deploy_vercel
        ;;
    netlify)
        deploy_netlify
        ;;
    manual|auto)
        # Auto-detect: try Vercel, fall back to manual
        if detect_npx >/dev/null 2>&1; then
            echo "[...] Auto-detecting deploy platform..."
            deploy_vercel || deploy_manual
        else
            echo "[INFO] No Node.js package manager found — manual deploy."
            deploy_manual
        fi
        ;;
    *)
        echo "Unknown platform: $PLATFORM"
        echo "Usage: $0 [vercel|netlify|manual] [project-dir]"
        exit 1
        ;;
esac

if [ -n "$DEPLOY_URL" ]; then
    echo ""
    echo "  Live URL: $DEPLOY_URL"
fi
