<script lang="ts">
    import { invoke } from '@tauri-apps/api/core';
    import { onMount } from 'svelte';
    import VirtualList from 'svelte-virtual-list'; // Ensure this package is installed
    import { Square, CheckSquare, OctagonX } from 'lucide-svelte';

    import { preferencesStore, resetPreferences } from '../store';
    import { get } from 'svelte/store';
    import type { Preferences } from '../store';

    // Use the store directly instead of assigning to `pref`
    $: pref = $preferencesStore;

    let options = [
    { id: 'basic', name: 'Basic Duplicate Search', enabled: true },
    { id: 'invalidpath', name: 'Valid Filename', enabled: false },
    { id: 'filename', name: 'Similar Filename Search', enabled: false },
    { id: 'duration', name: 'Minimum Duration:', enabled: false, min_dur: 0.5 },
    { id: 'audiosuite', name: 'Audiosuite Tags', enabled: false },
    { id: 'dbcompare', name: 'Database Compare:', enabled: false },
    { id: 'waveform', name: 'Waveform Comparison', enabled: false, db: null },
];
</script>




<div class="block">
  <div class="header">
    <h2>
            Configuration Options
    </h2>
    <button class="cta-button cancel" on:click={resetPreferences}>
      <OctagonX size={18}/>
      Reset Preferences
    </button>
  </div>
  <div class ="text">
    



  </div>
  <div class="grid">
    


  </div>
    <div class="bar">
     


    </div>  
    <div class="block inner">
       



    </div>
</div> 


<style>
  .block {
    background-color: var(--secondary-bg);
    padding: 20px 20px;
    border-radius: 8px;
    flex: 1; /* Allow features to grow and shrink */
    margin-bottom: 0px; /* Adjust as needed */
    display: flex;
    flex-direction: column; /* Stack items vertically */
    height: calc(100vh - 110px); /* Full viewport height */
  }

  .block.inner {
    background-color: var(--primary-bg);
    padding: 1rem;
    border-radius: 6px;
    /* display: flex; */
    /* flex-direction: column; */
    flex-grow: 1;
    /* height: 50vh; */
    /* height: calc(100vh - 240px); */
    overflow-y: auto;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    /* margin-bottom: 1rem; */
    background-color: var(--secondary-bg);
    border-bottom: 10px;
  }

  h2 {
    font-size: 24px;
    margin-bottom: 10px;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem;
  }

  .text {
    /* margin-top: -20px; */
    margin-bottom: 1rem;
    margin-left: 35px;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 1rem;
  }

  .grid.item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    /* gap: 8px; */
    cursor: pointer;
    background: none;
    border: none;
    padding: 5px;
    font: inherit;
    font-size: 1.1rem;
    font-family: Arial, sans-serif;
    color: var(--text-color);
    text-align: left;
    transition: color 0.2s;
  }


  .grid.item:focus {
    outline: 2px solid var(--hover-color);
    outline-offset: 2px;
  }


  .bar {
    display: flex;
    align-items: center;
    margin-bottom: 10px;
    margin-top: 10px;
  }
  
</style>