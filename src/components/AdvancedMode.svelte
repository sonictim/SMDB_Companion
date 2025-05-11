<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { Database, Search as SearchIcon, X } from "lucide-svelte";
  import { basename, extname } from "@tauri-apps/api/path";
  import {
    databaseStore,
    openDatabase,
    getCompareDb,
    setDatabase,
  } from "../stores/database";
  import type { FileRecord } from "../stores/types";
  import { preferencesStore } from "../stores/preferences";
  import { toggleAlgorithm, getAlgorithmTooltip } from "../stores/algorithms";
  import {
    resultsStore,
    filteredItemsStore,
    selectedItemsStore,
    enableSelectionsStore,
    toggleSelect,
    toggleChecked,
    totalChecksStore,
    selectedChecksStore, // Import the new store
    clearResults,
  } from "../stores/results";
  import {
    searchProgressStore,
    isSearching,
    initializeSearchListeners,
    toggleSearch,
  } from "../stores/status";

  import { metadataStore } from "../stores/metadata";
  import { ask, message } from "@tauri-apps/plugin-dialog";
  import { createVirtualizer } from "@tanstack/svelte-virtual";

  export let selectedDb: string | null = null;

  $: pref = $preferencesStore;
  $: filteredItems = $filteredItemsStore;
  $: selectedItems = $selectedItemsStore;
  $: enableSelections = $enableSelectionsStore;
  $: totalChecks = $totalChecksStore;
  $: selectedChecks = $selectedChecksStore; // Create a reactive reference to selected checks
  $: isBasicEnabled =
    $preferencesStore?.algorithms?.find((a) => a.id === "basic")?.enabled ||
    false;

  let processing = false;
  let loading = true;
  let idsToRemove: number[] = [];
  let filesToRemove: string[] = [];
  let dualMono: { id: number; path: string }[] = [];
  let lastPlayed = "Timbo";

  let columnConfigs = [
    { minWidth: 10, width: 30, name: "checkbox", header: "✔" },
    { minWidth: 100, width: 250, name: "filename", header: "Filename" },
    { minWidth: 150, width: 400, name: "path", header: "Path" },
    { minWidth: 100, width: 300, name: "description", header: "Description" },
    { minWidth: 20, width: 80, name: "algorithm", header: "Match" },
    { minWidth: 10, width: 25, name: "channels", header: "CH" },
    { minWidth: 10, width: 25, name: "bitdepth", header: "BD" },
    { minWidth: 10, width: 50, name: "samplerate", header: "SR" },
    { minWidth: 10, width: 80, name: "duration", header: "Duration" },
    { minWidth: 8, width: 30, name: "audio", header: "" },
  ];

  $: columnWidths = columnConfigs.map((config) => config.width);

  $: totalWidth =
    columnWidths.reduce((acc, width) => acc + width, 0) + 100 + "px";

  // Set CSS custom property when totalWidth changes
  $: {
    if (typeof document !== "undefined") {
      document.documentElement.style.setProperty("--total-width", totalWidth);
    }
  }

  let containerElement: HTMLElement;
  let containerWidth = 0;

  function startResize(index: number, event: MouseEvent) {
    event.preventDefault();

    const startX = event.clientX;
    const startWidth = columnConfigs[index].width;

    function onMouseMove(e: MouseEvent) {
      const diff = e.clientX - startX;
      const newWidth = Math.max(
        columnConfigs[index].minWidth,
        startWidth + diff
      );

      // Just update this single column's width
      const newConfigs = [...columnConfigs];
      newConfigs[index] = { ...newConfigs[index], width: newWidth };
      columnConfigs = newConfigs;
    }

    function onMouseUp() {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
    }

    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
  }

  function updateVirtualizer() {
    if ($rowVirtualizer) {
      const scrollElement = parentRef;
      const scrollTop = scrollElement?.scrollTop;

      $rowVirtualizer.measure();

      if (scrollTop !== undefined) {
        queueMicrotask(() => {
          if (scrollElement) scrollElement.scrollTop = scrollTop;
        });
      }
    }
  }

  $: {
    if ($selectedItemsStore) {
      updateVirtualizer();
    }
  }

  $: {
    // This reactive statement watches for filter changes
    const scrollElement = parentRef;
    const scrollTop = scrollElement?.scrollTop;

    queueMicrotask(() => {
      if (scrollElement && scrollTop !== undefined) {
        scrollElement.scrollTop = scrollTop;
      }
      updateVirtualizer();
    });
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
    loading = false;
    initializeSearchListeners();
    fetchData();

    // containerElement might not be assigned yet in onMount, use parentRef instead
    if (parentRef) {
      containerWidth = parentRef.clientWidth;
    }
  });

  let manualFilters = [
    { id: "All", name: "All Records", enabled: true },
    { id: "Relevant", name: "Relevant Records", enabled: true },
    { id: "Keep", name: "Records to Keep", enabled: true },
    { id: "Remove", name: "Records to Remove", enabled: true },
    { id: "spacer", name: "──────────", enabled: true },
  ];

  $: filters = [...manualFilters, ...pref.algorithms];

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
      .filter((item) => !item.algorithm.includes("Keep")) // Only keep items without "Keep"
      .map((item) => item.id); // Extract the ids
    filesToRemove = filteredItems
      .filter((item) => !item.algorithm.includes("Keep")) // Only keep items without "Keep"
      .map((item) => item.path + "/" + item.filename); // Extract the ids

    dualMono = filteredItems
      .filter((item) => item.algorithm.includes("DualMono")) // Only keep items with "Dual Mono"
      .map((item) => ({ id: item.id, path: item.path + "/" + item.filename })); // Extract the ids

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
          clearResults();
          setDatabase(updatedDb, false);
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
      ) // Only keep items without "Keep"
      .map((item) => item.id); // Extract the ids
    filesToRemove = filteredItems
      .filter(
        (item) => !item.algorithm.includes("Keep") && selectedItems.has(item.id)
      ) // Only keep items without "Keep"
      .map((item) => item.path + "/" + item.filename); // Extract the ids

    dualMono = filteredItems
      .filter(
        (item) =>
          item.algorithm.includes("DualMono") && selectedItems.has(item.id)
      ) // Only keep items with "Dual Mono"
      .map((item) => ({ id: item.id, path: item.path + "/" + item.filename })); // Extract the ids

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

  async function playAudioFile(record: FileRecord) {
    console.log("last played: ", lastPlayed);
    let filePath = record.path + "/" + record.filename;
    if (lastPlayed === filePath) {
      console.log("Stopping audio playback for:", filePath);
      await stopAudioFile();
      return;
    }
    lastPlayed = filePath;

    console.log("playing audio:", filePath);
    await invoke("play_audio", { path: filePath })
      .then(() => {
        console.log("Success:", filePath);
      })
      .catch((error) => {
        console.error("Error calling audio playback:", error);
      });
  }
  async function stopAudioFile() {
    lastPlayed = "";
    console.log("Stopping Audio Playback");
    await invoke("stop_audio")
      .then(() => {
        console.log("Success: Stopped audio playback");
      })
      .catch((error) => {
        console.error("Error stopping audio playback:", error);
      });
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
    SquareEqual,
    OctagonX,
    Volume,
    Loader,
    Copy,
    FileX2,
    Tag,
    AudioWaveform,
    Clock,
    GitCompareArrowsIcon,
    Hash,
    ShieldCheck,
    Search,
    Activity,
    ArrowLeftRight,
  } from "lucide-svelte";
  function getAlgorithmIcon(algoName: string) {
    const iconMap: Record<
      string,
      { component: any; tooltip: string; color?: string }
    > = {
      Keep: {
        component: ShieldCheck,
        tooltip: "Keep",
        color: "var(--success-color)",
      },
      Basic: { component: Copy, tooltip: "Duplicate Match" },
      InvalidPath: { component: FileX2, tooltip: "Invalid Path" },
      SimilarFilename: {
        component: Search,
        tooltip: "Similar Filename",
      },
      Tags: { component: Tag, tooltip: "Duplicate contains Tag" },
      FileTags: { component: Tag, tooltip: "Filename contains tag" },
      Waveforms: { component: AudioWaveform, tooltip: "Waveform Match" },
      Duration: { component: Clock, tooltip: "Duration Match" },
      Compare: { component: GitCompareArrowsIcon, tooltip: "Database Compare" },
      SimilarAudio: { component: Activity, tooltip: "Similar Audio" },
      ExactPCM: { component: AudioWaveform, tooltip: "Exact PCM Hash" },
      DualMono: { component: SquareEqual, tooltip: "Dual Mono" },
      Replace: { component: ArrowLeftRight, tooltip: "Replace Metadata" },
      Remove: {
        component: OctagonX,
        tooltip: "Marked for Removal",
        color: "var(--error-color)",
      },
    };

    return iconMap[algoName] || { component: Hash, tooltip: algoName };
  }

  let parentRef: Element;
  let parentWidth = 0;
  let parentHeight = 0;
  export let estimatedItemSize = 40;
  export let overscan = 5;

  // Create vertical virtualizer for rows
  $: rowVirtualizer = createVirtualizer({
    count: filteredItems.length,
    estimateSize: () => estimatedItemSize,
    overscan,
    getScrollElement: () => parentRef,
  });

  onMount(() => {
    const resizeObserver = new ResizeObserver((entries) => {
      if (entries[0]) {
        parentWidth = entries[0].contentRect.width;
        parentHeight = entries[0].contentRect.height;
        if ($rowVirtualizer) {
          $rowVirtualizer.measure();
        }
      }
    });

    if (parentRef) {
      resizeObserver.observe(parentRef);
    }

    return () => {
      resizeObserver.disconnect();
    };
  });

  // Replace static grid-template-columns with dynamic version
  $: gridTemplateColumns = columnWidths.map((width) => `${width}px`).join(" ");

  $: {
    console.log("Grid Template Columns:", gridTemplateColumns);
  }

  let processingBatch = false;

  function getAlgoClass(algo: { id: string }, algorithms: any[]) {
    if (
      (algo.id === "audiosuite" || algo.id === "filename") &&
      !algorithms.find((a) => a.id === "basic")?.enabled
    ) {
      return "inactive";
    }
    return "";
  }

  async function getFilenameWithoutExtension(fullPath: string) {
    const name = await basename(fullPath); // Extracts filename with extension
    const ext = await extname(fullPath); // Extracts extension
    return name.replace(ext, ""); // Removes extension
  }
</script>

<div class="top-bar">
  <div class="top-bar-left">
    <button class="nav-link" on:click={() => openDatabase(false)}>
      <Database size={18} />
      <span style="font-size: 24px;">
        {$databaseStore?.name || "Select Database"}
        {#if $databaseStore}
          <span style="font-size: 14px;"
            >{$databaseStore.size} total records</span
          >
        {/if}
        {#if selectedItems.size > 0}
          <span style="font-size: 14px;">({selectedItems.size} selected)</span>
        {/if}
      </span>
    </button>
  </div>
  <div class="top-bar-right">
    {#if $databaseStore}
      <button
        class="nav-link"
        on:click={async () => {
          toggleSearch();
        }}
        title="Search for Duplicates"
      >
        <div class="flex items-center gap-2">
          {#if $isSearching}
            <X size={18} />
            <span>Cancel Search</span>
          {:else}
            <SearchIcon size={18} />
            <span>Search for Records</span>
          {/if}
        </div>
      </button>
    {/if}
    {#if $resultsStore.length > 0}
      <button
        class="nav-link"
        on:click={removeRecords}
        title="Remove Duplicates"
      >
        <div class="flex items-center gap-2">
          <OctagonX size={18} />
          {#if selectedItems.size > 0}
            Remove {selectedChecks} Selected Records
          {:else}
            <span>Remove {totalChecks} Records</span>
          {/if}
        </div>
      </button>
    {/if}
  </div>
</div>

<div class="block">
  {#if loading}
    <p class="ellipsis">Loading data...</p>
  {:else if processing}
    <div class="block inner">
      <span>
        <Loader
          size={24}
          class="spinner ml-2"
          style="color: var(--accent-color)"
        />
        {removeMessage}
      </span>
      <div class="progress-container">
        <div class="progress-bar" style="width: {removeProgress}%"></div>
      </div>
    </div>
  {:else if $resultsStore.length > 0}
    <div class="virtual-table-container" bind:this={containerElement}>
      <div
        bind:this={parentRef}
        class="virtual-table-viewport"
        on:wheel={(e) => {
          // Don't stop propagation - can cause issues in some browsers
          // e.stopPropagation();

          // Explicitly handle the scroll with better cross-platform support
          if (parentRef) {
            // Different platforms have different scroll behavior
            // Delta multiplier to make scrolling feel consistent
            const multiplier = 1;

            // Use deltaMode to determine appropriate scaling
            // 0: pixels, 1: lines, 2: pages
            let scrollAmount = e.deltaY;

            // Apply platform-specific adjustments if needed
            if (e.deltaMode === 1) {
              // Line mode (typically Windows)
              scrollAmount *= 20; // Adjust for line-based scrolling
            }

            parentRef.scrollTop += scrollAmount * multiplier;
            // Prevent default to avoid double-scrolling
            e.preventDefault();
          }
        }}
      >
        <div class="virtual-table-header" style="width: {totalWidth};">
          <div
            class="grid-container rheader"
            style="grid-template-columns: {gridTemplateColumns};"
          >
            {#each columnConfigs as key, i}
              <div class="grid-item header {i === 0 ? 'sticky-column' : ''}">
                {key.header}
              </div>
            {/each}
          </div>

          <div
            class="resizer-container"
            style="grid-template-columns: {gridTemplateColumns}; display: grid; width: {totalWidth};"
          >
            {#each columnConfigs as column, i}
              <div class="resizer-cell">
                {#if i > 0 && i < 5}
                  <div
                    class="resizer"
                    on:mousedown={(event) => startResize(i, event)}
                  ></div>
                {:else}
                  <div></div>
                {/if}
              </div>
            {/each}
          </div>
        </div>

        <div
          class="virtual-table-body"
          style="height: {$rowVirtualizer.getTotalSize()}px; width: {totalWidth};"
        >
          {#each $rowVirtualizer.getVirtualItems() as virtualRow (virtualRow.index)}
            <div
              class="virtual-row"
              style="transform: translateY({virtualRow.start}px); height: {virtualRow.size}px; width: {totalWidth};"
            >
              <div
                class="list-item {filteredItems[
                  virtualRow.index
                ].algorithm.includes('Keep')
                  ? 'unselected-item'
                  : 'checked-item'}"
              >
                <div
                  class="grid-container"
                  style="{selectedItems.has(
                    filteredItems[virtualRow.index].id
                  ) && enableSelections
                    ? 'background-color: var(--accent-color)'
                    : ''};
                    grid-template-columns: {gridTemplateColumns};"
                >
                  {#each columnConfigs as column, i}
                    {#if column.name === "audio"}
                      <!-- Audio Column with sticky positioning if it's the first column -->
                      <div
                        class="grid-item {i === 0
                          ? 'sticky-column'
                          : i === 9
                            ? 'sticky-column-right'
                            : ''}
                              {selectedItems.has(
                          filteredItems[virtualRow.index].id
                        ) && enableSelections
                          ? 'selected'
                          : ''}"
                        on:click={() =>
                          playAudioFile(filteredItems[virtualRow.index])}
                      >
                        <Volume size={18} />
                      </div>
                    {:else if column.name === "checkbox"}
                      <!-- Checkbox Column with sticky positioning if it's the first column -->
                      <div
                        class="grid-item {i === 0
                          ? 'sticky-column'
                          : i === 9
                            ? 'sticky-column-right'
                            : ''}
                              {selectedItems.has(
                          filteredItems[virtualRow.index].id
                        ) && enableSelections
                          ? 'selected'
                          : ''}"
                        on:click={() =>
                          toggleChecked(filteredItems[virtualRow.index])}
                      >
                        {#if !filteredItems[virtualRow.index].algorithm.includes("Keep")}
                          <CheckSquare size={18} />
                        {:else}
                          <Square size={18} />
                        {/if}
                      </div>
                    {:else if column.name === "algorithm"}
                      <!-- Algorithm Column -->
                      <div
                        class="grid-item"
                        on:click={(event) =>
                          enableSelections
                            ? toggleSelect(
                                filteredItems[virtualRow.index],
                                event
                              )
                            : toggleChecked(filteredItems[virtualRow.index])}
                      >
                        <div class="algorithm-icons">
                          {#each filteredItems[virtualRow.index].algorithm.filter((algo: string) => algo !== "Keep" || filteredItems[virtualRow.index].algorithm.length === 1) as algo}
                            {@const iconData = getAlgorithmIcon(algo)}
                            <span class="icon-wrapper" title={iconData.tooltip}>
                              <svelte:component
                                this={iconData.component}
                                size={20}
                                style={iconData.color
                                  ? `color: ${iconData.color};`
                                  : ""}
                              />
                            </span>
                          {/each}
                        </div>
                      </div>
                    {:else}
                      <div
                        class="grid-item {column.name === 'filename'
                          ? 'bold'
                          : ''}"
                        on:click={(event) =>
                          enableSelections
                            ? toggleSelect(
                                filteredItems[virtualRow.index],
                                event
                              )
                            : toggleChecked(filteredItems[virtualRow.index])}
                      >
                        {filteredItems[virtualRow.index][
                          column.name as keyof FileRecord
                        ] || ""}
                      </div>
                    {/if}
                  {/each}
                </div>
              </div>
            </div>
          {/each}
        </div>
      </div>
    </div>
  {:else if $isSearching}
    <div class="block inner">
      <span>
        <Loader
          size={24}
          class="spinner ml-2"
          style="color: var(--accent-color)"
        />
        {$searchProgressStore.searchMessage}
      </span>
      <div class="progress-container">
        <div
          class="progress-bar"
          style="width: {$searchProgressStore.searchProgress}%"
        ></div>
      </div>
      <span>
        {$searchProgressStore.subsearchMessage}
      </span>
      <div class="progress-container">
        <div
          class="progress-bar"
          style="width: {$searchProgressStore.subsearchProgress}%"
        ></div>
      </div>
    </div>
  {:else}
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

          {#if algo.id === "dbcompare"}
            {#if algo.db !== null && algo.db !== undefined}
              {#await getFilenameWithoutExtension(algo.db) then filename}
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <span class="clickable" on:click={getCompareDb}>{filename}</span
                >
              {/await}
            {:else}
              <button
                type="button"
                class="small-button"
                style="border-color: var(--secondary-bg)"
                on:click={getCompareDb}>Select DB</button
              >
            {/if}
          {/if}

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
      {/each}
    </div>
  {/if}
</div>

<style>
  .virtual-table-container {
    position: relative;
    overflow: hidden;
  }

  .virtual-table-viewport {
    overflow: auto;
    height: 100%;
    width: 100%;
    will-change: transform;
    position: relative;
  }

  .virtual-table-header {
    width: max(var(--total-width), 100vw);
    position: sticky;
    top: 0;
    z-index: 10;
    background-color: var(--primary-bg);
    box-shadow: 0 1px 0 rgba(0, 0, 0, 0.1);
  }

  .virtual-table-body {
    position: relative;
    width: var(--total-width);
  }

  .virtual-row {
    position: absolute;
    top: 0;
    left: 0;
    width: var(--total-width);
  }

  .resizer-container {
    display: grid;
    height: 5px;
    position: relative;
    cursor: row-resize;
  }

  .resizer-cell {
    position: relative;
    overflow: visible;
    height: 100%;
  }

  .resizer {
    width: 4px;
    height: 40px;
    background-color: var(--inactive-color);
    position: absolute;
    right: -18px; /* Change from -20px to 0 */
    transform: translateX(50%); /* Center on the boundary */
    top: -40px;
    cursor: col-resize;
    z-index: 20;
    opacity: 0.7;
  }

  .resizer:hover {
    background-color: var(--hover-color);
    opacity: 1;
  }

  .grid-item {
    position: relative;
    padding: 3px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 14px;
    user-select: none;
    -webkit-user-select: none;
    -moz-user-select: none;
    -ms-user-select: none;
  }

  .grid-item.header {
    font-size: 16px;
    background-color: var(--secondary-bg);
  }

  .rheader {
    font-weight: bold;
    font-size: 16px;
    color: var(--accent-color);
    background-color: var(--secondary-bg);
    border-bottom: 1px solid var(--inactive-color);
    margin-left: 0px;
    margin-top: 0px;
    height: 45px;
    text-align: bottom;
    align-items: end;
    width: max(var(--total-width), 100vw);
  }

  .ellipsis {
    border-radius: 5px;
    animation: loading 1s infinite;
  }

  @keyframes loading {
    0% {
      opacity: 1;
    }
    50% {
      opacity: 0.5;
    }
    100% {
      opacity: 1;
    }
  }

  .header {
    display: flex;
    align-items: center;
    gap: 12px;
    background-color: var(--secondary-bg);
    margin-bottom: 10px;
  }

  .list-item {
    user-select: none;
    -webkit-user-select: none;
    -moz-user-select: none;
    -ms-user-select: none;
    width: max(var(--total-width), 100vw);
  }

  .algorithm-icons {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .icon-wrapper {
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .icon-wrapper:hover::after {
    content: attr(title);
    position: absolute;
    background: var(--primary-bg);
    border: 1px solid var(--border-color);
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 10px;
    z-index: 100;
    white-space: nowrap;
    top: 100%;
    left: 50%;
    transform: translateX(-50%);
  }

  /* Make BOTH header and cells sticky */
  .sticky-column {
    position: sticky !important;
    left: 0;
    z-index: 15;
    background-color: var(
      --primary-bg
    ); /* Background prevents content behind from showing through */
    box-shadow: 1px 0 3px rgba(0, 0, 0, 0.1); /* Optional shadow for depth */
  }
  .sticky-column-right {
    position: sticky !important;
    right: 0;
    z-index: 15;
    background-color: var(
      --primary-bg
    ); /* Background prevents content behind from showing through */
    box-shadow: 1px 0 3px rgba(0, 0, 0, 0.1); /* Optional shadow for depth */
    text-align: right;
  }

  /* Add these styles to preserve highlighting on sticky columns */
  .grid-item.sticky-column.selected,
  .grid-item.sticky-column-right.selected {
    background-color: var(--accent-color) !important;
  }

  /* For checked items (not just selected) */
  .checked-item .grid-item.sticky-column {
    background-color: var(--primary-bg);
  }
  .checked-item .grid-item.sticky-column-right {
    background-color: var(--primary-bg);

    /* If you need different styling for checked vs unchecked */
    /* background-color: var(--checked-bg-color); */
  }

  .select-field {
    flex-grow: 0;
  }

  .checked-item {
    color: var(--warning-hover);
    background-color: var(--primary-bg);
  }
  /* font-weight: bold; */
  .block {
    background-color: var(--secondary-bg);
    padding: 0px 0px;
    /* padding-bottom: 20px; */
    border-radius: 8px;
    flex: 1;
    /* Allow features to grow and shrink */
    margin-bottom: 0px;
    /* margin-top: -20px; */
    /* Adjust as needed */
    display: flex;
    flex-direction: column;
    /* Stack items vertically */
    height: calc(100vh - 55px);
    /* Full viewport height */
  }

  .block.inner {
    background-color: var(--primary-bg);
    padding: 0px;
    border-radius: 6px;
    /* display: flex; */
    /* flex-direction: column; */
    flex-grow: 1;
    /* height: 50vh; */
    /* height: calc(100vh - 240px); */
    overflow-y: auto;
  }

  .grid {
    grid-template-columns: repeat(3, 1fr);
    grid-template-rows: repeat(3, auto);
    grid-auto-flow: column;
  }
</style>
