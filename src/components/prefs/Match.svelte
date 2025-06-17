<script lang="ts">
  import VirtualList from "svelte-virtual-list";
  import { Square, CheckSquare, OctagonX } from "lucide-svelte";
  // Import from main store instead
  import {
    preferencesStore,
    match_criteria_add,
    match_criteria_remove,
  } from "../../stores/preferences";

  // Use the store directly instead of assigning to `pref`
  let currentColumn = "";
  let selectedMatches = new Set<string>();

  function toggleignore_filetype() {
    preferencesStore.update((p) => ({
      ...p,
      ignore_filetype: !p.ignore_filetype,
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
</script>

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
    <button type="button" class="grid item" on:click={toggleignore_filetype}>
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
    <VirtualList items={Array.from($preferencesStore.match_criteria)} let:item>
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
