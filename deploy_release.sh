#!/bin/bash

# Run necessary setup scripts
./scripts/update_cargo.toml.sh
./scripts/make_public.sh

# Get the current branch name
current_branch=$(git rev-parse --abbrev-ref HEAD)

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

# Force update release branch with current branch contents
echo "Making release branch identical to $current_branch (will overwrite release)..."
git checkout -B release || { echo "Failed to switch to release branch"; exit 1; }
echo "release branch now matches $current_branch."

# Force push to release branch
echo "Force pushing to release branch..."
git push -f origin release || { echo "Push failed"; exit 1; }
echo "Force push successful - release branch now exactly matches $current_branch."

# Return to original branch
echo "Returning to $current_branch branch..."
git checkout "$current_branch"

echo "release deployment complete!"