#!/bin/bash

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

# Switch to release and merge
if [ "$current_branch" != "release" ]; then
    echo "Switching to release branch and merging from $current_branch..."
    git checkout release || { echo "Failed to switch to release branch"; exit 1; }
    git merge "$current_branch" || { echo "Merge failed. Resolve conflicts and try again."; exit 1; }
    echo "Merge successful."
fi

# Push to release branch
echo "Pushing to release branch..."
git push origin release


# Return to the original branch
echo "Returning to $current_branch branch..."
git checkout "$current_branch"

echo "Release deployment complete!"


