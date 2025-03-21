import { writable, type Writable } from 'svelte/store';

export type FileRecord = { root: string; path: string; algorithm: string[]; id: number };
export type Metadata = { find: string; replace: string; column: string; case_sensitive: boolean; mark_dirty: boolean };

export const metadataDefault = { find: '', replace: '', column: 'FilePath', case_sensitive: false, mark_dirty: true };

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