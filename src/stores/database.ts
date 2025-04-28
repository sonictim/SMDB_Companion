console.log('Loading module:', 'database.ts');  // Add to each file

import { get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { preferencesStore } from './preferences';
import { createLocalStore, createSessionStore } from './utils';
import type { Database } from './types';
import { open } from "@tauri-apps/plugin-dialog";



export const databaseStore = createSessionStore<Database | null>('database', null);


export async function setDatabase(path: string | null, is_compare: boolean) {
    databaseStore.set({
        path: '',
        name: 'Select Database',
        size: 0,
        columns: [],
        isLoading: true,
        error: null
    });

    try {
        // Get database path from Tauri
        const name = await invoke<string>("open_db", {path: path, isCompare: is_compare });
        const size = await invoke<number>("get_db_size");
        const columns = await invoke<string[]>("get_columns");  
        
        databaseStore.set({
            path: '',
            name: name,
            size: size,
            columns: columns,
            isLoading: false,
            error: null
        });
        
    } catch (error) {
        console.error("Error setting database:", error);
        
        // Update store with error
        databaseStore.set({
            path: '',
            name: null,
            size: 0,
            columns: [],
            isLoading: false,
            error: String(error)
        });
        
        return null;
    }

}


export async function openDatabase(is_compare: boolean){
    let path = await openSqliteFile();
    if (path) {setDatabase(path, is_compare);}
    
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


 export async function openSqliteFile(): Promise<string | null> {
    try {
      let db = await open({
        multiple: false,
        directory: false,
        defaultPath: "~/Library/Application Support/SoundminerV6/Databases",
        filters: [{ name: "SQLite Database", extensions: ["sqlite"] }],
      });
      if (Array.isArray(db)) {
        db = db[0];
      }
      return db;
    }
    catch (error) {
      console.error("Error opening SQLite file:", error);
      return null;
    }
  }