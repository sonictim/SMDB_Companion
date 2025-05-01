console.log('Loading module:', 'utils.ts');  // Add to each file

  import { invoke } from "@tauri-apps/api/core";
import { getAllWindows, Window } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { writable } from 'svelte/store';

import { listen } from "@tauri-apps/api/event";
import { applyColors } from "./colors";
import { preferencesStore } from "./preferences";
import type { Colors } from "./types";
import { get } from "svelte/store";
  import { onMount, onDestroy } from "svelte";



// For localStorage (persistent between sessions)
export function createLocalStore<T>(key: string, initialValue: T) {
  let storedValue: T;
  try {
    const stored = localStorage.getItem(key);
    storedValue = stored ? JSON.parse(stored) : initialValue;
  } catch (e) {
    console.error(`Error loading ${key} from localStorage:`, e);
    storedValue = initialValue;
  }
  
  const store = writable<T>(storedValue);
  
  // Subscribe to save changes to sessionStorage
  store.subscribe(value => {
    localStorage.setItem(key, JSON.stringify(value));
  });
  
  return store;
}


// For sessionStorage (cleared when session ends)
export function createSessionStore<T>(key: string, initialValue: T) {
  let storedValue: T;
  try {
    const stored = sessionStorage.getItem(key);
    storedValue = stored ? JSON.parse(stored) : initialValue;
  } catch (e) {
    console.error(`Error loading ${key} from sessionStorage:`, e);
    storedValue = initialValue;
  }
  
  const store = writable<T>(storedValue);
  
  // Subscribe to save changes to sessionStorage
  store.subscribe(value => {
    try {
      sessionStorage.setItem(key, JSON.stringify(value));
    } catch (e) {
      if (e instanceof Error && e.name === 'QuotaExceededError') {
        console.warn(`Storage quota exceeded for ${key}. Data will not be persisted.`);
        // Optionally implement fallback behavior here
      } else {
        console.error(`Error saving ${key} to sessionStorage:`, e);
      }
    }
  });
  
  return store;
}

// stores/migration.ts
export function migrateStores() {
  // Move data from old locations to new if needed
  try {
    // Example: Moving from old to new structure
    const oldResults = sessionStorage.getItem('oldResultsKey');
    if (oldResults && !sessionStorage.getItem('results')) {
      sessionStorage.setItem('results', oldResults);
      sessionStorage.removeItem('oldResultsKey');
    }
  } catch (e) {
    console.error('Store migration error:', e);
  }
}


export async function checkForUpdates(): Promise<{
    latest: string;
    needsUpdate: boolean;
    versionChanges: string;
    isBeta: boolean;
}> {
      let isBeta = false;
    try {
      const response = await fetch(
        "https://smdbc.com/latest.php?token=how-cool-am-i"
      );
      if (!response.ok) throw new Error(`HTTP error: ${response.status}`);
      const response_beta = await fetch(
        "https://smdbc.com/latest_beta.php?token=how-cool-am-i"
      );
      if (!response_beta.ok) throw new Error(`HTTP error: ${response.status}`);
      const response_changelog = await fetch(
        "https://smdbc.com/changelog.php?token=how-cool-am-i"
      );
      if (!response_beta.ok) throw new Error(`HTTP error: ${response.status}`);

      let latestVersion = await response.text();
      const currentVersion: string = await invoke("get_current_version");
      if (!currentVersion.endsWith(".0")) {
        isBeta = true;
        // If the current version ends with .0, use the beta version
        latestVersion = await response_beta.text();
      }

      const fullLog = await response_changelog.text();
      let versionChanges = parseChangelog(fullLog, currentVersion);

      console.log("Latest version:", latestVersion);
      console.log("Current version:", currentVersion);

      // Compare versions (you can use semver-parser in frontend)
      const needsUpdate =
        compareVersions(latestVersion.trim(), currentVersion as string) > 0;

      return {
        latest: latestVersion.trim(),
        needsUpdate,
        versionChanges,
        isBeta,

      };
    } catch (error) {
      console.error("Failed to check for updates:", error);
      throw error;
    }
  }


  // Helper function to parse changelog and extract relevant changes
  function parseChangelog(changelogJson: string, currentVersion: string) {
    try {
      // Parse the JSON
      const changelog = JSON.parse(changelogJson);

      // Compare versions and collect changes from newer versions
      let relevantChanges = "";

      // Process each version entry
      changelog.forEach(
        (entry: { version: string; date: any; changes: any[] }) => {
          // Compare this version with current version
          if (compareVersions(entry.version, currentVersion) > 0) {
            // This is a newer version, add its changes
            // relevantChanges += `Version ${entry.version} (${entry.date}):\n`;

            // Add each change with bullet points
            if (Array.isArray(entry.changes)) {
              entry.changes.forEach((change) => {
                relevantChanges += `• ${change}\n`;
              });
            }

            // Add extra newline between versions
          }
        }
      );
      relevantChanges += "\n";

      return relevantChanges.trim();
    } catch (error) {
      console.error("Error parsing changelog:", error);
      return "Unable to parse changelog";
    }
  }
  // Simple version comparison helper
  function compareVersions(v1: string, v2: string): number {
    const parts1 = v1.split(".").map(Number);
    const parts2 = v2.split(".").map(Number);

    for (let i = 0; i < Math.max(parts1.length, parts2.length); i++) {
      const p1 = parts1[i] || 0;
      const p2 = parts2[i] || 0;
      if (p1 !== p2) return p1 - p2;
    }
    return 0;
  }




 export function refreshPreferences() {
    // Force a re-render by creating a new reference
    preferencesStore.update(prefs => ({...prefs}));
    console.log("Preferences refreshed");
  }