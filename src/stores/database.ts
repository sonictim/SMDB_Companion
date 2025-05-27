console.log('Loading module:', 'database.ts');  // Add to each file

import { get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { preferencesStore, checkThinned } from './preferences';
import { createLocalStore, createSessionStore } from './utils';
import { clearResults, currentFilterStore } from './results';
import type { Database } from './types';
import { open, } from "@tauri-apps/plugin-dialog";
import { openPath } from "@tauri-apps/plugin-opener";
import { viewStore, showSearchView } from './menu';
import { platform } from '@tauri-apps/plugin-os';
import { homeDir } from '@tauri-apps/api/path';
import { showStatus, resetSearchProgress } from './status';



export const databaseStore = createSessionStore<Database | null>('database', null);


export async function setDatabase(path: string | null, is_compare: boolean) {
    // Immediately signal loading state
    databaseStore.set({
        path: '',
        name: 'Select Database',
        size: 0,
        columns: [],
        isLoading: true,
        error: null
    });
    
    if (!path) return;

    try {
        console.log("Opening database:", path);
        
        // Get database path from Tauri
        const name = await invoke<string>("open_db", {path: path, isCompare: is_compare });
        const size = await getSize();
        console.log("Database opened:", name, "Size:", size);
        const columns = await invoke<string[]>("get_columns"); 
        const pref  = get(preferencesStore);

        let db = {
            path: path,
            name: name,
            size: size,
            columns: columns,
            isLoading: false,
            error: null
        }
        
        // Set the database in the store
        databaseStore.set(db);
        
        // Add to recent databases
        addRecentDatabase({name: name, path: path});
        
        // Run various initialization tasks
        await checkThinned(path);
        
        // Reset all search-related state
        // 1. Clear results to avoid stale data
        // clearResults();
        
        // 2. Reset search status
        showStatus.set(false);
        resetSearchProgress();
        
        // 3. Reset any filter selections
        currentFilterStore.set("Relevant");
        
        // Reset view if needed
        // let view = get(viewStore);
        // if (view === "results") {
        //     showSearchView();
        // }
        
        // Try to refresh the UI without a full window reload
        // await invoke("refresh_all_windows");
        
        console.log("Database successfully initialized:", name);
        return db;
        
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
    if (path) {
        setDatabase(path, is_compare);
        

    }
    
}

export async function closeDatabase(): Promise<boolean> {

    try {
        // Close the database first, then update the store
        await invoke<string>("close_db");
        console.log("Database closed");
        
        // Set database to null after successful close
        databaseStore.set(null);
        preferencesStore.update((prefs) => ({
            ...prefs,
            algorithms: prefs.algorithms.map((algo) => {
              if (algo.id === "dbcompare") {
                console.log("Updating dbcompare:", algo, "New DB:", null);
                return { ...algo, enabled: true, db: null };
              }
              return algo;
            }),
          }));
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
      // Get the user's home directory
      const home = await homeDir();
      
      // Construct the absolute path based on platform
      let defaultPath: string;
      if (platform() === "windows") {
        defaultPath = `${home}\\AppData\\Roaming\\SoundminerV6\\Databases`;
      } else {
        defaultPath = `${home}/Library/Application Support/SoundminerV6/Databases`;
      }
      
      console.log("Opening file picker at:", defaultPath);
      let db = await open({
        multiple: false,
        directory: false,
        defaultPath,
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
 export async function openDbFolder() {
    try {
      // Call our Rust command to open the database folder
      const result = await invoke<string>("open_database_folder");
      console.log(result);
      return true;
    }
    catch (error) {
      console.error("Error opening folder:", error);
      return false;
    }
  }


export const recentDbStore = createLocalStore<{name: string, path: string}[]>('recentDatabases', []);

function addRecentDatabase(db: {name: string, path: string}) {
    if (!db || !db.name) return;
    recentDbStore.update(currentList => {
        // Remove the path if it already exists
        const updatedList = currentList.filter(item => item.name !== db.name);
        
        // Add the new path to the beginning of the list
        updatedList.unshift(db);
        
        // Limit the list to the last 10 entries
        if (updatedList.length > 10) {
            updatedList.pop();
        }
        
        return updatedList;
    });
}

  export async function getCompareDb() {
    try {
      let compareDb = await openSqliteFile();
      if (compareDb) {
        preferencesStore.update((prefs) => ({
          ...prefs,
          algorithms: prefs.algorithms.map((algo) => {
            if (algo.id === "dbcompare") {
              console.log("Updating dbcompare:", algo, "New DB:", compareDb);
              return { ...algo, enabled: true, db: compareDb };
            }
            return algo;
          }),
        }));
      }
    } catch (error) {
      console.error("Error selecting file:", error);
    }
  }

