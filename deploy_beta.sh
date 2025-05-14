#!/bin/bash

# Run necessary setup scripts
./scripts/update_cargo.toml.sh
./scripts/make_public.sh

# Get the current branch name
current_branch=$(git rev-parse --abbrev-ref HEAD)

# Update submodules to the latest commit on their default branch
echo "Updating FFCodex submodule to latest version..."
git submodule update --init --recursive --remote
echo "Submodule updated to latest version."

# Check if there are uncommitted changes
if [[ $(git status --porcelain) ]]; then
  echo "Uncommitted changes detected."
  read -p "Stage all changes? (y/n): " stage
  if [[ $stage == "y" || $stage == "Y" ]]; then
    git add .
    echo "Changes staged."
  fi
  
  # Commit staged work
  read -p "Enter commit message: " msg
  git commit -m "$msg"
fi

# Force update beta branch with current branch contents
echo "Making beta branch identical to $current_branch (will overwrite beta)..."
git checkout -B beta || { echo "Failed to switch to beta branch"; exit 1; }
echo "Beta branch now matches $current_branch."

# Force push to beta branch
echo "Force pushing to beta branch..."
git push -f origin beta || { echo "Push failed"; exit 1; }
echo "Force push successful - beta branch now exactly matches $current_branch."

# Return to original branch
echo "Returning to $current_branch branch..."
git checkout "$current_branch"

echo "Beta deployment complete!"

./scripts/version_up.sh

# Schedule make_private.sh to run after 20 minutes
(sleep 1200 && ./scripts/make_private.sh) &
echo "Repository will be set back to private in 20 minutes"