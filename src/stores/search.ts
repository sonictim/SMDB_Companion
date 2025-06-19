import { createLocalStore, createSessionStore } from './utils';
import { get } from 'svelte/store';
import { open, } from "@tauri-apps/plugin-dialog";
import { invoke } from '@tauri-apps/api/core';
import { preferencesStore } from './preferences';
import { viewStore, showResultsView, isRemove, isFilesOnly, showPopup } from './menu';
import { updateResultsStore } from './results';
import type { FileRecord, SearchProgressState, Algorithm } from './types';
import { databaseStore, setDatabase, setDbSize } from './database';
import { showStatus, searchProgressStore, cancelSearch } from './status';
import { confirm, message } from "@tauri-apps/plugin-dialog";





export const searchFoldersStore = createLocalStore<String[]>('SearchFolders', []);



export async function addSearchFolders() {
    let folders = await open({
        multiple: true,
        directory: true,
     
      });

  if (!folders) return;
  let store = get(searchFoldersStore);
  folders.forEach(folder => {
    if (!store.includes(folder)) {
        store.push(folder);
        searchFoldersStore.set(store);
    }

    });
}

export function removeSearchFolders(folders: string[]) {
  if (!folders) return;
  let store = get(searchFoldersStore);
    folders.forEach(folder => {
        const index = store.indexOf(folder);
        if (index > -1) {
        store.splice(index, 1);
        }
    });
    searchFoldersStore.set(store);
}


export function clearSearchFolders() {
  searchFoldersStore.set([]);
}



export async function toggleFolderSearch(): Promise<boolean> {
    console.log("Toggle Search");
    isFilesOnly.set(true);
    isRemove.set(true);
    const currentSearching = get(showStatus);
    
    if (!currentSearching) {
       return await folderSearch();
    } else {
        cancelSearch();
        return false;
    }
}

/**
 * Start a search operation based on current preferences
 * @returns {Promise<string>} The next active tab to navigate to
 */
export async function folderSearch(): Promise<boolean> {

   
    await setDatabase("Folder Search", false)
    // Set showStatus to true at the start of the search process
    showStatus.set(true);
    showPopup.set(false);
    
    const preferences = get(preferencesStore);
    
    if (!preferences || !preferences.algorithms) {
        console.error("Preferences store not properly initialized");
        alert(
            "Application settings not loaded properly. Please restart the application."
        );
        showStatus.set(false); // Make sure to reset if we exit early
        return false;
    }
    
    let algorithms = preferences.algorithms;

    console.log("Starting Search");
    updateResultsStore([]);

    let algorithmState = algorithms.reduce(
        (acc: Record<string, boolean | number | string>, algo: Algorithm) => {
            acc[algo.id] = algo.enabled;
            if (algo.id === "duration") {
                acc["min_dur"] = algo.min_dur ?? 0;
            }
            if (algo.id === "dbcompare") {
                acc["compare_db"] = algo.db ?? "";
            }
            return acc;
        },
        {} as Record<string, boolean | number | string>
    );

    if (!algorithmState.basic) {
        algorithmState.audiosuite = false;
    }

    // Reset search progress to initial state to ensure UI shows the correct status
    searchProgressStore.set({
        searchProgress: 0,
        searchMessage: "Initializing search...",
        searchStage: "starting",
        subsearchProgress: 0,
        subsearchMessage: "Preparing search...",
        subsearchStage: "starting"
    });

    try {
        // Log the search start for debugging
        console.log("Invoking backend search with params:", { algorithmState, preferences });
        const db = get(databaseStore);
        if (!db || !db.url) {
            console.error("No database loaded or database path is invalid");
            message("No database loaded. Please open a database before searching.");
            showStatus.set(false); // Make sure to reset on error
            return false;
        }
        
        const result = await invoke<FileRecord[][]>("search_file_system", {
            enabled: algorithmState,
            pref: preferences,
            folders: get(searchFoldersStore),
        });
        
        console.log("Search Results:", result);
        setDbSize(result.length);
        
        
        // If we have results, we'll navigate to the results page
        if (result && result.length > 0) {
            if (get(viewStore) === "search") showResultsView();
            updateResultsStore(result);
            showStatus.set(false);
            
            return true;
        }
        else{
            message("No results found for the current search criteria in this database.");
        }

    } catch (error) {
        console.error("Search error:", error);
        showStatus.set(false); // Make sure to reset on error
    }
    showStatus.set(false); // Make sure to reset on error
    return false;
}

