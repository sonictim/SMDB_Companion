import type { Metadata } from './types';
import { writable, type Writable } from 'svelte/store';

export const metadataDefault = { 
    find: '', 
    replace: '', 
    column: 'FilePath', 
    case_sensitive: false, 
    mark_dirty: true };


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
