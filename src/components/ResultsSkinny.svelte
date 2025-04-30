<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";

  import type { FileRecord } from "../stores/types";
  import { preferencesStore } from "../stores/preferences";
  import {
    resultsStore,
    filteredItemsStore,
    selectedItemsStore,
    currentFilterStore,
    enableSelectionsStore,
    toggleEnableSelections,
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
    manualFiltersStore, // Import the new manualFiltersStore
    filtersStore, // Import the derived filtersStore
  } from "../stores/results";
  import { metadataStore } from "../stores/metadata";
  import { ask, message } from "@tauri-apps/plugin-dialog";
  import { createVirtualizer } from "@tanstack/svelte-virtual";
  import { databaseStore, setDatabase } from "../stores/database";

  export let isRemove: boolean;
  export let activeTab: string; // This prop is now bindable
  export let selectedDb: string | null = null;

  $: pref = $preferencesStore;
  $: results = $resultsStore;
  $: metadata = $metadataStore;
  $: filteredItems = $filteredItemsStore;
  $: selectedItems = $selectedItemsStore;
  $: currentFilter = $currentFilterStore;
  $: enableSelections = $enableSelectionsStore;
  $: totalChecks = $totalChecksStore;
  $: selectedChecks = $selectedChecksStore; // Create a reactive reference to selected checks

  let processing = false;
  let loading = true;
  let idsToRemove: number[] = [];
  let filesToRemove: string[] = [];
  let dualMono: { id: number; path: string }[] = [];
  let lastPlayed = "Timbo";

  // Function to update UI after store changes
  function updateUI() {
    // Force reactivity by causing an update
    results = [...results];
  }

  // Handle filter change
  function handleFilterChange(event: Event) {
    const select = event.target as HTMLSelectElement;
    updateCurrentFilter(select.value);
  }

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

  function addCheck(item: FileRecord) {
    if (!item.algorithm.includes("Keep")) {
      const updatedItem = {
        ...item,
        algorithm: [...item.algorithm, "Keep"],
      };

      results = results.map((r) => (r.id === item.id ? updatedItem : r));
    }
  }

  function removeCheck(item: FileRecord) {
    if (item.algorithm.includes("Keep")) {
      const updatedItem = {
        ...item,
        algorithm: item.algorithm.filter((algo) => algo !== "Keep"),
      };

      results = results.map((r) => (r.id === item.id ? updatedItem : r));
    }
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
          activeTab = "search";
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
    loading = false;
    fetchData();

    if (containerElement) {
      containerWidth = containerElement.clientWidth;
    }
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
          // clearResults();
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

  function toggleStripMono(event: Event) {
    const select = event.target as HTMLSelectElement;
    preferencesStore.update((p) => {
      p.strip_dual_mono = select.value === "true";
      return p;
    });
  }
  function handleFileEraseChange(event: Event) {
    const select = event.target as HTMLSelectElement;
    preferencesStore.update((p) => {
      p.erase_files = select.value;
      return p;
    });
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
        activeTab = "search";
      }
    });
  });

  onDestroy(() => {
    if (unlistenRemoveFn) unlistenRemoveFn();
  });

  function item(value: FileRecord, index: number, array: FileRecord[]): void {
    throw new Error("Function not implemented.");
  }
  import {
    CheckSquare,
    Square,
    SquareEqual,
    NotebookPenIcon,
    OctagonX,
    Volume2,
    Volume,
    TriangleAlert,
    Loader,
    Play,
    Copy,
    FileX2,
    EqualApproximately,
    Tag,
    Tags,
    AudioWaveform,
    Clock,
    GitCompareArrowsIcon,
    Music,
    AudioLines,
    CheckCircle,
    Hash,
    ShieldCheck,
    Search,
    Activity,
    Asterisk,
    CopyCheck,
    CopyPlus,
    CopyMinus,
    CopySlash,
    ScanText,
    TextSearch,
    SearchCheck,
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

  function algoEnabled(algo: string): boolean {
    const algorithm = pref.algorithms.find((option) => option.id === algo);
    return algorithm?.enabled || false;
  }

  let processingBatch = false;

  async function checkSelectedWithIndicator() {
    processingBatch = true;
    // Use setTimeout to allow UI to update before heavy processing
    setTimeout(() => {
      try {
        checkSelected();
      } finally {
        processingBatch = false;
      }
    }, 10);
  }
</script>

<div class="block" style=" width: 75vw;">
  <div class="header">
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

    <div style="margin-left: auto; display: flex; gap: 20px;">
      {#if isRemove}
        {#if selectedItems.size > 0}
          <button class="cta-button cancel" on:click={removeSelectedRecords}>
            <OctagonX size="18" />
            Remove {selectedChecks} Selected Records
          </button>
          <button class="cta-button cancel" on:click={removeRecords}>
            <OctagonX size="18" />
            Remove all {totalChecks} Records
          </button>
        {:else if $databaseStore == null || $databaseStore.name == "" || $databaseStore.name == "Select Database"}
          <button class="cta-button inactive">
            <OctagonX size="18" />
            Remove {totalChecks} Records
          </button>
        {:else}
          <button class="cta-button cancel" on:click={removeRecords}>
            <OctagonX size="18" />
            Remove {totalChecks} Records
          </button>
        {/if}
      {:else}
        <button class="cta-button cancel" on:click={replaceMetadata}>
          <NotebookPenIcon size="18" />
          <span>Replace '{metadata.find}' with '{metadata?.replace || ""}'</span
          >
        </button>
      {/if}
    </div>
  </div>
  {#if $preferencesStore.showToolbars}
    <div
      class="bar"
      style="margin-top: 10px; margin-bottom: 20px; padding: 0px;"
    >
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
          <!-- <span>Filter by: </span>
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
          </select> -->
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
  {/if}
  <div
    class="block inner"
    bind:this={containerElement}
    style="margin-bottom: 15px;"
  >
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
    {:else}
      <div class="virtual-table-container" style="height: 80vh; width: 100%;">
        <div bind:this={parentRef} class="virtual-table-viewport">
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
                              <span
                                class="icon-wrapper"
                                title={iconData.tooltip}
                              >
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
    {/if}
  </div>
  {#if $preferencesStore.showToolbars}
    <div class="header" style="margin-bottom: 0px; margin-top: 0px;">
      {#if isRemove}
        <span>
          <!-- Remove Records from: -->
          <select
            class="select-field"
            bind:value={pref.safety_db}
            on:change={() => preferencesStore.set(pref)}
          >
            {#each [{ bool: true, text: "Remove From New Database" }, { bool: false, text: "Remove from Current Database" }] as option}
              <option value={option.bool}>{option.text}</option>
            {/each}
          </select>
          {#if pref.safety_db}
            with tag:
            <input
              class="input-field"
              style="width: 100px"
              placeholder="thinned"
              type="text"
              id="new_db_tag"
              bind:value={pref.safety_db_tag}
              on:change={() => preferencesStore.set(pref)}
            />
          {:else}
            <TriangleAlert
              size="30"
              class="blinking"
              style="color: var(--warning-hover); margin-bottom: -10px"
            />
          {/if}
        </span>
        {#if algoEnabled("dual_mono")}
          <span>
            <!-- Dual Mono Files: -->
            <select
              class="select-field"
              bind:value={pref.strip_dual_mono}
              on:change={() => preferencesStore.set(pref)}
            >
              {#each [{ id: false, text: "Preserve Dual Mono" }, { id: true, text: "Strip Dual Mono" }] as option}
                <option value={option.id}>{option.text}</option>
              {/each}
            </select>
            {#if pref.strip_dual_mono}
              <TriangleAlert
                size="30"
                class="blinking"
                style="color: var(--warning-hover); margin-bottom: -10px"
              />
            {/if}
          </span>
        {/if}
        <span>
          <!-- Checked Files: -->
          <select
            class="select-field"
            bind:value={$preferencesStore.erase_files}
          >
            {#each [{ id: "Keep", text: "Keep Files on Disk" }, { id: "Trash", text: "Move Files To Trash" }, { id: "Delete", text: "Permanently Delete Files" }] as option}
              <option value={option.id}>{option.text}</option>
            {/each}
          </select>
          {#if pref.erase_files !== "Keep"}
            <TriangleAlert
              size="30"
              class={pref.erase_files == "Delete" ? "blinking" : ""}
              style="color: var(--warning-hover); margin-bottom: -10px"
            />
          {/if}
        </span>
      {/if}
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
    position: sticky;
    top: 0;
    z-index: 10;
    background-color: var(--primary-bg);
    box-shadow: 0 1px 0 rgba(0, 0, 0, 0.1);
  }

  .virtual-table-body {
    position: relative;
  }

  .virtual-row {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
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
    height: 60px;
    background-color: var(--inactive-color);
    position: absolute;
    right: -18px; /* Change from -20px to 0 */
    transform: translateX(50%); /* Center on the boundary */
    top: -60px;
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
    background-color: var(--primary-bg);
  }

  .rheader {
    /* text-align: center;
    align-content: center;
    align-items: center;
    justify-content: center; */
    /* display: flex; */
    font-weight: bold;
    font-size: 16px;
    color: var(--accent-color);
    background-color: var(--primary-bg);
    border-bottom: 1px solid var(--inactive-color);
    margin-left: 0px;
    width: 100vw;
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

  .header h2 {
    margin: 0;
  }

  .list-item {
    user-select: none;
    -webkit-user-select: none;
    -moz-user-select: none;
    -ms-user-select: none;
    width: 100vw;
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
  .checked-item .grid-item.sticky-column,
  .checked-item .grid-item.sticky-column-right {
    /* If you need different styling for checked vs unchecked */
    /* background-color: var(--checked-bg-color); */
  }

  .select-field {
    flex-grow: 0;
  }
</style>
