#!/usr/bin/env bash
# =============================================================================
# setup-android-signing.sh
#
# One-shot script to:
#   1. Generate an Android release keystore (projex-release.jks)
#   2. Set the four GitHub Actions secrets required for signed APK builds
#
# Prerequisites:
#   - keytool  (bundled with JDK; install via: brew install --cask temurin)
#   - gh       (GitHub CLI; install via: brew install gh && gh auth login)
#
# Usage:
#   chmod +x scripts/setup-android-signing.sh
#   ./scripts/setup-android-signing.sh
#
# The keystore file is saved to ~/.projex/projex-release.jks by default.
# Override with: KEYSTORE_OUT=/path/to/file ./scripts/setup-android-signing.sh
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

# ── Dependency checks ─────────────────────────────────────────────────────────
step "Checking prerequisites"

if ! command -v keytool &>/dev/null; then
    error "keytool not found. Install JDK: brew install --cask temurin"
    exit 1
fi
success "keytool found: $(keytool -help 2>&1 | head -1 || true)"

if ! command -v gh &>/dev/null; then
    error "GitHub CLI (gh) not found. Install: brew install gh && gh auth login"
    exit 1
fi

if ! gh auth status &>/dev/null; then
    error "GitHub CLI not authenticated. Run: gh auth login"
    exit 1
fi
success "gh authenticated as: $(gh api user --jq '.login')"

# ── Configuration ─────────────────────────────────────────────────────────────
step "Configuration"

KEYSTORE_DIR="${KEYSTORE_OUT:-$HOME/.projex}"
KEYSTORE_FILE="$KEYSTORE_DIR/projex-release.jks"
KEY_ALIAS="projex"

# Detect GitHub repo from git remote
REPO=$(gh repo view --json nameWithOwner --jq '.nameWithOwner' 2>/dev/null || true)
if [ -z "$REPO" ]; then
    error "Cannot detect GitHub repository. Run this script from inside the repo directory."
    exit 1
fi
info "Target repository: $REPO"
info "Keystore output:   $KEYSTORE_FILE"
info "Key alias:         $KEY_ALIAS"

# ── Keystore generation ───────────────────────────────────────────────────────
step "Keystore"

mkdir -p "$KEYSTORE_DIR"
chmod 700 "$KEYSTORE_DIR"

if [ -f "$KEYSTORE_FILE" ]; then
    warn "Keystore already exists at $KEYSTORE_FILE"
    read -r -p "  Re-use existing keystore? [Y/n] " REUSE
    REUSE="${REUSE:-Y}"
    if [[ "$REUSE" =~ ^[Yy]$ ]]; then
        info "Using existing keystore."
        # Still need passwords for the existing keystore
        read -r -s -p "  Enter STORE password for existing keystore: " STORE_PASS; echo
        read -r -s -p "  Enter KEY  password for existing keystore: " KEY_PASS;   echo
    else
        info "Generating a new keystore (will overwrite)."
        REUSE="N"
    fi
fi

if [ ! -f "$KEYSTORE_FILE" ] || [[ "${REUSE:-N}" =~ ^[Nn]$ ]]; then
    echo ""
    echo -e "${BOLD}Enter keystore details (used to identify the app publisher):${RESET}"
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

    info "Generating keystore..."
    keytool -genkey -v \
        -keystore "$KEYSTORE_FILE" \
        -alias "$KEY_ALIAS" \
        -keyalg RSA \
        -keysize 2048 \
        -validity 10000 \
        -storepass "$STORE_PASS" \
        -keypass "$KEY_PASS" \
        -dname "$DNAME" \
        -noprompt

    chmod 600 "$KEYSTORE_FILE"
    success "Keystore created: $KEYSTORE_FILE"
fi

# ── Encode keystore ───────────────────────────────────────────────────────────
step "Encoding keystore to base64"

KEYSTORE_B64=$(base64 -i "$KEYSTORE_FILE")
success "Encoded (${#KEYSTORE_B64} chars)"

# ── Set GitHub Secrets ────────────────────────────────────────────────────────
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

# ── Backup reminder ───────────────────────────────────────────────────────────
step "Done"

success "All 4 GitHub Secrets configured for $REPO"
echo ""
echo -e "${BOLD}${YELLOW}⚠️  IMPORTANT — Back up your keystore:${RESET}"
echo -e "   File: ${CYAN}$KEYSTORE_FILE${RESET}"
echo -e "   If lost, you cannot publish updates to existing installs."
echo -e "   Recommended: copy to a password manager or encrypted cloud storage."
echo ""
echo -e "${BOLD}Next steps:${RESET}"
echo -e "   Push a version tag to trigger the publish workflow:"
echo -e "   ${CYAN}git tag v1.0.5 && git push origin v1.0.5${RESET}"
echo ""
