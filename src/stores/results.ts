// src/stores/results.ts
import type { FileRecord } from './types';
import { writable, type Writable } from 'svelte/store';


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