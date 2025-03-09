import { writable } from 'svelte/store';

export type FileRecord = { root: string; path: string; algorithm: string[]; id: number };
export type Metadata = { find: string; replace: string; column: string; case_sensitive: boolean; mark_dirty: boolean };

export const metadataDefault = { find: '', replace: '', column: 'FilePath', case_sensitive: false, mark_dirty: true };

// Load previous session results or default to an empty array
export const resultsStore = writable<FileRecord[]>(
    JSON.parse(sessionStorage.getItem('results') || '[]')
);

// Fix the metadata store initialization
let initialMetadata: Metadata;
try {
    const storedMetadata = sessionStorage.getItem('metadata');
    initialMetadata = storedMetadata ? JSON.parse(storedMetadata) : metadataDefault;
} catch (e) {
    console.error('Error loading metadata:', e);
    initialMetadata = metadataDefault;
}

export const metadataStore = writable<Metadata>(initialMetadata);

// Subscribe to changes and save to sessionStorage
resultsStore.subscribe(value => {
    sessionStorage.setItem('results', JSON.stringify(value));
});

metadataStore.subscribe(value => {
    sessionStorage.setItem('metadata', JSON.stringify(value));
});
