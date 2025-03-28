import { writable, type Writable } from 'svelte/store';
import { listen } from "@tauri-apps/api/event";

export type HashMap = Record<string, string>;

export type FileRecord = { filename: string; path: string; algorithm: string[]; id: number; duration: string; samplerate: string; bitdepth: string; channels: string; description: string; };
export type Metadata = { find: string; replace: string; column: string; case_sensitive: boolean; mark_dirty: boolean };

export const metadataDefault = { find: '', replace: '', column: 'FilePath', case_sensitive: false, mark_dirty: true };

export const isSearching = writable(false);

export type SearchProgressState = {
    searchProgress: number;
    searchMessage: string;
    searchStage: string;
    subsearchProgress: number;
    subsearchMessage: string;
    subsearchStage: string;
};

// Create a store for search progress with persistence
export const searchProgressStore = writable<SearchProgressState>({
    searchProgress: 0,
    searchMessage: "Initializing...",
    searchStage: "",
    subsearchProgress: 0,
    subsearchMessage: "Requesting Records...",
    subsearchStage: ""
});

// Fix the metadata store initialization
let initialMetadata: Metadata;
try {
    const storedMetadata = sessionStorage.getItem('metadata');
    initialMetadata = storedMetadata ? JSON.parse(storedMetadata) : metadataDefault;
} catch (e) {
    console.error('Error loading metadata:', e);
    initialMetadata = metadataDefault;
}

export const metadataStore: Writable<Metadata> = writable<Metadata>(initialMetadata);

// Subscribe to changes and save to sessionStorage
metadataStore.subscribe(value => {
    sessionStorage.setItem('metadata', JSON.stringify(value));
});

// Initialize results store with proper typing
export const resultsStore: Writable<FileRecord[]> = writable<FileRecord[]>([]);

// Helper function to update results store
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
}

export const virtualizerStore = writable(null);
export const scrollPositionStore = writable(0);

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