<script lang="ts">
    import { invoke } from '@tauri-apps/api/core';
    import { onMount } from 'svelte';
    import VirtualList from 'svelte-virtual-list'; // Ensure this package is installed
    import { Square, CheckSquare, OctagonX } from 'lucide-svelte';

    import { preferencesStore, resetPreferences } from '../../store';
    import { get } from 'svelte/store';
    import type { Preferences } from '../../store';

    // Use the store directly instead of assigning to `pref`
    $: pref = $preferencesStore;

    let options = [
      { id: 'duration', name: 'Ignore Filetypes:', enabled: false},
      { id: 'basic', name: 'Remove Records From: Original Database/New Database', enabled: true },
      { id: 'filename', name: 'Duplicate Files: Keep/Trash/Delete ', enabled: false },
      { id: 'invalidpath', name: 'New Database Tag: ', enabled: false },
    ];

    let preset = ["default", "TJF"];

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




    <div class="block inner">
      <div class="grid">
        <span>Remove Records From: 
          <select class="select-field">
            {#each ["New Database", "Current Database"] as option}
            <option value={option}>{option}</option>
            {/each}
          </select>
      </span>
      <span>Duplicate Files On Disk:
        <select class="select-field">
          {#each ["Keep", "Move To Trash", "Permanently Delete"] as option}
          <option value={option}>{option}</option>
          {/each}
        </select> 
      </span>
        <span>New Database Tag: <input class="input-field" placeholder="-thinned"/> </span>
      
        <button type="button" class="grid item"  >
          {#if $preferencesStore.ignore_filetype}
              <CheckSquare size={20} class="checkbox checked" />
          {:else}
              <Square size={20} class="checkbox" />
          {/if}
          <span>Ignore Filetypes (extensions)</span>
      </button>
      </div>
    </div>
    <div class ="bar" style="margin-top: 20px;">
      <button class="cta-button small">Save:</button>
      <input type="text" class="input-field" placeholder="Enter New Configuraion Preset Name"  style=" margin-right: 20px;" /> 
      <button class="cta-button small">
      Load:
      </button>
      <select class="select-field"  style=" margin-right: 10px;"> 
        {#each preset as p}
          <option value={p}>{p}</option>
        {/each}
      </select>
      <button class="cta-button small cancel">
        Delete
        </button>
  
    </div>
</div> 

