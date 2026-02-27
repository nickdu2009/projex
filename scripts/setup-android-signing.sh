#!/usr/bin/env bash
# =============================================================================
# setup-android-signing.sh
#
# One-shot script to:
#   1. Generate an Android release keystore (or use an existing one)
#   2. Write credentials to android-signing.env (gitignored)
#   3. Optionally set the four GitHub Actions secrets via gh CLI
#
# Prerequisites:
#   - keytool  (bundled with JDK; install via: brew install --cask temurin)
#   - gh       (GitHub CLI, optional; install via: brew install gh && gh auth login)
#
# Usage:
#   chmod +x scripts/setup-android-signing.sh
#   ./scripts/setup-android-signing.sh
#
# Environment overrides:
#   KEYSTORE_PATH   Path to an existing .jks file (skip generation)
#   KEYSTORE_OUT    Output path for a newly generated keystore
#                   (default: <repo-root>/secret/projex-release.jks)
#   ENV_OUT         Output path for the .env file
#                   (default: <repo-root>/secret/android-signing.env)
#   SKIP_GH         Set to "1" to skip GitHub Secrets upload
#
# Examples:
#   # Use existing keystore, write env file only (no gh upload):
#   KEYSTORE_PATH=secret/projex-release.jks SKIP_GH=1 ./scripts/setup-android-signing.sh
#
#   # Generate new keystore to custom path and upload secrets:
#   KEYSTORE_OUT=~/my-keys/projex.jks ./scripts/setup-android-signing.sh
# =============================================================================

set -euo pipefail

# ── Colour helpers ────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
CYAN='\033[0;36m'; BOLD='\033[1m'; RESET='\033[0m'

info()    { echo -e "${CYAN}[INFO]${RESET}  $*"; }
success() { echo -e "${GREEN}[OK]${RESET}    $*"; }
warn()    { echo -e "${YELLOW}[WARN]${RESET}  $*"; }
error()   { echo -e "${RED}[ERROR]${RESET} $*" >&2; }
step()    { echo -e "\n${BOLD}── $* ──${RESET}"; }

# ── Resolve repo root ─────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# ── Paths ─────────────────────────────────────────────────────────────────────
KEYSTORE_PATH="${KEYSTORE_PATH:-}"
KEYSTORE_OUT="${KEYSTORE_OUT:-$REPO_ROOT/secret/projex-release.jks}"
# Ensure the secret directory exists with restricted permissions
mkdir -p "$REPO_ROOT/secret"
chmod 700 "$REPO_ROOT/secret"
ENV_OUT="${ENV_OUT:-$REPO_ROOT/secret/android-signing.env}"
SKIP_GH="${SKIP_GH:-0}"
KEY_ALIAS="projex"

# ── Dependency checks ─────────────────────────────────────────────────────────
step "Checking prerequisites"

if ! command -v keytool &>/dev/null; then
    error "keytool not found. Install JDK: brew install --cask temurin"
    exit 1
fi
success "keytool found"

GH_AVAILABLE=0
if command -v gh &>/dev/null && gh auth status &>/dev/null 2>&1; then
    GH_AVAILABLE=1
    success "gh authenticated as: $(gh api user --jq '.login')"
else
    warn "GitHub CLI (gh) not available or not authenticated."
    warn "Secrets will NOT be uploaded automatically."
    warn "You can upload manually using the generated $ENV_OUT file."
    SKIP_GH=1
fi

# ── Keystore: use existing or generate new ────────────────────────────────────
step "Keystore"

if [ -n "$KEYSTORE_PATH" ]; then
    # Resolve relative to repo root if not absolute
    if [[ "$KEYSTORE_PATH" != /* ]]; then
        KEYSTORE_PATH="$REPO_ROOT/$KEYSTORE_PATH"
    fi
    if [ ! -f "$KEYSTORE_PATH" ]; then
        error "Specified KEYSTORE_PATH does not exist: $KEYSTORE_PATH"
        exit 1
    fi
    info "Using existing keystore: $KEYSTORE_PATH"
    read -r -s -p "  Enter STORE password: " STORE_PASS; echo
    read -r -s -p "  Enter KEY   password: " KEY_PASS;   echo
    # Verify the password is correct
    if ! keytool -list -keystore "$KEYSTORE_PATH" \
            -storepass "$STORE_PASS" &>/dev/null; then
        error "Incorrect store password for $KEYSTORE_PATH"
        exit 1
    fi
    success "Keystore password verified"

elif [ -f "$KEYSTORE_OUT" ]; then
    warn "Keystore already exists at $KEYSTORE_OUT"
    read -r -p "  Re-use existing keystore? [Y/n] " REUSE
    REUSE="${REUSE:-Y}"
    if [[ "$REUSE" =~ ^[Yy]$ ]]; then
        KEYSTORE_PATH="$KEYSTORE_OUT"
        info "Using existing keystore."
        read -r -s -p "  Enter STORE password: " STORE_PASS; echo
        read -r -s -p "  Enter KEY   password: " KEY_PASS;   echo
        if ! keytool -list -keystore "$KEYSTORE_PATH" \
                -storepass "$STORE_PASS" &>/dev/null; then
            error "Incorrect store password"
            exit 1
        fi
        success "Keystore password verified"
    else
        KEYSTORE_PATH=""
    fi
fi

if [ -z "$KEYSTORE_PATH" ]; then
    echo ""
    echo -e "${BOLD}Enter keystore details (identifies the app publisher):${RESET}"
    read -r -p "  First and last name  [Nick Du]:    " DNAME_CN;  DNAME_CN="${DNAME_CN:-Nick Du}"
    read -r -p "  Organisation unit    [Personal]:   " DNAME_OU;  DNAME_OU="${DNAME_OU:-Personal}"
    read -r -p "  Organisation         [Projex]:     " DNAME_O;   DNAME_O="${DNAME_O:-Projex}"
    read -r -p "  City / Locality      [Shanghai]:   " DNAME_L;   DNAME_L="${DNAME_L:-Shanghai}"
    read -r -p "  State / Province     [Shanghai]:   " DNAME_ST;  DNAME_ST="${DNAME_ST:-Shanghai}"
    read -r -p "  Country code (2-letter) [CN]:      " DNAME_C;   DNAME_C="${DNAME_C:-CN}"

    echo ""
    while true; do
        read -r -s -p "  Set STORE password (≥6 chars): " STORE_PASS; echo
        read -r -s -p "  Confirm STORE password:        " STORE_PASS2; echo
        [ "$STORE_PASS" = "$STORE_PASS2" ] && break
        warn "Passwords do not match. Try again."
    done

    echo ""
    read -r -p "  Use same password for KEY? [Y/n] " SAME_PASS
    SAME_PASS="${SAME_PASS:-Y}"
    if [[ "$SAME_PASS" =~ ^[Yy]$ ]]; then
        KEY_PASS="$STORE_PASS"
    else
        while true; do
            read -r -s -p "  Set KEY password (≥6 chars): " KEY_PASS; echo
            read -r -s -p "  Confirm KEY password:        " KEY_PASS2; echo
            [ "$KEY_PASS" = "$KEY_PASS2" ] && break
            warn "Passwords do not match. Try again."
        done
    fi

    DNAME="CN=${DNAME_CN}, OU=${DNAME_OU}, O=${DNAME_O}, L=${DNAME_L}, ST=${DNAME_ST}, C=${DNAME_C}"
    KEYSTORE_PATH="$KEYSTORE_OUT"

    info "Generating keystore at $KEYSTORE_PATH ..."
    keytool -genkey -v \
        -keystore "$KEYSTORE_PATH" \
        -alias "$KEY_ALIAS" \
        -keyalg RSA \
        -keysize 2048 \
        -validity 10000 \
        -storepass "$STORE_PASS" \
        -keypass "$KEY_PASS" \
        -dname "$DNAME" \
        -noprompt

    chmod 600 "$KEYSTORE_PATH"
    success "Keystore created: $KEYSTORE_PATH"
fi

# ── Encode keystore ───────────────────────────────────────────────────────────
step "Encoding keystore to base64"

KEYSTORE_B64=$(base64 -i "$KEYSTORE_PATH")
success "Encoded (${#KEYSTORE_B64} chars)"

# ── Write env file ────────────────────────────────────────────────────────────
step "Writing credentials to $ENV_OUT"

cat > "$ENV_OUT" <<EOF
# Android signing credentials — generated by setup-android-signing.sh
# ⚠️  This file contains secrets. It is gitignored. DO NOT commit it.
#
# To upload to GitHub manually:
#   gh secret set ANDROID_KEYSTORE_BASE64  --body "\$(grep ANDROID_KEYSTORE_BASE64 android-signing.env | cut -d= -f2-)"
#   gh secret set ANDROID_STORE_PASSWORD   --body "\$(grep ANDROID_STORE_PASSWORD  android-signing.env | cut -d= -f2-)"
#   gh secret set ANDROID_KEY_ALIAS        --body "\$(grep ANDROID_KEY_ALIAS       android-signing.env | cut -d= -f2-)"
#   gh secret set ANDROID_KEY_PASSWORD     --body "\$(grep ANDROID_KEY_PASSWORD    android-signing.env | cut -d= -f2-)"
#
# To use for local signed builds:
#   source android-signing.env
#   export ANDROID_KEYSTORE_PATH="$KEYSTORE_PATH"
#   cd src-tauri/gen/android && ./gradlew assembleRelease

ANDROID_KEYSTORE_BASE64=$KEYSTORE_B64
ANDROID_STORE_PASSWORD=$STORE_PASS
ANDROID_KEY_ALIAS=$KEY_ALIAS
ANDROID_KEY_PASSWORD=$KEY_PASS
ANDROID_KEYSTORE_PATH=$KEYSTORE_PATH
EOF

chmod 600 "$ENV_OUT"
success "Written: $ENV_OUT  (permissions: 600)"

# ── Set GitHub Secrets ────────────────────────────────────────────────────────
if [ "$SKIP_GH" = "1" ]; then
    step "Skipping GitHub Secrets upload (SKIP_GH=1 or gh not available)"
    info "Upload manually with the commands printed in $ENV_OUT"
else
    REPO=$(gh repo view --json nameWithOwner --jq '.nameWithOwner' 2>/dev/null || true)
    if [ -z "$REPO" ]; then
        error "Cannot detect GitHub repository. Run from inside the repo directory."
        exit 1
    fi

    step "Setting GitHub Actions Secrets → $REPO"

    set_secret() {
        local name="$1"
        local value="$2"
        echo -n "  Setting $name ... "
        if echo "$value" | gh secret set "$name" --repo "$REPO"; then
            echo -e "${GREEN}✓${RESET}"
        else
            echo -e "${RED}✗${RESET}"
            error "Failed to set secret $name"
            exit 1
        fi
    }

    set_secret "ANDROID_KEYSTORE_BASE64" "$KEYSTORE_B64"
    set_secret "ANDROID_STORE_PASSWORD"  "$STORE_PASS"
    set_secret "ANDROID_KEY_ALIAS"       "$KEY_ALIAS"
    set_secret "ANDROID_KEY_PASSWORD"    "$KEY_PASS"

    success "All 4 GitHub Secrets configured for $REPO"
fi

# ── Summary ───────────────────────────────────────────────────────────────────
step "Done"

echo -e "  Keystore:   ${CYAN}$KEYSTORE_PATH${RESET}"
echo -e "  Env file:   ${CYAN}$ENV_OUT${RESET}"
echo ""
echo -e "${BOLD}${YELLOW}⚠️  Back up your keystore — losing it means you cannot update existing installs:${RESET}"
echo -e "   ${CYAN}cp $KEYSTORE_PATH ~/Library/CloudStorage/  # or any secure location${RESET}"
echo ""
echo -e "${BOLD}Next steps:${RESET}"
if [ "$SKIP_GH" = "1" ]; then
    echo -e "   1. Upload secrets to GitHub (see commands in $ENV_OUT)"
    echo -e "   2. Push a version tag:"
else
    echo -e "   1. Push a version tag:"
fi
echo -e "      ${CYAN}git tag v1.0.5 && git push origin v1.0.5${RESET}"
echo ""
