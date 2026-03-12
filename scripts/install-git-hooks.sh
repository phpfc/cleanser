#!/bin/bash

# Script to install git hooks for the cleanser project
# Run this after cloning the repository

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Installing git hooks...${NC}"

# Get the root directory of the git repository
GIT_ROOT=$(git rev-parse --show-toplevel)
cd "$GIT_ROOT"

# Create hooks directory if it doesn't exist
mkdir -p .git/hooks

# Copy pre-push hook
cat > .git/hooks/pre-push << 'EOF'
#!/bin/bash

# Pre-push hook to run linting and tests before pushing
# This mirrors the CI checks in .github/workflows/ci.yml

set -e

echo "🔍 Running pre-push checks..."
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if we're in a git repository
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    echo -e "${RED}✗ Not in a git repository${NC}"
    exit 1
fi

# Get the root directory of the git repository
GIT_ROOT=$(git rev-parse --show-toplevel)
cd "$GIT_ROOT"

echo -e "${YELLOW}📋 Step 1/3: Checking formatting...${NC}"
if cargo fmt --all -- --check; then
    echo -e "${GREEN}✓ Formatting check passed${NC}"
else
    echo -e "${RED}✗ Formatting check failed${NC}"
    echo -e "${YELLOW}Run 'cargo fmt --all' to fix formatting issues${NC}"
    exit 1
fi
echo ""

echo -e "${YELLOW}🔧 Step 2/3: Running clippy...${NC}"
if cargo clippy --workspace --exclude cleanser-gui --all-targets -- -D warnings; then
    echo -e "${GREEN}✓ Clippy passed${NC}"
else
    echo -e "${RED}✗ Clippy found issues${NC}"
    echo -e "${YELLOW}Fix the clippy warnings above before pushing${NC}"
    exit 1
fi
echo ""

echo -e "${YELLOW}🧪 Step 3/3: Running tests...${NC}"
if cargo test --workspace --exclude cleanser-gui --verbose; then
    echo -e "${GREEN}✓ Tests passed${NC}"
else
    echo -e "${RED}✗ Tests failed${NC}"
    echo -e "${YELLOW}Fix the failing tests before pushing${NC}"
    exit 1
fi
echo ""

echo -e "${GREEN}✅ All pre-push checks passed!${NC}"
echo -e "${GREEN}🚀 Pushing to remote...${NC}"
echo ""

exit 0
EOF

# Make hook executable
chmod +x .git/hooks/pre-push

echo -e "${GREEN}✓ Git hooks installed successfully!${NC}"
echo ""
echo "The following hooks have been installed:"
echo "  - pre-push: Runs formatting, clippy, and tests before pushing"
echo ""
echo "To skip the pre-push hook (not recommended), use: git push --no-verify"
