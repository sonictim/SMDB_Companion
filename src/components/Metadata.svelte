<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  import {
    X,
    CheckSquare,
    Square,
    Search,
    NotebookPenIcon,
  } from "lucide-svelte";

  export let activeTab: string; // This prop is now bindable
  export let isRemove: boolean;
  export let selectedDb: string | null;

  import type { Preferences } from "../stores/types";
  import { preferencesStore } from "../stores/preferences";
  import { get } from "svelte/store";
  let pref: Preferences = get(preferencesStore);

  let findText = "";
  let replaceText = "";
  let isCaseSensitive = false;
  let selectedColumn = "FilePath"; // Default option

  async function replaceMetadata() {
    isRemove = false;
    // Your logic for replacing metadata goes here
    console.log(
      `Finding: ${findText}, Replacing: ${replaceText}, Case Sensitive: ${isCaseSensitive}, Column: ${selectedColumn}`
    );
    await invoke<string>("find", {
      find: findText,
      column: selectedColumn,
      caseSensitive: isCaseSensitive,
    })
      .then((result) => {
        console.log(result);
      })
      .catch((error) => console.error(error));

    activeTab = "results";
  }

  function toggleCaseSensitivity() {
    isCaseSensitive = !isCaseSensitive;
  }

  function checkDB(): boolean {
    return selectedDb === "Select Database";
  }
</script>

<div class="block">
  <div class="header">
    <h2>Metadata Replacement</h2>
    {#if selectedDb == null}
      <button class="cta-button inactive">
        <Search size={18} />
        <span> Find Records </span>
      </button>
    {:else}
      <button class="cta-button" on:click={replaceMetadata}>
        <Search size={18} />
        <span> Find Records </span>
      </button>
    {/if}
  </div>
  <div class="input-group2">
    <label for="case-sensitive">
      <button type="button" class="grid item" on:click={toggleCaseSensitivity}>
        {#if isCaseSensitive}
          <CheckSquare size={20} class="checkbox checked" />
        {:else}
          <Square size={20} class="checkbox" />
        {/if}
        <span>Case Sensitive</span>
      </button>
    </label>
  </div>

  <div class="input-group">
    <label for="find-text">Find:</label>
    <input
      type="text"
      id="find-text"
      bind:value={findText}
      placeholder="Enter text to find"
      class="input-field"
    />
  </div>

  <div class="input-group">
    <label for="replace-text">Replace:</label>
    <input
      type="text"
      id="replace-text"
      bind:value={replaceText}
      placeholder="Enter text to replace"
      class="input-field"
    />
  </div>

  <div class="input-group">
    <label for="column-select">in Column:</label>
    <select id="column-select" bind:value={selectedColumn} class="select-field">
      {#each pref.columns as option}
        <option value={option}>{option}</option>
      {/each}
    </select>
  </div>
</div>
<!-- <button class="cta-button cancel" on:click={replaceMetadata}>
        <NotebookPenIcon size=18/>
        <span>
            Replace
        </span>
    </button> -->
