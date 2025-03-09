<script lang="ts">
    import { X, Search, AlertCircle, Loader, Square, CheckSquare } from 'lucide-svelte';
    import { invoke } from '@tauri-apps/api/core';
    import { onMount } from 'svelte';
    export let isRegistered: boolean;

    import { registrationStore } from '../store';
    import { get } from 'svelte/store';

    import type { Registration } from '../store';

    let reg: Registration = get(registrationStore);




    // let reg: Registration = { name: '', email: '', license: '' };
    let total: number = 0;

    let r2 = 'temp';
  
  async function getreg() {
      try {
          r2 = await invoke<string>('get_reg', {data: reg}); // Fetch total count first
      } catch (error) {
          console.error('Failed to fetch data:', error);
        }
  }

  async function setReg() {
      registrationStore.set(reg);
      invoke<boolean>('check_reg', {data: reg})
      .then((result) => { isRegistered = result; console.log("Registration:", result); })
      .catch((error) => console.error(error));
  }

  async function fetchData() {
      try {
          total = await invoke<number>('get_records_size'); // Fetch total count first
      } catch (error) {
          console.error('Failed to fetch data:', error);
        }
  }
  
    onMount(fetchData);
    onMount(getreg);
  
  
  
  
  </script>
  <div class="block">
    <div class="header">
      <h2>
          Search Results:
        <span class="basic-text" style="display: inline-flex; margin: 0;">
          {total} duplicates found
        </span>
      </h2>
      <button class="cta-button cancel" on:click={setReg}>
        Register
      </button>
    </div>
    <div class="input-group2">
        <label for="case-sensitive">
            Registration Required to View Results
        </label>

    </div>
    <div class="input-group">
        <label for="name">Name:</label>
        <input
            type="text"
            id="find-text"
            bind:value={reg.name}
            placeholder="Enter Registration Name"
            class="input-field"
        />
    </div>
    <div class="input-group">
        <label for="email">Email:</label>
        <input
            type="text"
            id="find-text"
            bind:value={reg.email}
            placeholder="Enter Registration Email"
            class="input-field"
        />
    </div>

    <div class="input-group">
        <label for="Reg">License:</label>
        <input
            type="text"
            id="replace-text"
            bind:value={reg.license}
            placeholder="Enter License number"
            class="input-field"
        />
    </div>
    {r2}

  </div>
   
