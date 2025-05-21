console.log('Loading module:', 'status.ts');  // Add to each file


import type { SearchProgressState, Algorithm, FileRecord } from './types';
import { writable, type Writable, get } from 'svelte/store';
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { preferencesStore } from './preferences';
import { resultsStore } from './results';
import { viewStore, showResultsView, isRemove } from './menu';


export const showStatus = writable(false);
export const currentTaskId = writable<number | null>(null);
export const virtualizerStore = writable(null);
export const scrollPositionStore = writable(0);



// Create a store for search progress with persistence
export const searchProgressStore = writable<SearchProgressState>({
    searchProgress: 0,
    searchMessage: "Initializing...",
    searchStage: "",
    subsearchProgress: 0,
    subsearchMessage: "Requesting Records...",
    subsearchStage: ""
});


// Single event listener setup flag
let listenersInitialized = false;

/**
 * Initialize search status event listeners
 * Returns a promise that resolves when listeners are set up
 */
export async function initializeSearchListeners(): Promise<void> {
    // Only set up listeners once per application lifecycle
    if (!listenersInitialized) {
        console.log("Setting up search listeners for the first time");

        // Create persistent listeners that will remain active throughout the app's lifecycle
        await listen<{
            progress: number;
            message: string;
            stage: string;
        }>("search-status", (event) => {
            const status = event.payload;
            searchProgressStore.update((state) => ({
                ...state,
                searchProgress: status.progress,
                searchMessage: status.message,
                searchStage: status.stage,
            }));
            
            // Set showStatus to true whenever we get a search status update
            // This ensures the UI stays in "searching" mode while the backend is working
            // if (status.stage !== "complete" && status.stage !== "cancelled") {
            //     showStatus.set(true);
            // } else if (status.stage === "complete" || status.stage === "cancelled") {
            //     showStatus.set(false);
                
            // }
            
            // console.log(
            //     `Search status: ${status.stage} - ${status.progress}% - ${status.message}`
            // );
        });

        await listen<{
            progress: number;
            message: string;
            stage: string;
        }>("search-sub-status", (event) => {
            const status = event.payload;
            searchProgressStore.update((state) => ({
                ...state,
                subsearchProgress: status.progress,
                subsearchMessage: status.message,
                subsearchStage: status.stage,
            }));
            // console.log(
            //     `Search sub-status: ${status.stage} - ${status.progress}% - ${status.message}`
            // );
        });

        listenersInitialized = true;
    } else {
        console.log("Search listeners already initialized");
    }
}

/**
 * Toggle search state and start/cancel search accordingly
 * @returns {Promise<void>}
 */
export async function toggleSearch(): Promise<boolean> {
    console.log("Toggle Search");
    isRemove.set(true);
    const currentSearching = get(showStatus);
    
    if (!currentSearching) {
       return await search();
    } else {
        cancelSearch();
        return false;
    }
}

/**
 * Start a search operation based on current preferences
 * @returns {Promise<string>} The next active tab to navigate to
 */
export async function search(): Promise<boolean> {
    // Set showStatus to true at the start of the search process
    showStatus.set(true);
    
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
    resultsStore.set([]);

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
        
        const result = await invoke<FileRecord[]>("search", {
            enabled: algorithmState,
            pref: preferences,
        });
        
        console.log("Search Results:", result);
        
        
        // If we have results, we'll navigate to the results page
        if (result && result.length > 0) {
            if (get(viewStore) === "search") showResultsView();
            resultsStore.set(result);
            showStatus.set(false);
            
            return true;
        }
    } catch (error) {
        console.error("Search error:", error);
        showStatus.set(false); // Make sure to reset on error
    }
    showStatus.set(false); // Make sure to reset on error
    return false;
}

/**
 * Cancel an ongoing search operation
 */
export async function cancelSearch(): Promise<void> {
    showStatus.set(false);
    await invoke("cancel_search")
        .then(() => {
            console.log("Search cancellation requested");
            
            // Reset progress in store
            resetSearchProgress();
        })
        .catch((error) => {
            console.error("Error cancelling search:", error);
        });
}

/**
 * Reset search progress state
 */
export function resetSearchProgress(): void {
    searchProgressStore.set({
        searchProgress: 0,
        searchMessage: "Search cancelled",
        searchStage: "",
        subsearchProgress: 0,
        subsearchMessage: "Search cancelled",
        subsearchStage: "",
    });
}

/**
 * Clean up search status event listeners - keeping this for backwards compatibility
 */
export function cleanupSearchListeners(): void {
    // This function is now a no-op since listeners are persistent
    // Kept for API compatibility
    console.log("cleanupSearchListeners is deprecated - listeners are now persistent");
}

