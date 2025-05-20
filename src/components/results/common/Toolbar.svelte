<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";

  import { preferencesStore } from "../../../stores/preferences";
  import {
    resultsStore,
    filteredItemsStore,
    selectedItemsStore,
    currentFilterStore,
    enableSelectionsStore,
    clearSelected,
    invertSelected,
    toggleSelect,
    toggleChecked,
    checkSelected,
    uncheckSelected,
    toggleChecksSelected,
    totalChecksStore,
    selectedChecksStore,
    updateCurrentFilter,
    filtersStore,
  } from "../../../stores/results";
  import { metadataStore } from "../../../stores/metadata";
  import { setDatabase } from "../../../stores/database";
  import { showSearchView } from "../../../stores/menu";
  import { ask, message } from "@tauri-apps/plugin-dialog";

  export let isRemove: boolean;
  export let selectedDb: string | null = null;

  $: pref = $preferencesStore;
  $: results = $resultsStore;
  $: metadata = $metadataStore;
  $: filteredItems = $filteredItemsStore;
  $: selectedItems = $selectedItemsStore;
  $: currentFilter = $currentFilterStore;
  $: enableSelections = $enableSelectionsStore;

  let processing = false;
  let loading = true;
  let idsToRemove: number[] = [];
  let filesToRemove: string[] = [];
  let dualMono: { id: number; path: string }[] = [];

  let processingBatch = false;
  let loadingResults = true;
  let showLoadingOverlay = true;

  function updateUI() {
    results = [...results];
  }

  function handleFilterChange(event: Event) {
    const select = event.target as HTMLSelectElement;
    updateCurrentFilter(select.value);
  }

  async function replaceMetadata() {
    const confirmed = await ask("Are you sure? This is NOT undoable", {
      title: "⚠️ Confirm Replace",
      kind: "warning",
      okLabel: "Yes",
      cancelLabel: "Cancel",
    });

    if (confirmed && metadata.find && metadata.replace) {
      await invoke("replace_metadata", {
        data: metadata,
      })
        .then(() => {
          console.log("Successfully replaced metadata");
          metadata.find = "";
          metadata.replace = "";
          results = [];
          showSearchView;
        })
        .catch((error) => {
          console.error("Error replacing metadata:", error);
        });
    }
  }

  async function fetchData() {
    try {
      loading = true;
    } catch (error) {
      console.error("Failed to fetch data:", error);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    setTimeout(() => {
      loading = false;
    }, 100);

    activateResultsTab();
  });

  $: filters = $filtersStore;

  async function confirmDialog() {
    let dbDialog = "Create Safety Copy";
    if (!pref.safety_db) dbDialog = "❌ Current Database";

    let filesDialog = "Keep in Place";
    if (pref.erase_files === "Trash") filesDialog = "⚠️ Move to Trash";
    else if (pref.erase_files === "Delete")
      filesDialog = "❌ Permanently Delete";

    let dualMonoDialog = "Leave Unchanged";
    if (pref.strip_dual_mono) dualMonoDialog = "❌ Convert to Mono";

    let warningDialog = "";
    if (
      pref.erase_files === "Delete" ||
      !pref.safety_db ||
      pref.strip_dual_mono
    ) {
      warningDialog = "\n\n⚠️ Are you sure? This is NOT undoable!";
    }

    let titleDialog = "Confirm Remove";
    if (pref.erase_files === "Delete" || !pref.safety_db) {
      titleDialog = "Confirm Remove";
    }

    let dialog = `Files on Disk: ${filesDialog}\nDatabase: ${dbDialog}\nDualMono Files: ${dualMonoDialog} ${warningDialog}`;

    const confirmed = await ask(dialog, {
      title: titleDialog,
      kind: "warning",
      okLabel: "Yes",
      cancelLabel: "Cancel",
    });

    return confirmed;
  }

  async function removeRecords() {
    idsToRemove = filteredItems
      .filter((item) => !item.algorithm.includes("Keep"))
      .map((item) => item.id);
    filesToRemove = filteredItems
      .filter((item) => !item.algorithm.includes("Keep"))
      .map((item) => item.path + "/" + item.filename);

    dualMono = filteredItems
      .filter((item) => item.algorithm.includes("DualMono"))
      .map((item) => ({ id: item.id, path: item.path + "/" + item.filename }));

    if (idsToRemove.length > 0 || dualMono.length > 0) {
      if (!(await confirmDialog())) return;
      processing = true;
      await invoke<string>("remove_records", {
        records: idsToRemove,
        clone: pref.safety_db,
        cloneTag: pref.safety_db_tag,
        delete: pref.erase_files,
        files: filesToRemove,
        dualMono: dualMono,
        stripDualMono: pref.strip_dual_mono,
      })
        .then((updatedDb) => {
          if (dualMono.length > 0 && pref.strip_dual_mono) {
            message(
              "Dual Mono files converted to Mono!\n\nRecords marked as dirty in Soundminer. For safety, open Soundminer and run the following:\n'Database -> Show Dirty'\nPress: 'CMD + A' to select all\n'Database -> Embed Selected'\n'Database -> Rebuild Waveforms for Selected'"
            );
          }
          console.log("Successfully removed records with IDs:", idsToRemove);
          processing = false;
          setDatabase(updatedDb, false);
          showSearchView();
        })
        .catch((error) => {
          console.error("Error removing records:", error);
          processing = false;
        });
    } else {
      console.log("No records to remove");
      await message("No records to remove!");
    }
  }

  async function removeSelectedRecords() {
    idsToRemove = filteredItems
      .filter(
        (item) => !item.algorithm.includes("Keep") && selectedItems.has(item.id)
      )
      .map((item) => item.id);
    filesToRemove = filteredItems
      .filter(
        (item) => !item.algorithm.includes("Keep") && selectedItems.has(item.id)
      )
      .map((item) => item.path + "/" + item.filename);

    dualMono = filteredItems
      .filter(
        (item) =>
          item.algorithm.includes("DualMono") && selectedItems.has(item.id)
      )
      .map((item) => ({ id: item.id, path: item.path + "/" + item.filename }));

    if (idsToRemove.length > 0 || dualMono.length > 0) {
      if (!(await confirmDialog())) return;
      processing = true;
      await invoke<string>("remove_records", {
        records: idsToRemove,
        clone: pref.safety_db,
        cloneTag: pref.safety_db_tag,
        delete: pref.erase_files,
        files: filesToRemove,
        dualMono: dualMono,
        stripDualMono: pref.strip_dual_mono,
      })
        .then((updatedDb) => {
          if (dualMono.length > 0 && pref.strip_dual_mono) {
            message(
              "Dual Mono files converted to Mono!\n\nRecords marked as dirty in Soundminer. For safety, open Soundminer and run the following:\n'Database -> Show Dirty'\nPress: 'CMD + A' to select all\n'Database -> Embed Selected'\n'Database -> Rebuild Waveforms for Selected'"
            );
          }
          console.log("Successfully removed records with IDs:", idsToRemove);
          selectedDb = updatedDb;
        })
        .catch((error) => {
          console.error("Error removing records:", error);
          processing = false;
        });
    } else {
      console.log("No records to remove");
      await message("No records to remove!");
    }
  }

  function toggleMarkDirty() {
    metadataStore.update((p) => ({
      ...p,
      mark_dirty: !p.mark_dirty,
    }));
  }

  let removeProgress = 0;
  let removeMessage = "Initializing...";
  let removeStage = "";
  let unlistenRemoveFn: () => void;

  onMount(async () => {
    unlistenRemoveFn = await listen<{
      progress: number;
      message: string;
      stage: string;
    }>("remove-status", (event) => {
      const status = event.payload;
      removeProgress = status.progress;
      removeMessage = status.message;
      removeStage = status.stage;
      console.log(
        `Remove status: ${status.stage} - ${status.progress}% - ${status.message}`
      );
      if (status.stage === "complete") {
        processing = false;
        fetchData();
      }
    });
  });

  onDestroy(() => {
    if (unlistenRemoveFn) unlistenRemoveFn();
  });

  import {
    CheckSquare,
    Square,
    NotebookPenIcon,
    OctagonX,
    TriangleAlert,
    Loader,
  } from "lucide-svelte";

  function algoEnabled(algo: string): boolean {
    const algorithm = pref.algorithms.find((option) => option.id === algo);
    return algorithm?.enabled || false;
  }

  async function checkSelectedWithIndicator() {
    processingBatch = true;
    setTimeout(() => {
      try {
        checkSelected();
      } finally {
        processingBatch = false;
      }
    }, 10);
  }

  function activateResultsTab() {
    loadingResults = true;
    showLoadingOverlay = true;

    setTimeout(() => {
      const timer = setTimeout(() => {
        loadingResults = false;
        showLoadingOverlay = false;
      }, 500);

      return () => clearTimeout(timer);
    }, 100);
  }

  $: {
    if (filteredItems && filteredItems.length > 0 && loadingResults) {
      setTimeout(() => {
        loadingResults = false;
        showLoadingOverlay = false;
      }, 500);
    }
  }
</script>

<div class="bar" style="margin-top: 10px; margin-bottom: 20px; padding: 0px;">
  {#if enableSelections}
    <button class="small-button" on:click={toggleChecksSelected}
      >Toggle Selected</button
    >
    <button class="small-button" on:click={checkSelectedWithIndicator}
      >Check Selected</button
    >
    <button class="small-button" on:click={uncheckSelected}
      >Uncheck Selected</button
    >
    <button class="small-button" on:click={invertSelected}
      >Invert Selections</button
    >
    <button class="small-button" on:click={clearSelected}
      >Clear Selections</button
    >
    {#if selectedItems.size > 0}
      <p style="margin-left: 10px">({selectedItems.size} selected)</p>
    {/if}
    {#if processingBatch}
      <div class="batch-processing">
        <Loader size={24} class="spinner" />
        <span>Processing {selectedItems.size} items...</span>
      </div>
    {/if}
  {/if}

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
</div>
