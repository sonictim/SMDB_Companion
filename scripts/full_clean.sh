#!/bin/bash
# Clean up existing node_modules
rm -rf node_modules
rm -rf src-tauri/target

# Install specific compatible versions
pnpm install svelte@3.59.2 --save-exact
pnpm install @sveltejs/vite-plugin-svelte@2.4.6 --save-exact
pnpm install vite@4.4.11 --save-exact
pnpm install svelte-virtual-list@3.0.1 --save-exact

# Force reinstall all dependencies 
pnpm install --force
