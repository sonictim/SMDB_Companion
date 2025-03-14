<script lang="ts">
  import {
    CheckSquare,
    Square,
    NotebookPenIcon,
    OctagonX,
    Volume2,
    TriangleAlert,
    Loader,
  } from "lucide-svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import VirtualList from "svelte-virtual-list";
  import Select from "./prefs/Select.svelte";
  import { algorithmsStore, preferencesStore } from "../store";
  import { resultsStore, metadataStore } from "../session-store";
  import { get } from "svelte/store";
  import type { FileRecord } from "../store";
  import { ask } from "@tauri-apps/plugin-dialog";

  export let removeResults;
  export let isRemove: boolean;
  export let activeTab: string; // This prop is now bindable
  export let selectedDb: string | null = null;

  // type FileRecord = {
  //   root: string;
  //   path: string;
  //   algorithm: string[];
  //   id: number;
  // };

  $: pref = $preferencesStore;
  $: results = $resultsStore;
  $: metadata = $metadataStore;

  let processing = false;
  let loading = true;
  // let items: FileRecord[] = [];
  let total: number = 0;
  let selectedItems = new Set<number>();
  // let checkedItems = new Set<FileRecord>();
  let currentFilter = "Relevant";
  let idsToRemove: number[] = [];
  let filesToRemove: string[] = [];
  let filteredItems: FileRecord[] = [];
  let enableSelections = false;

  function toggle_enable_selections() {
    enableSelections = !enableSelections;
  }

  // Update filtered items whenever the filter or items change
  $: {
    let newFiltered = filterItems(results, currentFilter);
    newFiltered.forEach((item) => {
      if (selectedItems.has(item.id)) {
        selectedItems.add(item.id); // Ensure selected state persists
      }
    });
    filteredItems = newFiltered;
  }

  // Filter function
  function filterItems(items: FileRecord[], filter: string): FileRecord[] {
    switch (filter) {
      case "All":
        return items;
      case "Relevant":
        return items.filter(
          (item) =>
            !item.algorithm.includes("Keep") || item.algorithm.length > 1,
        );
      case "Keep":
        return items.filter((item) => item.algorithm.includes("Keep"));
      case "Remove":
        return items.filter((item) => !item.algorithm.includes("Keep"));
      default:
        // Match specific algorithm name
        return items.filter((item) => item.algorithm.includes(filter));
    }
  }

  // Handle filter change
  function handleFilterChange(event: Event) {
    const select = event.target as HTMLSelectElement;
    currentFilter = select.value;
  }

  // Store both percentage and pixel values
  type ColumnConfig = {
    minWidth: number;
    width: number; // Current width in pixels
    percentage: number; // Width as a percentage of total width
  };

  // Define initial column configurations with percentages
  let columnConfigs: ColumnConfig[] = [
    { minWidth: 10, width: 20, percentage: 2 }, // Checkbox
    { minWidth: 100, width: 200, percentage: 30 }, // Root
    { minWidth: 150, width: 300, percentage: 53 }, // Path
    { minWidth: 50, width: 50, percentage: 15 }, // Algorithm
  ];

  // Computed property to get current column widths
  $: columnWidths = columnConfigs.map((config) => config.width);

  // Container element reference for getting available width
  let containerElement: HTMLElement;
  let containerWidth = 0;

  // Listen for resize events
  function handleResize() {
    if (containerElement) {
      const newContainerWidth = containerElement.clientWidth;

      // Only update if container width has changed
      if (newContainerWidth !== containerWidth) {
        containerWidth = newContainerWidth;
        updateColumnWidthsFromContainer();
      }
    }
  }

  // Update column widths based on container size
  function updateColumnWidthsFromContainer() {
    // Skip if container isn't available yet
    if (!containerWidth) return;

    // Calculate total available width (minus some buffer for padding/margins)
    const availableWidth = containerWidth - 20;

    // Update column widths based on percentages
    columnConfigs = columnConfigs.map((config) => {
      const calculatedWidth = Math.max(
        config.minWidth,
        Math.floor(availableWidth * (config.percentage / 100)),
      );
      return { ...config, width: calculatedWidth };
    });
  }

  function startResize(index: number, event: MouseEvent) {
    event.preventDefault();

    const startX = event.clientX;
    const startWidth = columnConfigs[index].width;
    const totalWidthBefore = columnConfigs.reduce(
      (sum, col) => sum + col.width,
      0,
    );

    function onMouseMove(e: MouseEvent) {
      const diff = e.clientX - startX;
      const newWidth = Math.max(
        columnConfigs[index].minWidth,
        startWidth + diff,
      );

      // Update width in pixels
      columnConfigs[index].width = newWidth;

      // Recalculate percentages for all columns
      const totalWidthAfter = columnConfigs.reduce(
        (sum, col) => sum + col.width,
        0,
      );
      columnConfigs = columnConfigs.map((config) => {
        return {
          ...config,
          percentage: (config.width / totalWidthAfter) * 100,
        };
      });
    }

    function onMouseUp() {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
    }

    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
  }

  // function checkSelected(list: FileRecord[]) {
  //   list.forEach(item => addCheck(item));
  // }

  // function uncheckSelected(list: FileRecord[]) {
  //   list.forEach(item => removeCheck(item));
  // }

  // function addCheck(item: FileRecord) {
  //   if (!checkedItems.has(item)) checkedItems.add(item);
  //   checkedItems = new Set(checkedItems);
  // }

  // function removeCheck(item: FileRecord) {
  //   if (checkedItems.has(item)) checkedItems.delete(item);
  //   checkedItems = new Set(checkedItems);
  // }
  // function addSelect(item: FileRecord) {
  //   if (!selectedItems.has(item)) selectedItems.add(item);
  //   selectedItems = new Set(selectedItems);
  // }

  // function removeSelect(item: FileRecord) {
  //   if (selectedItems.has(item)) selectedItems.delete(item);
  //   selectedItems = new Set(selectedItems);
  // }

  function toggleSelect(item: FileRecord) {
    if (selectedItems.has(item.id)) {
      selectedItems.delete(item.id);
    } else {
      selectedItems.add(item.id);
    }
    selectedItems = new Set(selectedItems);
    console.log("toggled: ", selectedItems);
  }

  function toggleChecked(item: FileRecord) {
    const isKeeping = item.algorithm.includes("Keep");

    // Clone the item and update its algorithm array
    const updatedAlgorithms = isKeeping
      ? item.algorithm.filter((algo) => algo !== "Keep") // Remove "Keep"
      : [...item.algorithm, "Keep"]; // Add "Keep"

    const updatedItem = {
      ...item,
      algorithm: updatedAlgorithms,
    };

    // Adjust total count
    total += isKeeping ? -1 : 1;

    // Update items array
    results = results.map((i) => (i === item ? updatedItem : i));
  }

  function clearSelected() {
    selectedItems.clear();
    selectedItems = new Set(selectedItems);
  }

  async function replaceMetadata() {
    const confirmed = await ask("Are you sure? This is not undoable", {
      title: "Confirm Replace",
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
      total = results.filter((item) => !item.algorithm.includes("Keep")).length; // Fetch total count first
      // items = await invoke<FileRecord[]>("get_results"); // Then fetch the items
    } catch (error) {
      console.error("Failed to fetch data:", error);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loading = false;
    fetchData();

    // Initial size calculation
    if (containerElement) {
      containerWidth = containerElement.clientWidth;
      updateColumnWidthsFromContainer();
    }

    // Add window resize listener
    window.addEventListener("resize", handleResize);

    // Clean up listener on component destruction
    return () => {
      window.removeEventListener("resize", handleResize);
    };
  });

  let filters = [
    { id: "All", name: "All Records" },
    { id: "Relevant", name: "Relevant Records" },
    { id: "Keep", name: "Records to Keep" },
    { id: "Remove", name: "Records to Remove" },
    { id: "Basic", name: "Basic Duplicate Search" },
    { id: "InvalidPath", name: "Valid Filename" },
    { id: "SimilarFilename", name: "Similar Filename Search" },
    { id: "Tags", name: "Audiosuite Tags" },
    { id: "Waveform", name: "Waveform Comparison" },
    { id: "Duration", name: "Duration" },
    { id: "Compare", name: "Database Compare" },
  ];

  async function removeRecords() {
    if (pref.erase_files === "Delete" || !pref.safety_db) {
      const confirmed = await ask("Are you sure? This is not undoable", {
        title: "Confirm Remove",
        kind: "warning",
        okLabel: "Yes",
        cancelLabel: "Cancel",
      });

      if (!confirmed) return;
    }

    // Filter out items that have "Keep" in their algorithm
    idsToRemove = filteredItems
      .filter((item) => !item.algorithm.includes("Keep")) // Only keep items without "Keep"
      .map((item) => item.id); // Extract the ids
    filesToRemove = filteredItems
      .filter((item) => !item.algorithm.includes("Keep")) // Only keep items without "Keep"
      .map((item) => item.path + "/" + item.root); // Extract the ids

    if (idsToRemove.length > 0) {
      processing = true;
      // Call the backend function to remove the records by passing the ids
      await invoke<string>("remove_records", {
        records: idsToRemove,
        clone: pref.safety_db,
        cloneTag: pref.safety_db_tag,
        delete: pref.erase_files,
        files: filesToRemove,
      })
        .then((updatedDb) => {
          console.log("Successfully removed records with IDs:", idsToRemove);
          selectedDb = updatedDb;
          // Optionally, update the filteredItems to remove the items locally
        })
        .catch((error) => {
          console.error("Error removing records:", error);
          processing = false;
        });
    }
  }
  async function removeSelected() {
    if (selectedItems.size > 0) {
      processing = true;
      idsToRemove = Array.from(selectedItems.values());

      filesToRemove = filteredItems
        .filter(
          (item) =>
            !selectedItems.has(item.id) || !item.algorithm.includes("Keep"),
        ) // Only keep items without "Keep"
        .map((item) => item.path + "/" + item.root);

      await invoke<string>("remove_records", {
        records: idsToRemove,
        clone: pref.safety_db,
        cloneTag: pref.safety_db_tag,
        delete: pref.erase_files,
        files: filesToRemove,
      })
        .then((newDbName) => {
          console.log("Successfully removed records with IDs:", selectedItems);
          selectedDb = newDbName;
        })
        .catch((error) => {
          console.error("Error removing records:", error);
          activeTab = "search";
        });
    }
  }

  async function previewFile(record: FileRecord) {
    let filePath = record.path + "/" + record.root;
    console.log("playing audio:", filePath);
    await invoke("play_audio", { path: filePath })
      .then(() => {
        console.log("Success:", filePath);
      })
      .catch((error) => {
        console.error("Error calling audio playback:", error);
      });
  }
  // async function previewFile(record: FileRecord) {
  //   let filePath = record.path + "/" + record.root;
  //   await invoke("open_quicklook", { filePath: filePath })
  //     .then(() => {
  //       console.log("QuickLook:", filePath);
  //     })
  //     .catch((error) => {
  //       console.error("Error calling quicklook:", error);
  //     });
  // }

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

  // Add these variables for search status
  let removeProgress = 0;
  let removeMessage = "Initializing...";
  let removeStage = "";
  let unlistenRemoveFn: () => void;

  // Setup event listener when component mounts
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
        `Remove status: ${status.stage} - ${status.progress}% - ${status.message}`,
      );
      if (status.stage === "complete") {
        //         filteredItems = filteredItems.filter(
        //   (item) => !idsToRemove.includes(item.id),
        // );
        // if (filteredItems.length == 0) activeTab = "search";
        // selectedDb = await invoke<string>("get_db_name");
        processing = false;
        fetchData();
        activeTab = "search";
      }
    });
  });

  // Cleanup event listener when component unmounts
  onDestroy(() => {
    if (unlistenRemoveFn) unlistenRemoveFn();
  });
</script>

<div class="block">
  <div class="header">
    <h2>Search Results:</h2>
    <span style="font-size: 18px">
      {#if isRemove}
        {total} of {results.length} Records marked for Removal
      {:else}
        {results.length} Records found
      {/if}
    </span>

    <div style="margin-left: auto; display: flex; gap: 20px;">
      {#if isRemove}
        {#if enableSelections}
          <button
            class="cta-button cancel"
            style="margin-right: 8px"
            on:click={removeSelected}
          >
            <OctagonX size="18" />
            Remove Selected
          </button>
        {/if}

        <button class="cta-button cancel" on:click={removeRecords}>
          <OctagonX size="18" />
          Remove Checked
        </button>
      {:else}
        <button class="cta-button cancel" on:click={replaceMetadata}>
          <NotebookPenIcon size="18" />
          <span>Replace '{metadata.find}' with '{metadata?.replace || ""}'</span
          >
        </button>
      {/if}
    </div>
  </div>

  <div class="bar" style="margin-bottom: 16px; margin-top: 10px">
    <!-- <div class="button-group"> -->
    <button type="button" class="grid item" on:click={toggle_enable_selections}>
      {#if enableSelections}
        <CheckSquare size={20} class="checkbox checked" />
      {:else}
        <Square size={20} class="checkbox" />
      {/if}
      <span>Enable Selections</span>
    </button>

    <!-- <button class="small-button" on:click={() => checkSelected([...selectedItems])}>Check Selected</button>
      <button class="small-button" on:click={() => uncheckSelected([...selectedItems])}>Uncheck Selected</button> -->

    {#if enableSelections}
      <button class="small-button" on:click={clearSelected}
        >Clear all Selections</button
      >
    {/if}
    <!-- </div> -->

    <div class="filter-container">
      {#if isRemove}
        <span>Filter by: </span>
        <select
          class="select-field"
          bind:value={currentFilter}
          on:change={handleFilterChange}
        >
          {#each filters as option}
            <option value={option.id}>{option.name}</option>
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

  <div
    class="block inner"
    bind:this={containerElement}
    on:resize={handleResize}
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
      <div class="grid-header">
        <div
          class="grid-container rheader"
          style="grid-template-columns: {columnWidths[0]}px {columnWidths[1]}px {columnWidths[2]}px {columnWidths[3]}px;"
        >
          <!-- Checkbox Header -->
          <div class="grid-item header">✔</div>

          <!-- Root Header -->
          <div class="grid-item header bold">Filename</div>

          <!-- Path Header -->
          <div class="grid-item header">Path</div>

          <!-- Algorithm Header -->
          <div class="grid-item header">
            <span>
              Algorithm
              <Volume2 size={20} />
            </span>
          </div>
        </div>

        <!-- Resizers -->
        <div
          class="resizer-container"
          style="grid-template-columns: {columnWidths[0]}px {columnWidths[1]}px {columnWidths[2]}px {columnWidths[3]}px; "
        >
          <div class="resizer-cell">
            <div
              class="resizer"
              on:mousedown={(event) => startResize(0, event)}
            ></div>
          </div>
          <div class="resizer-cell">
            <div
              class="resizer"
              on:mousedown={(event) => startResize(1, event)}
            ></div>
          </div>
          <div class="resizer-cell">
            <div
              class="resizer"
              on:mousedown={(event) => startResize(2, event)}
            ></div>
          </div>
          <div class="resizer-cell">
            <!-- Last column doesn't need a resizer -->
          </div>
        </div>
      </div>

      <VirtualList items={filteredItems} let:item>
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <!-- on:click={() =>  enableSelections ? toggleSelect(item) : toggleChecked(item)} -->
        <div
          class="list-item {item.algorithm.includes('Keep')
            ? 'unselected-item'
            : 'checked-item'}"
        >
          <div
            class="grid-container"
            style="{selectedItems.has(item.id) && enableSelections
              ? 'background-color: var(--accent-color)'
              : ''};    grid-template-columns: {columnWidths[0]}px {columnWidths[1]}px {columnWidths[2]}px {columnWidths[3]}px;"
          >
            <!-- Checkbox Column -->
            <div class="grid-item" on:click={() => toggleChecked(item)}>
              {#if !item.algorithm.includes("Keep")}
                <CheckSquare size={14} />
              {:else}
                <Square size={14} />
              {/if}
            </div>

            <!-- Root Column -->
            <div
              class="grid-item bold"
              on:click={() =>
                enableSelections ? toggleSelect(item) : toggleChecked(item)}
            >
              {item.root}
            </div>

            <!-- Path Column -->
            <div
              class="grid-item"
              on:click={() =>
                enableSelections ? toggleSelect(item) : toggleChecked(item)}
            >
              {item.path}
            </div>

            <!-- Algorithm Column -->
            <div class="grid-item" on:click={() => previewFile(item)}>
              {item.algorithm
                .filter((algo: string) => algo !== "Keep")
                .join(", ")}
            </div>
          </div>
        </div>
      </VirtualList>
    {/if}
  </div>
  <div class="header" style="margin-bottom: 0px">
    <span>
      Remove Records From:
      <select
        class="select-field"
        bind:value={pref.safety_db}
        on:change={() => preferencesStore.set(pref)}
      >
        {#each [{ bool: true, text: "Database Copy" }, { bool: false, text: "Current Database" }] as option}
          <option value={option.bool}>{option.text}</option>
        {/each}
      </select>
      {#if pref.safety_db}
        with tag:
        <input
          class="input-field"
          placeholder="thinned"
          type="text"
          id="new_db_tag"
          bind:value={pref.safety_db_tag}
          on:change={() => preferencesStore.set(pref)}
        />
      {:else}
        <TriangleAlert
          size="20"
          class="blinking"
          style="color: var(--warning-hover)"
        />
      {/if}
    </span>
    <span>
      {#if pref.erase_files !== "Keep"}
        <TriangleAlert
          size="20"
          class={pref.erase_files == "Delete" ? "blinking" : ""}
          style="color: var(--warning-hover)"
        />
      {/if}
      Duplicate Files On Disk:
      <select class="select-field" on:change={handleFileEraseChange}>
        {#each [{ id: "Keep", text: "Keep" }, { id: "Trash", text: "Move To Trash" }, { id: "Delete", text: "Permanently Delete" }] as option}
          <option value={option.id}>{option.text}</option>
        {/each}
      </select>
    </span>
  </div>
</div>

<style>
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
    right: -6px; /* Changed from 0 to -4px to move right */
    top: -60px;
    cursor: col-resize;
    z-index: 20;
    opacity: 0.7;
  }

  .resizer:hover {
    background-color: var(--hover-color);
    opacity: 1;
  }

  /* Make sure the position is set correctly for grid items */
  .grid-item {
    position: relative;
    padding: 3px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
  }

  .grid-item.header {
    font-size: 16px;
    background-color: var(--primary-bg);
  }

  /* Update header styles */
  .rheader {
    font-weight: bold;
    font-size: 16px;
    color: var(--accent-color);
    background-color: var(--primary-bg);
    border-bottom: 1px solid var(--inactive-color);
  }

  .ellipsis {
    /* display: inline-block;
    width: 80px;
    height: 10px; */
    /* background-color: #ccc; */
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
    gap: 12px; /* Adjust spacing */
    background-color: var(--secondary-bg);
    margin-bottom: 10px;
  }

  .header h2 {
    margin: 0; /* Removes extra spacing */
  }
</style>
