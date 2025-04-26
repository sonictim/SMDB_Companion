console.log('Loading module:', 'metadata.ts');  // Add to each file


import type { Metadata } from './types';
import { writable, type Writable } from 'svelte/store';
import {createSessionStore } from './utils';



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

export const metadataStore = createSessionStore<Metadata>('metadata', initialMetadata);

