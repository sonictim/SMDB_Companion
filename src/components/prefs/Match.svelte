<script lang="ts">
  import VirtualList from "svelte-virtual-list";
  import { Square, CheckSquare, OctagonX } from "lucide-svelte";
  // Import from main store instead
  import {
    preferencesStore,
    updateSimilarityThreshold,
    updateWaveformSearchType,
    match_criteria_add,
    match_criteria_remove,
    safe_folder_remove,
    update_batch_size,
    addSafeFolder,
  } from "../../stores/preferences";
  import { invoke } from "@tauri-apps/api/core";

  // Use the store directly instead of assigning to `pref`
  let currentColumn = "";
  $: pref = $preferencesStore;
  let selectedMatches = new Set<string>();
  let selectedSafeMatches = new Set<string>();
  let isRemoving = false;

  function toggleignore_filetype() {
    preferencesStore.update((p) => ({
      ...p,
      ignore_filetype: !p.ignore_filetype,
    }));
  }
  function toggleStoreWaveforms() {
    preferencesStore.update((p) => ({
      ...p,
      store_waveforms: !p.store_waveforms,
    }));
  }
  function toggleFetchWaveforms() {
    preferencesStore.update((p) => ({
      ...p,
      fetch_waveforms: !p.fetch_waveforms,
    }));
  }

  function toggleMatch(item: string) {
    if (selectedMatches.has(item)) {
      selectedMatches.delete(item);
    } else {
      selectedMatches.add(item);
    }
    selectedMatches = new Set(selectedMatches); // Ensure reactivity
  }
  function toggleSafeMatch(item: string) {
    if (selectedSafeMatches.has(item)) {
      selectedSafeMatches.delete(item);
    } else {
      selectedSafeMatches.add(item);
    }
    selectedSafeMatches = new Set(selectedSafeMatches); // Ensure reactivity
  }

  function removeMatches(list: string[]) {
    list.forEach((item) => match_criteria_remove(item));
    clearMatches();
  }
  function removeSafeFolders(list: string[]) {
    list.forEach((item) => safe_folder_remove(item));
    clearSafeMatches();
  }

  function clearMatches() {
    selectedMatches.clear();
    selectedMatches = new Set(); // Ensure reactivity
  }
  function clearSafeMatches() {
    selectedSafeMatches.clear();
    selectedSafeMatches = new Set(); // Ensure reactivity
  }

  function addColumn() {
    match_criteria_add(currentColumn);
    currentColumn = "";
  }

  function handleColumnChange(event: Event) {
    currentColumn = (event.target as HTMLSelectElement).value;
  }

  // Get filtered columns that are not in match_criteria
  $: filteredColumns = $preferencesStore.columns.filter(
    (col) => !$preferencesStore.match_criteria.includes(col)
  );

  let confirmRemove = false;

  async function clearFingerprints() {
    isRemoving = true;
    await invoke("clear_fingerprints")
      .then(() => {
        console.log("Successfully cleared fingerprints");
        confirmRemove = false;
        isRemoving = false;
      })
      .catch((error) => {
        console.error("Error clearing fingerprints:", error);
        confirmRemove = false;
        isRemoving = false;
      });
  }

  function getAlgorithmTooltip(id: string): string {
    const tooltips: Record<string, string> = {
      Exact:
        "Exact Match: Finds identical audio files with different filenames",
      Similar:
        "Relative Match: Finds similar audio files with different filenames using a threshold comparison.  Helpful for finding files that have been altered from their source",
      Subset:
        "Subset Match: Finds audio files that are smaller piece of a longer audio files",
    };

    return tooltips[id] || "No description available";
  }

  // Wrapper functions for type conversions
  function handleWaveformSearchTypeChange(event: Event): void {
    updateWaveformSearchType((event.target as HTMLSelectElement).value);
  }

  function handleBatchSizeChange(event: Event): void {
    const newValue = parseFloat((event.target as HTMLInputElement).value);
    console.log("🔧 [PREFS] Batch size changed in UI to:", newValue);
    update_batch_size(newValue);
  }

  function handleSimilarityThresholdChange(event: Event): void {
    updateSimilarityThreshold(
      parseFloat((event.target as HTMLInputElement).value)
    );
  }
</script>

<div class="grid-container">
  <div class="page-columns">
    <div class="block">
      <div class="header">
        <span class="tooltip-trigger">
          <span class="tooltip-text">
            A list of all the required matches for a file to be considered a
            duplicate of another.
          </span>
          <h2>Duplicate Match Criteria</h2>
        </span>
        <button
          class="cta-button cancel"
          on:click={() => removeMatches([...selectedMatches])}
        >
          <OctagonX size="18" />
          Remove Selected
        </button>
      </div>

      <div class="header">
        <button class="cta-button small" on:click={addColumn}>Add</button>
        <select
          class="select-field"
          bind:value={currentColumn}
          on:change={handleColumnChange}
        >
          {#each filteredColumns as option}
            <option value={option}>{option}</option>
          {/each}
        </select>
        <button
          type="button"
          class="grid item"
          on:click={toggleignore_filetype}
        >
          {#if $preferencesStore.ignore_filetype}
            <CheckSquare size={20} class="checkbox checked" />
          {:else}
            <Square size={20} class="checkbox" />
          {/if}
          <span>Ignore Filetypes (extensions)</span>
        </button>
      </div>
      <div class="block inner">
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <VirtualList
          items={Array.from($preferencesStore.match_criteria)}
          let:item
        >
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <div
            on:click={() => toggleMatch(item)}
            class="list-item"
            class:selected-item={selectedMatches.has(item)}
            class:unselected-item={!selectedMatches.has(item)}
          >
            {item}
          </div>
        </VirtualList>
      </div>
    </div>
    <div class="block">
      <div class="header">
        <span class="tooltip-trigger">
          <span class="tooltip-text" style="height: 340%;">
            Files in safe folders are completely left out of any duplicate
            search. Useful for folders that contain files you do not want to
            protect or files you may not mind having as duplicates in your
            library, such as sampler instrument libraries.
          </span>
          <h2>Safe/Ignore Folders</h2>
        </span>
        <span>
          <button
            class="cta-button cancel"
            on:click={() => removeSafeFolders([...selectedSafeMatches])}
          >
            <OctagonX size="18" />
            Remove Selected
          </button>
        </span>
      </div>

      <div class="header">
        <div class="button-group">
          <button class="cta-button small" on:click={addSafeFolder}>Add</button>
        </div>
      </div>
      <div class="block inner">
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <VirtualList
          items={Array.from($preferencesStore.safe_folders)}
          let:item
        >
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <div
            on:click={() => toggleSafeMatch(item)}
            class="list-item"
            class:selected-item={selectedSafeMatches.has(item)}
            class:unselected-item={!selectedSafeMatches.has(item)}
          >
            {item}
          </div>
        </VirtualList>
      </div>
    </div>
  </div>

  <div class="block">
    <div class="header">
      <h2>Audio Content Search Options</h2>
    </div>
    <div class="grid">
      <span>
        <button type="button" class="grid item" on:click={toggleStoreWaveforms}>
          {#if $preferencesStore.store_waveforms}
            <CheckSquare size={20} class="checkbox checked" />
          {:else}
            <Square size={20} class="checkbox" />
          {/if}
          <span>Store audio fingerprints in database</span>
        </button>
        <span style="margin-left: 50px">
          Every
          <input
            type="number"
            class="input-field"
            style="width: 80px"
            placeholder="1000"
            step="100"
            min="100"
            max="10000"
            value={$preferencesStore.batch_size}
            on:input={handleBatchSizeChange}
          />
          records
          <!-- <span class="inactive" style="margin-left: 5px"> 0-100%</span> -->
        </span>
      </span>
      <span class="tooltip-trigger">
        Compare Algorithm:
        <select
          class="select-field"
          bind:value={$preferencesStore.waveform_search_type}
          on:change={handleWaveformSearchTypeChange}
        >
          {#each [{ text: "Exact Match", val: "Exact" }, { text: "Relative Match", val: "Similar" }] as { text, val }}
            <!-- {#each [{ text: "Exact Match", val: "Exact" }, { text: "Relative Match", val: "Similar" }, { text: "Subset Match", val: "Subset" }] as { text, val }} -->
            <option value={val}>{text}</option>
          {/each}
        </select>
        <span class="tooltip-text" style="height: 90%;">
          {getAlgorithmTooltip($preferencesStore.waveform_search_type)}
        </span>
      </span>

      <span>
        <button type="button" class="grid item" on:click={toggleFetchWaveforms}>
          {#if $preferencesStore.fetch_waveforms}
            <CheckSquare size={20} class="checkbox checked" />
          {:else}
            <Square size={20} class="checkbox" />
          {/if}
          <span>Fetch stored audio fingerprints from database</span>
        </button>
      </span>
      {#if pref.waveform_search_type != "Exact"}
        <span style="margin-left: 70px">
          Threshold:
          <input
            type="number"
            class="input-field"
            style="width: 100px"
            placeholder="90"
            step="1"
            min="0"
            max="100"
            value={$preferencesStore.similarity_threshold}
            on:input={handleSimilarityThresholdChange}
          />
          <span class="inactive" style="margin-left: 5px"> 0-100%</span>
        </span>
      {:else}
        <span></span>
      {/if}
      <span>
        {#if isRemoving}
          <p class="ellipsis">Clearing fingerprints from database</p>
        {:else}
          <button
            class="cta-button small cancel"
            on:click={() => (confirmRemove = true)}
          >
            Clear Audio Fingerprints from Database
          </button>
        {/if}
      </span>
      {#if confirmRemove}
        <span>
          Are you sure? This is not undoable!
          <button
            class="cta-button small"
            on:click={() => (confirmRemove = false)}
          >
            Cancel
          </button>
          <button class="cta-button small cancel" on:click={clearFingerprints}>
            Confirm
          </button>
        </span>
      {/if}
    </div>
  </div>
</div>

<style>
  .block {
    height: 100%;
    margin-top: 0;
    margin-bottom: 0;
    /* padding: 15px; */
    flex: 1;
  }
  .page-columns {
    display: grid;
    grid-template-columns: 1fr 1fr; /* Left, center, right */
    height: 100%;
    /* gap: 20px; */
  }

  .grid-container {
    height: 100%;
    display: grid;
    grid-template-columns: 1fr;
    grid-template-rows: 2fr 1fr;
    gap: 0px;
    margin-bottom: 0px;
  }

  .tooltip-trigger {
    position: relative;
    display: inline-flex;
    align-items: center;
  }

  .tooltip-trigger:hover .tooltip-text {
    visibility: visible;
    opacity: 1;
  }

  .tooltip-text {
    visibility: hidden;
    width: 220px;
    background-color: var(--tooltip-bg, #333);
    color: var(--text-color);
    text-align: center;
    border-radius: 6px;
    padding: calc(var(--font-size) / 2);
    position: absolute;
    z-index: 10001;
    top: 125%;
    left: 50%;
    transform: translateX(-50%);
    opacity: 0;
    transition: opacity 0.3s;
    font-size: var(--font-size-xs);
    line-height: 1.4; /* Add proper line height */
    pointer-events: none;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
    white-space: normal;
    word-wrap: break-word;
    overflow-wrap: break-word; /* Additional word breaking */
    height: 200%;
  }

  .tooltip-text::after {
    content: "";
    position: absolute;
    bottom: 100%;
    left: 50%;
    margin-left: -calc(var(--font-size) / 4);
    border-width: calc(var(--font-size) / 4);
    border-style: solid;
    border-color: transparent transparent var(--tooltip-bg, #333) transparent;
  }
</style>
