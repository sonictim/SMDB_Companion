console.log('Loading module:', 'registration.ts');  // Add to each file
// src/stores/registration.ts
import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import type { Registration } from './types';
import { createLocalStore, } from './utils';

const defaultReg: Registration = { name: '', email: '', license: ''};




export const registrationStore = createLocalStore<Registration>('registration', defaultReg);


export async function checkRegistered() {
  console.log("Checking Registration");
  let reg = get(registrationStore);
  
  try {
    const isRegistered = await invoke<boolean>("check_reg", { data: reg });
    console.log("Registration:", isRegistered);
    return isRegistered;
  } catch (error) {
    console.error(error);
    return false;
  }
}



