<script lang="ts">
  import {
    filteredItemsStore,
    currentFilterStore,
    updateCurrentFilter,
    filtersStore,
  } from "../../stores/results";
  import { metadataStore } from "../../stores/metadata";
  import { isRemove } from "../../stores/menu";

  $: metadata = $metadataStore;
  $: filteredItems = $filteredItemsStore;
  $: currentFilter = $currentFilterStore;

  let loadingResults = true;
  let showLoadingOverlay = true;

  function handleFilterChange(event: Event) {
    const select = event.target as HTMLSelectElement;
    updateCurrentFilter(select.value);
  }

  $: filters = $filtersStore;

  function toggleMarkDirty() {
    metadataStore.update((p) => ({
      ...p,
      mark_dirty: !p.mark_dirty,
    }));
  }

  import { CheckSquare, Square } from "lucide-svelte";

  $: {
    if (filteredItems && filteredItems.length > 0 && loadingResults) {
      setTimeout(() => {
        loadingResults = false;
        showLoadingOverlay = false;
      }, 500);
    }
  }
</script>

<div class="filter-container">
  {#if isRemove}
    <span>Filter by: </span>
    <select
      class="select-field"
      bind:value={currentFilter}
      on:change={handleFilterChange}
    >
      {#each filters as option}
        {#if option.enabled}
          {#if option.id === "spacer"}
            <option disabled>{option.name}</option>
          {:else}
            <option value={option.id}>{option.name}</option>
          {/if}
        {/if}
      {/each}
    </select>
  {:else}
    <button
      type="button"
      class="grid item"
      style="margin-left: 120px"
      on:click={toggleMarkDirty}
    >
      {#if $metadataStore.mark_dirty}
        <CheckSquare
          size={20}
          class="checkbox checked {metadata.column == 'FilePath' ||
          metadata.column == 'Filename' ||
          metadata.column == 'Pathname'
            ? 'inactive'
            : ''}"
        />
      {:else}
        <Square size={20} class="checkbox" />
      {/if}
      <span
        class={metadata.column == "FilePath" ||
        metadata.column == "Filename" ||
        metadata.column == "Pathname"
          ? "inactive"
          : ""}>Mark Records as Dirty</span
      >
    </button>
  {/if}
</div>

<style>
  .filter-container {
    display: flex;
    align-items: center;
    gap: 5px;
    /* Keeps spacing between "Filter by:" and select */
    /* margin-left: auto; */
    /* Pushes the filter section to the right */
  }
</style>
