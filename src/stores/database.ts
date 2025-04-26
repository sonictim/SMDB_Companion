console.log('Loading module:', 'database.ts');  // Add to each file

import { get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { preferencesStore } from './preferences';
import { createLocalStore, createSessionStore } from './utils';
import type { Database } from './types';


// Initialize with null (no database loaded)
export const databaseStore = createSessionStore<Database | null>('database', null);

export async function openDatabase(is_compare: boolean = false): Promise<string | null> {
    // Set loading state
    databaseStore.set({
        path: '',
        size: 0,
        columns: [],
        isLoading: true,
        error: null
    });
    
    try {
        // Get database path from Tauri
        const dbPath = await invoke<string>("open_db", { isCompare: is_compare });
        console.log("db path: ", dbPath);
        
        // Get database size
        const size = await getSize();
        
        // Get columns
        const columns = await fetchColumns();
        
        // Update store with complete database info
        databaseStore.set({
            path: dbPath,
            size,
            columns,
            isLoading: false,
            error: null
        });
        
        return dbPath;
    } catch (error) {
        console.error("Error opening database:", error);
        
        // Update store with error
        databaseStore.set({
            path: '',
            size: 0,
            columns: [],
            isLoading: false,
            error: String(error)
        });
        
        return null;
    }
}

export async function closeDatabase(): Promise<boolean> {
    try {
        // Close the database first, then update the store
        await invoke<string>("close_db");
        console.log("Database closed");
        
        // Set database to null after successful close
        databaseStore.set(null);
        return true;
    } catch (error) {
        console.error("Error closing database:", error);
        return false;
    }
}

export async function getSize(): Promise<number> {
    try {
        const size = await invoke<number>("get_db_size");
        console.log("Database size:", size);
        
        // Update size in store if database exists
        const currentDb = get(databaseStore);
        if (currentDb) {
            databaseStore.set({
                ...currentDb,
                size
            });
        }
        
        return size;
    } catch (error) {
        console.error("Error getting database size:", error);
        return 0;
    }
}

export async function fetchColumns(): Promise<string[]> {
    try {
        const columns = await invoke<string[]>("get_columns");
        
        // Update columns in store if database exists
        const currentDb = get(databaseStore);
        if (currentDb) {
            databaseStore.set({
                ...currentDb,
                columns
            });
        }
        
        // Also update preferences store as in your original code
        preferencesStore.update((currentPrefs) => ({
            ...currentPrefs,
            columns: columns,
        }));
        
        return columns;
    } catch (error) {
        console.error("Failed to fetch columns:", error);
        return [];
    }
}

// Utility function to check if database is open
export function isDatabaseOpen(): boolean {
    const db = get(databaseStore);
    return db !== null && db.path !== '';
}

// Utility function to get current database path
export function getDatabasePath(): string | null {
    const db = get(databaseStore);
    return db ? db.path : null;
}