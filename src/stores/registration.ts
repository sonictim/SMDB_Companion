console.log('Loading module:', 'registration.ts');  // Add to each file
// src/stores/registration.ts
import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import type { Registration } from './types';
import { createLocalStore, createSessionStore } from './utils';

const defaultReg: Registration = { name: '', email: '', license: ''};




export const registrationStore = createLocalStore<Registration>('registrationInfo', defaultReg);
export const isRegistered = writable<boolean>(false);

export const regInput = createSessionStore<Registration>('regInput', defaultReg);
export const attemptFailed = writable<boolean>(false);


export async function checkRegistered() {
  console.log("Checking Registration");
  let reg = get(registrationStore);
  
  try {
    const isRegistered = await invoke<boolean>("check_reg", { data: reg });
    console.log("Registration:", isRegistered);
    attemptFailed.set(!isRegistered);
    console.log("Attempt Failed:", get(attemptFailed));
    return isRegistered;
  } catch (error) {
    console.error(error);
    return false;
  }
}

export async function setReg() {
  // Get the latest data from regInput
  const regData = get(regInput);
  
  // Update the registrationStore
  registrationStore.set(regData);
  
  // Instead of using the store for validation, directly use the data we just set
  try {
    const registrationStatus = await invoke<boolean>("check_reg", { data: regData });
    console.log("Registration:", registrationStatus);
    isRegistered.set(registrationStatus); // Update the isRegistered store
    console.log("isRegistered store updated:", get(isRegistered));
    attemptFailed.set(!registrationStatus);
    console.log("Attempt Failed:", get(attemptFailed));
    return registrationStatus;
  } catch (error) {
    console.error(error);
    return false;
  }
}


