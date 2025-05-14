import { createLocalStore } from "./utils";
import type { HotKeys, HashMap } from "./types";
import { get } from "svelte/store";
import type { Hash } from "lucide-svelte";
import { emit } from '@tauri-apps/api/event';

export const OldDefaultHotKeys: HotKeys = {
  settings: "CmdOrCtrl+,",
  showToolbars: "CmdOrCtrl+T", // Fixed: was just "," before
  showSearchView: "1",
  showResultsView: "2", 
  showSplitView: "3",
  showNoFrillsView: "4",
  showRegistration: "5",
  openDatabase: "CmdOrCtrl+O",
  openRecent: "CmdOrCtrl+Shift+O",
  closeDatabase: "CmdOrCtrl+W",
  searchDatabase: "CmdOrCtrl+Enter",
  cancelSearch: "Esc",
  checkSelected: "C",
  uncheckSelected: "U",
  toggleSelected: "T",
  invertSelected: "I",
  clearSelected: "Backspace",
  helpMenu: "F1"
}

export const defaultHotKeys: HashMap[] = [
  {"settings": "CmdOrCtrl+,"},
  {"showToolbars": "CmdOrCtrl+T"},
  {"showSearchView": "1"},
  {"showResultsView": "2"},
  {"showSplitView": "3"},
  {"showNoFrillsView": "4"},
  {"showRegistration": "5"},
  {"openDatabase": "CmdOrCtrl+O"},
  {"openRecent": "CmdOrCtrl+Shift+O"},
  {"closeDatabase": "CmdOrCtrl+W"},
  {"searchDatabase": "CmdOrCtrl+Enter"},
  {"cancelSearch": "Esc"},
  {"checkSelected": "C"},
  {"uncheckSelected": "U"},
  {"toggleSelected": "T"},
  {"invertSelected": "I"},
  {"clearSelected": "Backspace"},
  {"helpMenu": "F1"}
]

export const hotkeysStore = createLocalStore<HashMap[]>('hotkeys', defaultHotKeys);

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
    // Emit an event that components and menu can listen for
    await emit('hotkey-change', {
      timestamp: Date.now(),
    });
    console.log('Emitted hotkey change event');
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

