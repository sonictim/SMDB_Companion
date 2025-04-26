console.log('Loading module:', 'colors.ts');  // Add to each file

import type { FileRecord } from './types';
import { createSessionStore } from './utils';

export const resultsStore = createSessionStore<FileRecord[]>('results', []);

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

export function clearResults(): void {
    resultsStore.set([]);
}