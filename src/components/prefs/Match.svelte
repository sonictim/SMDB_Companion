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
    update_batch_size,
  } from "../../stores/preferences";
  import { invoke } from "@tauri-apps/api/core";

  // Use the store directly instead of assigning to `pref`
  let currentColumn = "";
  $: pref = $preferencesStore;
  let selectedMatches = new Set<string>();
  let waveform_match = false;
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
  function toggle_all_records() {
    preferencesStore.update((p) => ({
      ...p,
      display_all_records: !p.display_all_records,
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

  function removeMatches(list: string[]) {
    list.forEach((item) => match_criteria_remove(item));
    clearMatches();
  }

  function clearMatches() {
    selectedMatches.clear();
    selectedMatches = new Set(); // Ensure reactivity
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
</script>

<div class="grid-container">
  <div class="block">
    <div class="header">
      <h2>Duplicate Match Criteria</h2>
      <button
        class="cta-button cancel"
        on:click={() => removeMatches([...selectedMatches])}
      >
        <OctagonX size="18" />
        Remove Selected
      </button>
    </div>

    <div class="header">
      <div class="button-group">
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
      </div>
      <button
        type="button"
        class="grid item"
        style="margin-left: 120px"
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
  <div class="block" style=" height: 30vh;">
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
            min="0"
            max="10000"
            value={$preferencesStore.batch_size}
            on:input={(e) =>
              update_batch_size(
                parseFloat((e.target as HTMLInputElement).value)
              )}
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
          on:change={(e) =>
            updateWaveformSearchType((e.target as HTMLSelectElement).value)}
        >
          {#each [{ text: "Exact Match", val: "Exact" }, { text: "Relative Match", val: "Similar" }] as { text, val }}
            <!-- {#each [{ text: "Exact Match", val: "Exact" }, { text: "Relative Match", val: "Similar" }, { text: "Subset Match", val: "Subset" }] as { text, val }} -->
            <option value={val}>{text}</option>
          {/each}
        </select>
        <span class="tooltip-text">
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
            on:input={(e) =>
              updateSimilarityThreshold(
                parseFloat((e.target as HTMLInputElement).value)
              )}
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
    height: calc(80vh - 250px);
  }

  .grid-container {
    height: calc(100vh - 160px);
    display: grid;
    grid-template-columns: 1fr;
    grid-template-rows: 2fr 1fr;
    gap: 20px;
  }

  .algorithm-help {
    font-size: 10px;
    /* margin-top: 8px; */
    /* margin-left: 70px; */
    /* padding: 8px; */
    /* background-color: #f5f5f5; */
    /* border-left: 3px solid #007bff; */
    /* font-size: 13px; */
    /* color: #555; */
    /* max-width: 400px; */
  }
</style>
