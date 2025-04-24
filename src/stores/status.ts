import type { SearchProgressState } from './types';
import { writable, type Writable } from 'svelte/store';
import { listen } from "@tauri-apps/api/event";


export const isSearching = writable(false);
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
            console.log(
                `Search status: ${status.stage} - ${status.progress}% - ${status.message}`
            );
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
            console.log(
                `Search sub-status: ${status.stage} - ${status.progress}% - ${status.message}`
            );
        });

        listenersInitialized = true;
    } else {
        console.log("Search listeners already initialized");
    }
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