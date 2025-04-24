// src/stores/registration.ts
import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import type { Registration } from './types';
import { loadFromLocalStorage, saveToLocalStorage } from './utils';

const defaultReg: Registration = { name: '', email: '', license: '' };
const initialReg = loadFromLocalStorage<Registration>('registrationInfo', defaultReg);

function createRegistrationStore() {
  const store = writable<Registration>(initialReg);
  
  return {
    subscribe: store.subscribe,
    set: store.set,
    update: store.update,
    
    async checkRegistered() {
      const regData = get(store);
      
      try {
        const isRegistered = await invoke<boolean>("check_reg", { 
          data: regData 
        });
        
        return isRegistered;
      } catch (error) {
        console.error("Registration check failed:", error);
        return false;
      }
    },
    
    async setReg(reg: Registration) {
      store.set(reg);
      
      try {
        const result = await invoke<boolean>("check_reg", { data: reg });
        return result;
      } catch (error) {
        console.error("Failed to set registration:", error);
        return false;
      }
    },
    
    reset() {
      store.set(defaultReg);
    }
  };
}

export const registrationStore = createRegistrationStore();

// Save to localStorage whenever updated
registrationStore.subscribe(value => {
  saveToLocalStorage('registrationInfo', value);
});