import { createLocalStore } from "./utils";
import type { HotKeys, HashMap } from "./types";
import { get } from "svelte/store";
import type { Hash } from "lucide-svelte";
import { emit } from '@tauri-apps/api/event';
import { platform } from '@tauri-apps/plugin-os';



export const defaultHotKeys: HashMap[] = [
  {"openDatabase": "CmdOrCtrl+O"},
  {"openRecent": "CmdOrCtrl+Shift+O"},
  {"closeDatabase": "CmdOrCtrl+W"},
  {"searchDatabase": "CmdOrCtrl+Enter"},
  {"cancelSearch": "Esc"},
  {"removeRecords": "CmdOrCtrl+Backspace"},
  {"settings": "CmdOrCtrl+,"},
  {"showToolbars": ","},
  {"showSearchView": "1"},
  {"showResultsView": "2"},
  {"showSplitView": "3"},
  {"showNoFrillsView": "4"},
  {"showRegistration": "5"},
  {"checkSelected": "C"},
  {"uncheckSelected": "U"},
  {"toggleSelected": "T"},
  {"invertSelected": "I"},
  {"clearSelected": "Backspace"},
  {"helpMenu": "F1"},
  
  // Mouse modifiers for table selection
  {"toggleRowSelect": "Click"},
  {"toggleSelectAll": "Alt+Click"},
  {"selectRange": "Shift+Click"},
  {"unselectRange": "CmdOrCtrl+Shift+Click"},
  {"lassoSelect": "Drag"},
  {"lassoUnselect": "CmdOrCtrl+Drag"}
]

// Initialize the hotkeys store with defaults, with a safety check for empty arrays
export const hotkeysStore = (() => {
  // Try to get existing hotkeys
  try {
    const storedHotkeys = localStorage.getItem('hotkeys');
    if (storedHotkeys) {
      const parsed = JSON.parse(storedHotkeys);
      // Check if it's an array and not empty
      if (Array.isArray(parsed) && parsed.length > 0) {
        console.log(`Found ${parsed.length} hotkeys in localStorage`);
        return createLocalStore<HashMap[]>('hotkeys', parsed);
      } else {
        console.warn('Found empty or invalid hotkeys in localStorage, using defaults');
      }
    }
  } catch (e) {
    console.error('Error parsing hotkeys from localStorage:', e);
  }
  
  // If we get here, either there were no stored hotkeys or they were invalid
  console.log(`Using ${defaultHotKeys.length} default hotkeys`);
  return createLocalStore<HashMap[]>('hotkeys', defaultHotKeys);
})();

export function checkForNewDefaults(): void {
    const store = get(hotkeysStore);
    console.log(`Checking hotkeys - found ${store.length} existing entries in store`);
    
    // If store is empty or not an array, reset to defaults
    if (!Array.isArray(store) || store.length === 0) {
        console.log("Hotkeys store empty or invalid, resetting to defaults");
        hotkeysStore.set([...defaultHotKeys]);
        return;
    }
    
    // Create a map of existing hotkeys for quick lookup
    const existingHotkeysMap = new Map();
    let validEntries = 0;
    
    store.forEach(item => {
        if (item && typeof item === 'object') {
            const keys = Object.keys(item);
            if (keys.length > 0) {
                const key = keys[0];
                existingHotkeysMap.set(key, item[key]);
                validEntries++;
            }
        }
    });
    
    console.log(`Found ${validEntries} valid hotkey entries out of ${store.length}`);
    
    // Start with the current valid store values
    const updatedHotkeys = store.filter(item => 
        item && typeof item === 'object' && Object.keys(item).length > 0
    );
    
    let addedCount = 0;
    
    // Add any new default hotkeys that don't exist in the current store
    defaultHotKeys.forEach(defaultItem => {
        const defaultKey = Object.keys(defaultItem)[0];
        if (!existingHotkeysMap.has(defaultKey)) {
            // This is a new hotkey that doesn't exist in the store
            updatedHotkeys.push(defaultItem);
            addedCount++;
            console.log(`Added missing hotkey: ${defaultKey} -> ${defaultItem[defaultKey]}`);
        }
    });
    
    console.log(`Added ${addedCount} default hotkeys to store`);
    
    // Only update if we made changes
    if (addedCount > 0 || validEntries < store.length) {
        console.log("Updating hotkeys store with new values");
        hotkeysStore.set(updatedHotkeys);
    }
}


// Helper function to get a hotkey value by key name
export function getHotkey(key: string): string {
  const hotkeys = get(hotkeysStore);
  
  // Find the object in the array that contains this key
  const hotkeyObj = hotkeys.find(obj => Object.keys(obj)[0] === key);
  
  // Return the value if found, otherwise look in the default hotkeys
  if (hotkeyObj) {
    return hotkeyObj[key];
  }
  
  // Try to find in default hotkeys
  const defaultObj = defaultHotKeys.find(obj => Object.keys(obj)[0] === key);
  return defaultObj ? defaultObj[key] : "";
}

// Notify other parts of the application about hotkey changes
export async function notifyHotkeyChange(): Promise<void> {
  try {
    // Validate the hotkeys store before notification
    const currentHotkeys = get(hotkeysStore);
    
    // Ensure we're not sending an empty array or corrupted data
    if (!Array.isArray(currentHotkeys) || currentHotkeys.length === 0) {
      console.warn('Hotkeys appear to be empty or corrupted, running checkForNewDefaults before notification');
      checkForNewDefaults();
    }
    
    const timestamp = Date.now();
    
    // Ensure that localStorage reflects the current state BEFORE emit
    // This ensures all windows will get the same state when they react to the event
    const finalHotkeys = get(hotkeysStore);
    localStorage.setItem('hotkeys', JSON.stringify(finalHotkeys));
    console.log(`Updated localStorage with ${finalHotkeys.length} hotkeys before notification`);
    
    // Emit an event that components and menu can listen for
    await emit('hotkey-change', {
      timestamp,
      count: finalHotkeys.length
    });
    console.log(`Emitted hotkey-change event with timestamp ${timestamp}`);
  } catch (err) {
    console.error('Failed to emit hotkey change event:', err);
  }
}

// Helper function to update a hotkey value by key name
export async function setHotkey(key: string, value: string): Promise<void> {
  const hotkeys = get(hotkeysStore);
  
  // Create a new array to avoid mutating the original
  const updatedHotkeys = [...hotkeys];
  
  // Find the index of the object that contains this key
  const index = updatedHotkeys.findIndex(obj => Object.keys(obj)[0] === key);
  
  if (index !== -1) {
    // Replace the entire object with a new one containing the updated value
    updatedHotkeys[index] = { [key]: value };
  } else {
    // If the key doesn't exist yet, add a new object for it
    updatedHotkeys.push({ [key]: value });
  }
  
  // Update the store with the modified array
  hotkeysStore.set(updatedHotkeys);
  
  // Notify about the change to trigger menu refresh
  await notifyHotkeyChange();
}

