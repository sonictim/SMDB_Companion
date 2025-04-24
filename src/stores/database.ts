// src/stores/database.ts
import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

function createDatabaseStore() {
  const { subscribe, set, update } = writable({
    selectedDb: null,
    dbSize: 0,
    isLoading: false,
    error: null
  });
  
  return {
    subscribe,
    
    async openDatabase(is_compare: boolean = false) {
      update(state => ({ ...state, isLoading: true, error: null }));
      
      try {
        const dbPath = await invoke<string>("open_db", { isCompare: is_compare });
        
        update(state => ({
          ...state,
          selectedDb: dbPath,
          isLoading: false
        }));
        
        await this.getSize();
        
        return dbPath;
      } catch (error) {
        console.error("Failed to open database:", error);
        
        update(state => ({
          ...state,
          isLoading: false,
          error: String(error)
        }));
        
        return null;
      }
    },
    
    async closeDatabase() {
      update(state => ({ ...state, isLoading: true, error: null }));
      
      try {
        await invoke<string>("close_db");
        
        update(state => ({
          ...state,
          selectedDb: null,
          dbSize: 0,
          isLoading: false
        }));
        
        return true;
      } catch (error) {
        console.error("Failed to close database:", error);
        
        update(state => ({
          ...state,
          isLoading: false,
          error: String(error)
        }));
        
        return false;
      }
    },
    
    async getSize() {
      try {
        const size = await invoke<number>("get_db_size");
        
        update(state => ({
          ...state,
          dbSize: size
        }));
        
        return size;
      } catch (error) {
        console.error("Failed to get database size:", error);
        return 0;
      }
    }
  };
}

export const databaseStore = createDatabaseStore();