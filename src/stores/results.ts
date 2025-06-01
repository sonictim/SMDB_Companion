console.log('Loading module:', 'results.ts');  // Fixed module name

import type { FileRecord } from './types';
import { writable, derived, get } from 'svelte/store';
import { preferencesStore } from './preferences';
import { getHotkey } from './hotkeys';
import { invoke } from "@tauri-apps/api/core";
import { getIsMac } from './utils';
import { message, } from "@tauri-apps/plugin-dialog";


// Column configuration type
export interface ColumnConfig {
  minWidth: number;
  width: number;
  name: string;
  header: string;
}

// Default column configurations
const defaultColumnConfigs: ColumnConfig[] = [
  { minWidth: 10, width: 30, name: "checkbox", header: "✔" },
  { minWidth: 100, width: 250, name: "filename", header: "Filename" },
  { minWidth: 150, width: 400, name: "path", header: "Path" },
  { minWidth: 100, width: 300, name: "description", header: "Description" },
  { minWidth: 20, width: 80, name: "algorithm", header: "Match" },
  { minWidth: 10, width: 25, name: "channels", header: "CH" },
  { minWidth: 10, width: 25, name: "bitdepth", header: "BD" },
  { minWidth: 10, width: 50, name: "samplerate", header: "SR" },
  { minWidth: 10, width: 80, name: "duration", header: "Duration" },
  { minWidth: 8, width: 30, name: "audio", header: "" },
];

// Create persistent store for column configurations using localStorage
function createColumnStore() {
  const STORAGE_KEY = 'smdb-column-configs';
  
  // Load from localStorage or use defaults
  const loadColumnConfigs = (): ColumnConfig[] => {
    try {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored) {
        const parsed = JSON.parse(stored);
        // Validate the stored data has required fields
        if (Array.isArray(parsed) && parsed.every(col => 
          typeof col.minWidth === 'number' && 
          typeof col.width === 'number' && 
          typeof col.name === 'string' && 
          typeof col.header === 'string'
        )) {
          return parsed;
        }
      }
    } catch (error) {
      console.warn('Failed to load column configs from localStorage:', error);
    }
    return [...defaultColumnConfigs];
  };

  const { subscribe, set, update } = writable<ColumnConfig[]>(loadColumnConfigs());

  return {
    subscribe,
    set: (configs: ColumnConfig[]) => {
      set(configs);
      try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(configs));
      } catch (error) {
        console.warn('Failed to save column configs to localStorage:', error);
      }
    },
    update: (fn: (configs: ColumnConfig[]) => ColumnConfig[]) => {
      update(configs => {
        const newConfigs = fn(configs);
        try {
          localStorage.setItem(STORAGE_KEY, JSON.stringify(newConfigs));
        } catch (error) {
          console.warn('Failed to save column configs to localStorage:', error);
        }
        return newConfigs;
      });
    },
    updateColumnWidth: (index: number, width: number) => {
      update(configs => {
        const newConfigs = [...configs];
        if (newConfigs[index]) {
          newConfigs[index] = { 
            ...newConfigs[index], 
            width: Math.max(newConfigs[index].minWidth, width) 
          };
        }
        try {
          localStorage.setItem(STORAGE_KEY, JSON.stringify(newConfigs));
        } catch (error) {
          console.warn('Failed to save column configs to localStorage:', error);
        }
        return newConfigs;
      });
    },
    resetToDefaults: () => {
      const configs = [...defaultColumnConfigs];
      set(configs);
      try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(configs));
      } catch (error) {
        console.warn('Failed to save column configs to localStorage:', error);
      }
    }
  };
}

// Column-related stores
export const columnConfigStore = createColumnStore();

// Derived stores for column calculations
export const columnWidthsStore = derived(
  columnConfigStore,
  ($columnConfigs) => $columnConfigs.map(config => config.width)
);

export const totalWidthStore = derived(
  columnWidthsStore,
  ($columnWidths) => $columnWidths.reduce((acc, width) => acc + width, 0) + 12 + "px"
);

export const gridTemplateColumnsStore = derived(
  columnWidthsStore,
  ($columnWidths) => $columnWidths.map(width => `${width}px`).join(" ")
);

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

// Remove specific IDs from results
export function removeIdsFromResults(idsToRemove: number[]): void {
    if (!idsToRemove || idsToRemove.length === 0) {
        return;
    }
    
    const idsSet = new Set(idsToRemove);
    
    // Remove records with matching IDs from the results store
    resultsStore.update(items => {
        return items.filter(item => !idsSet.has(item.id));
    });
    
    // Clear any selections for the removed items
    selectedItemsStore.update(currentSelected => {
        const newSelected = new Set(currentSelected);
        idsToRemove.forEach(id => newSelected.delete(id));
        return newSelected;
    });
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

    const isMac = await getIsMac();

    

    const pathsToReveal = Array.from(selectedItems)
      .map(id => {
        const item = get(filteredItemsStore).find(i => i.id === id);
        if (!item) return null;
        
        const separator = isMac ? "/" : "\\";
        return item.path + separator + item.filename;
      })
      .filter((path): path is string => path !== null);

    if (pathsToReveal.length > 0) {
      try {
        // message("Attempting to reveal the following files:\n" + pathsToReveal.join("\n"));
        await invoke("reveal_files", { paths: pathsToReveal });
        console.log("Revealed files:", pathsToReveal);
      } catch (error) {
        console.error("Error revealing files:", error);
      }
    } else {
      console.warn("No valid paths to reveal.");
    }

  }