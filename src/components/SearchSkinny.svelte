<script lang="ts">
  import { X, SearchCheck, Square, CheckSquare } from "lucide-svelte";
  import { onMount, onDestroy } from "svelte";

  let isFinding = false;

  import { preferencesStore } from "../stores/preferences";
  import {
    getAlgorithmTooltip,
    toggleAlgorithm,
    getAlgoClass,
    checkAnyAlgorithmEnabled,
  } from "../stores/algorithms";
  import { databaseStore, openSqliteFile } from "../stores/database";
  $: database = $databaseStore;
  import {
    isSearching,
    initializeSearchListeners,
    toggleSearch, // Import the moved functions
  } from "../stores/status";

  import { basename, extname } from "@tauri-apps/api/path";

  async function getFilenameWithoutExtension(fullPath: string) {
    const name = await basename(fullPath); // Extracts filename with extension
    const ext = await extname(fullPath); // Extracts extension
    return name.replace(ext, ""); // Removes extension
  }

  $: isBasicEnabled =
    $preferencesStore?.algorithms?.find((a) => a.id === "basic")?.enabled ||
    false;

  onMount(() => {
    initializeSearchListeners();

    console.log("SearchSkinny component mounted, isSearching:", $isSearching);

    const unsubscribe = isSearching.subscribe((value) => {
      console.log("SearchSkinny: isSearching changed:", value);
    });

    return () => {
      unsubscribe();
    };
  });
</script>

<div class="page-columns">
  <div class="block">
    <div class="header">
      <!-- <h2>Algorithms</h2> -->
      {#if database == null || database.name == "" || database.name == "Select Database" || !checkAnyAlgorithmEnabled()}
        <button class="cta-button inactive">
          <SearchCheck size={18} />
          <span>Search for Records</span>
        </button>
      {:else}
        <button
          class="cta-button {$isSearching ? 'cancel' : ''}"
          style="width: 100vw;"
          on:click={toggleSearch}
        >
          <div class="flex items-center gap-2">
            {#if $isSearching}
              <X size={18} />
              <span>Cancel</span>
            {:else}
              <SearchCheck size={18} />
              <span>Search for Records</span>
            {/if}
          </div>
        </button>
      {/if}
    </div>

    <div class="grid">
      {#each $preferencesStore.algorithms as algo}
        <div
          class="grid item {getAlgoClass(algo, $preferencesStore.algorithms)}"
        >
          <button
            type="button"
            class="grid item"
            on:click={() => toggleAlgorithm(algo.id)}
          >
            {#if algo.id === "audiosuite" || algo.id === "filename"}
              <span style="margin-right: 20px;"></span>
            {/if}

            {#if algo.enabled}
              <CheckSquare
                size={20}
                class="checkbox {(algo.id === 'audiosuite' ||
                  algo.id === 'filename') &&
                !isBasicEnabled
                  ? 'inactive'
                  : 'checked'}"
              />
            {:else}
              <Square size={20} class="checkbox inactive" />
            {/if}

            <span
              class="tooltip-trigger {(algo.id === 'audiosuite' ||
                algo.id === 'filename') &&
              !isBasicEnabled
                ? 'inactive'
                : ''}"
            >
              {algo.name}
              <span class="tooltip-text">{getAlgorithmTooltip(algo.id)} </span>
            </span>
          </button>

          {#if algo.id === "duration"}
            <input
              type="number"
              min="0"
              step="0.1"
              bind:value={algo.min_dur}
              class="duration-input"
              style="width: 55px; background-color: var(--primary-bg)"
            />
            s
          {/if}
        </div>
        {#if algo.id === "dbcompare"}
          {#if algo.db !== null && algo.db !== undefined}
            {#await getFilenameWithoutExtension(algo.db) then filename}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_no_static_element_interactions -->
              <span
                class="clickable"
                style="margin-left: 40px"
                on:click={openSqliteFile}>{filename}</span
              >
            {/await}
          {:else}
            <button type="button" class="small-button" on:click={openSqliteFile}
              >Select DB</button
            >
          {/if}
        {/if}
      {/each}
    </div>

    <span style="margin-left: 255px"> </span>
  </div>
</div>

<style>
  .grid {
    grid-template-columns: repeat(1, 1fr);
    /* margin-top: -10px; */
    gap: 0.5rem;
  }

  .page-columns {
    display: grid;
    grid-template-columns: repeat(1, 1fr); /* 3 equal columns */
    gap: 10px;
  }

  :global(.checkbox.checked) {
    color: var(--accent-color);
    min-width: 20px;
    min-height: 20px;
    width: 20px;
    height: 20px;
    flex-shrink: 0;
  }

  :global(.checkbox.inactive) {
    color: var(--inactive-color);
    min-width: 20px;
    min-height: 20px;
    width: 20px;
    height: 20px;
    flex-shrink: 0;
  }

  .block {
    padding: 20px 20px;
    border-radius: 8px;
    margin-bottom: 0px;
    height: calc(100vh - 110px);
    width: 20vw;
  }

  .clickable {
    color: var(--text-color);
    cursor: pointer;
    text-decoration: none;
  }

  .clickable:hover {
    color: var(--hover-color);
  }
</style>
