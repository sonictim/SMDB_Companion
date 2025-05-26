console.log('Loading module:', 'results.ts');  // Fixed module name

import type { FileRecord } from './types';
import { createSessionStore } from './utils';
import { writable, derived, get } from 'svelte/store';
import { preferencesStore } from './preferences';
import { getHotkey } from './hotkeys';
import { invoke } from "@tauri-apps/api/core";


// Main results store
// Change from using createSessionStore
// export const resultsStore = createSessionStore<FileRecord[]>('results', []);

// To using a regular writable store
export const resultsStore = writable<FileRecord[]>([]);

// Selection-related stores
export const selectedItemsStore = writable<Set<number>>(new Set());
export const lastSelectedIndexStore = writable<number>(-1);
export const enableSelectionsStore = writable<boolean>(true);

// Filter-related stores
export const currentFilterStore = writable<string>("Relevant");
export const manualFiltersStore = writable([
  { id: "All", name: "All Records", enabled: true },
  { id: "Relevant", name: "Relevant Records", enabled: true },
  { id: "Keep", name: "Records to Keep", enabled: true },
  { id: "Remove", name: "Records to Remove", enabled: true },
  { id: "spacer", name: "──────────", enabled: true },
]);

// Derived store for all available filters (manual + algorithm filters)
export const filtersStore = derived(
  [manualFiltersStore, preferencesStore],
  ([$manualFilters, $preferences]) => {
    return [...$manualFilters, ...$preferences.algorithms];
  }
);

// Derived store for filtered items
export const filteredItemsStore = derived(
  [resultsStore, currentFilterStore, preferencesStore],
  ([$results, $currentFilter, $preferences]) => {
    let items =  filterItems($results, $currentFilter);
    if (items.length === 0 && $results.length > 0) {
      // If the filter returns no results, return a placeholder item
      return [noResults];
    }
    return items;
  }
);

// Helper function for filtering items
export function filterItems(items: FileRecord[], filter: string): FileRecord[] {
  switch (filter) {
    case "All":
      return items;
    case "Relevant":
      let result = items.filter(
        (item) => !item.algorithm.includes("Keep") || item.algorithm.length > 1
      );
      if (result.length === 0 && items.length > 0) { return items; }
      return result;

    case "Keep":
      return items.filter((item) => item.algorithm.includes("Keep"));
    case "Remove":
      return items.filter((item) => !item.algorithm.includes("Keep"));
    case "audiosuite":
      return items.filter((item) => item.algorithm.includes("Tags"));
    case "dual_mono":
      return items.filter((item) => item.algorithm.includes("DualMono"));
    case "filename":
      return items.filter((item) => item.algorithm.includes("SimilarFilename"));
    case "waveform":
      return items.filter(
        (item) =>
          item.algorithm.includes("Waveforms") ||
          item.algorithm.includes("SimilarAudio") ||
          item.algorithm.includes("waveform")
      );
    default:
      // For algorithm filters, check if filter name matches any algorithm
      return items.filter((item) =>
        item.algorithm.some(
          (algo) => algo === filter || algo.toLowerCase() === filter.toLowerCase()
        )
      );
  }
}

// Update the results
export function updateResultsStore(newResults: any): void {
    // If newResults is already an array of records, use it directly
    if (newResults && !Array.isArray(newResults[0])) {
        resultsStore.set(newResults as FileRecord[]);
    }
    // If it's an array containing an array, flatten it
    else if (newResults && Array.isArray(newResults[0])) {
        resultsStore.set(newResults[0] as FileRecord[]);
    }
    else {
        resultsStore.set([]);
    }
    
    // Clear selections when results change
    clearSelected();
}

// Clear the results
export function clearResults(): void {
    resultsStore.set([]);
    clearSelected();
}

// Selection-related functions
export function toggleEnableSelections(): void {
  enableSelectionsStore.update(value => !value);
}

export function clearSelected(): void {
  selectedItemsStore.set(new Set());
}

export function invertSelected(): void {
  const filtered = get(filteredItemsStore);
  const selected = get(selectedItemsStore);
  
  selectedItemsStore.update(currentSelected => {
    const newSelected = new Set<number>();
    
    filtered.forEach(item => {
      if (!currentSelected.has(item.id)) {
        newSelected.add(item.id);
      }
    });
    
    return newSelected;
  });
}

// Helper functions to check if modifiers match the user-configured hotkeys
function getModifiersFromHotkey(hotkeyName: string): {alt: boolean, shift: boolean, meta: boolean} {
  const hotkey = getHotkey(hotkeyName);
  return {
    alt: hotkey.includes("Alt+"),
    shift: hotkey.includes("Shift+"),
    meta: hotkey.includes("CmdOrCtrl+")
  };
}

function isToggleSelectAllModifier(event: MouseEvent): boolean {
  const mods = getModifiersFromHotkey("toggleSelectAll");
  return event.altKey === mods.alt && 
         event.shiftKey === mods.shift && 
         (event.metaKey || event.ctrlKey) === mods.meta;
}

function isSelectRangeModifier(event: MouseEvent): boolean {
  const mods = getModifiersFromHotkey("selectRange");
  return event.altKey === mods.alt && 
         event.shiftKey === mods.shift && 
         (event.metaKey || event.ctrlKey) === mods.meta;
}

function isUnselectRangeModifier(event: MouseEvent): boolean {
  const mods = getModifiersFromHotkey("unselectRange");
  return event.altKey === mods.alt && 
         event.shiftKey === mods.shift && 
         (event.metaKey || event.ctrlKey) === mods.meta;
}

export function toggleSelect(item: FileRecord, event: MouseEvent): void {
  const filtered = get(filteredItemsStore);
  const currentIndex = filtered.findIndex(record => record.id === item.id);
  
  selectedItemsStore.update(currentSelected => {
    const newSelected = new Set(currentSelected);
    
    // Toggle Select All action
    if (isToggleSelectAllModifier(event)) {
      if (newSelected.size > 0) {
        newSelected.clear();
      } else {
        filtered.forEach(record => newSelected.add(record.id));
      }
      return newSelected;
    }
    
    // Handle Range selection operations
    if ((isSelectRangeModifier(event) || isUnselectRangeModifier(event)) && 
        get(lastSelectedIndexStore) !== -1) {
      const start = Math.min(get(lastSelectedIndexStore), currentIndex);
      const end = Math.max(get(lastSelectedIndexStore), currentIndex);
      
      // Unselect range
      if (isUnselectRangeModifier(event)) {
        for (let i = start; i <= end; i++) {
          newSelected.delete(filtered[i].id);
        }
      } 
      // Select range
      else {
        for (let i = start; i <= end; i++) {
          newSelected.add(filtered[i].id);
        }
      }
      // Always update the last selected index for range operations
      lastSelectedIndexStore.set(currentIndex);
    } else {
      // Normal toggle single selection
      if (newSelected.has(item.id)) {
        newSelected.delete(item.id);
      } else {
        newSelected.add(item.id);
      }
      // Always update the last selected index for regular clicks
      lastSelectedIndexStore.set(currentIndex);
    }
    
    return newSelected;
  });
}

// Check/Uncheck-related functions
export function toggleChecked(item: FileRecord): void {
  const isKeeping = item.algorithm.includes("Keep");
  
  resultsStore.update(items => {
    return items.map(i => {
      if (i.id === item.id) {
        const updatedAlgorithms = isKeeping
          ? i.algorithm.filter(algo => algo !== "Keep")
          : [...i.algorithm, "Keep"];
        
        return { ...i, algorithm: updatedAlgorithms };
      }
      return i;
    });
  });
}

export function checkSelected(): void {
  const selectedItems = get(selectedItemsStore);
  
  resultsStore.update(items => {
    return items.map(item => {
      if (selectedItems.has(item.id) && item.algorithm.includes("Keep")) {
        return {
          ...item,
          algorithm: item.algorithm.filter(algo => algo !== "Keep")
        };
      }
      return item;
    });
  });
}

export function uncheckSelected(): void {
  const selectedItems = get(selectedItemsStore);
  
  resultsStore.update(items => {
    return items.map(item => {
      if (selectedItems.has(item.id) && !item.algorithm.includes("Keep")) {
        return {
          ...item,
          algorithm: [...item.algorithm, "Keep"]
        };
      }
      return item;
    });
  });
}

export function toggleChecksSelected(): void {
  const selectedItems = get(selectedItemsStore);
  
  resultsStore.update(items => {
    return items.map(item => {
      if (selectedItems.has(item.id)) {
        const hasKeep = item.algorithm.includes("Keep");
        return {
          ...item,
          algorithm: hasKeep
            ? item.algorithm.filter(algo => algo !== "Keep")
            : [...item.algorithm, "Keep"]
        };
      }
      return item;
    });
  });
}

export function getTotalChecks(): number {
  const filtered = get(filteredItemsStore);
  return filtered.filter(item => !item.algorithm.includes("Keep")).length;
}

// Create a derived store for the number of checked items
export const totalChecksStore = derived(
  [filteredItemsStore],
  ([$filteredItems]) => {
    return $filteredItems.filter(item => !item.algorithm.includes("Keep")).length;
  }
);

// Create a derived store for the number of checked items within selection
export const selectedChecksStore = derived(
  [filteredItemsStore, selectedItemsStore],
  ([$filteredItems, $selectedItems]) => {
    return $filteredItems
      .filter(item => $selectedItems.has(item.id)) // Filter to only selected items
      .filter(item => !item.algorithm.includes("Keep")) // Count only those marked for removal
      .length;
  }
);

export function updateCurrentFilter(filter: string): void {
    currentFilterStore.set(filter);
}

const noResults = { 
    filename: "Filter", 
    path: "has", 
    algorithm: ["Keep"],
    id: 0, 
    duration: '', 
    samplerate: '', 
    bitdepth: '', 
    channels: '', 
    description: "no results", }


      export async function revealSelectedFiles() {
    const selectedItems = get(selectedItemsStore);
    if (selectedItems.size === 0) {
      console.warn("No items selected to reveal.");
      return;
    }

    const pathsToReveal = Array.from(selectedItems).map(id => {
      const item = get(filteredItemsStore).find(i => i.id === id);
      return item ? item.path + "/" + item.filename : null;
    }).filter(path => path !== null);

    if (pathsToReveal.length > 0) {
      try {
        await invoke("reveal_files", { paths: pathsToReveal });
        console.log("Revealed files:", pathsToReveal);
      } catch (error) {
        console.error("Error revealing files:", error);
      }
    } else {
      console.warn("No valid paths to reveal.");
    }

  }