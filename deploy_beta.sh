#!/bin/bash

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

# Switch to beta and merge
if [ "$current_branch" != "beta" ]; then
    echo "Switching to beta branch and merging from $current_branch..."
    git checkout beta || { echo "Failed to switch to beta branch"; exit 1; }
    git merge "$current_branch" || { echo "Merge failed. Resolve conflicts and try again."; exit 1; }
    echo "Merge successful."
fi

# Push to beta branch
echo "Pushing to beta branch..."
git push origin beta

# Return to original branch
echo "Returning to $current_branch branch..."
git checkout "$current_branch"

echo "Beta deployment complete!"