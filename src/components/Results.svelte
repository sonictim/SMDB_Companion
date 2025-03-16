<script lang="ts">
  import {
    CheckSquare,
    Square,
    NotebookPenIcon,
    OctagonX,
    Volume2,
    Volume,
    TriangleAlert,
    Loader,
    Play,
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
  let selectedItems = new Set<number>();
  // let checkedItems = new Set<FileRecord>();
  let currentFilter = "Relevant";
  let idsToRemove: number[] = [];
  let filesToRemove: string[] = [];
  let filteredItems: FileRecord[] = [];
  let enableSelections = true;
  let lastPlayed = "Timbo";

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
  function updateUI() {
    // Force a UI update by creating a new array reference
    results = [...results];

    // Recalculate filtered items
    filteredItems = filterItems(results, currentFilter);
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

  function toggleChecksSelected() {
    filteredItems.forEach((item) => {
      if (selectedItems.has(item.id)) {
        if (item.algorithm.includes("Keep")) {
          removeCheck(item);
        } else {
          addCheck(item);
        }
      }
    });
    updateUI();
  }

  function uncheckSelected() {
    filteredItems.forEach((item) => {
      if (selectedItems.has(item.id)) {
        if (!item.algorithm.includes("Keep")) {
          addCheck(item);
        }
      }
    });
    updateUI();
  }

  // function addCheck(item: FileRecord) {
  //   if (!item.algorithm.includes("Keep")) {
  //     item.algorithm.push("Keep");
  //   }
  // }

  function checkSelected() {
    filteredItems.forEach((item) => {
      if (selectedItems.has(item.id)) removeCheck(item);
    });
    updateUI();
  }

  // function removeCheck(item: FileRecord) {
  //   if (item.algorithm.includes("Keep")) {
  //     // Create a new algorithm array
  //     const updatedAlgorithms = item.algorithm.filter(
  //       (algo) => algo !== "Keep",
  //     );
  //   }
  // }

  function addCheck(item: FileRecord) {
    if (!item.algorithm.includes("Keep")) {
      // Create a new item with updated algorithm array
      const updatedItem = {
        ...item,
        algorithm: [...item.algorithm, "Keep"],
      };

      // Update the results array to trigger reactivity
      results = results.map((r) => (r.id === item.id ? updatedItem : r));
    }
  }

  function removeCheck(item: FileRecord) {
    if (item.algorithm.includes("Keep")) {
      // Create a new item with updated algorithm array
      const updatedItem = {
        ...item,
        algorithm: item.algorithm.filter((algo) => algo !== "Keep"),
      };

      // Update the results array to trigger reactivity
      results = results.map((r) => (r.id === item.id ? updatedItem : r));
    }
  }

  // Add a variable to track the last selected item index
  let lastSelectedIndex = -1;

  function toggleSelect(item: FileRecord, event: MouseEvent) {
    event.preventDefault();
    // Get the current item index
    const currentIndex = filteredItems.findIndex(
      (record) => record.id === item.id,
    );

    // Handle Option/Alt click (toggle all)
    if (event.altKey) {
      if (selectedItems.size > 0) {
        // If any items selected, clear all
        selectedItems.clear();
      } else {
        // Otherwise select all filtered items
        filteredItems.forEach((record) => selectedItems.add(record.id));
      }
      selectedItems = new Set(selectedItems);
      return;
    }

    // Handle Shift click (range selection)
    if (event.shiftKey && lastSelectedIndex !== -1) {
      // Calculate the range (works in both directions)
      const start = Math.min(lastSelectedIndex, currentIndex);
      const end = Math.max(lastSelectedIndex, currentIndex);

      // Add all items in range to selection
      for (let i = start; i <= end; i++) {
        selectedItems.add(filteredItems[i].id);
      }
    } else {
      // Normal click (toggle individual)
      if (selectedItems.has(item.id)) {
        selectedItems.delete(item.id);
      } else {
        selectedItems.add(item.id);
        // Update last selected index for future shift-clicks
        lastSelectedIndex = currentIndex;
      }
    }

    // Update the set
    selectedItems = new Set(selectedItems);
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

    // Update items array
    results = results.map((i) => (i === item ? updatedItem : i));
  }

  function invertSelected() {
    filteredItems.forEach((item) => {
      if (selectedItems.has(item.id)) {
        // If selected, remove from selection
        selectedItems.delete(item.id);
      } else {
        // If not selected, add to selection
        selectedItems.add(item.id);
      }
    });
    selectedItems = new Set(selectedItems);
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

  function getTotalChecks() {
    return filteredItems.filter((item) => !item.algorithm.includes("Keep"))
      .length;
  }

  async function fetchData() {
    try {
      loading = true;
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

  async function confirmDialog() {
    let dbDialog = "Create Safety Copy";
    if (!pref.safety_db) dbDialog = "❌ Current Database";

    let filesDialog = "Keep in Place";
    if (pref.erase_files === "Trash") filesDialog = "⚠️ Move to Trash";
    else if (pref.erase_files === "Delete")
      filesDialog = "❌ Permanently Delete";

    let warningDialog = "";
    if (pref.erase_files === "Delete" || !pref.safety_db) {
      warningDialog = "\n\n⚠️ Are you sure? This is NOT undoable!";
    }

    let titleDialog = "Confirm Remove";
    if (pref.erase_files === "Delete" || !pref.safety_db) {
      titleDialog = "Confirm Remove";
    }

    let dialog = `Files on Disk: ${filesDialog}\nDatabase: ${dbDialog} ${warningDialog}`;

    const confirmed = await ask(dialog, {
      title: titleDialog,
      kind: "warning",
      okLabel: "Yes",
      cancelLabel: "Cancel",
    });

    return confirmed;
  }

  async function removeRecords() {
    if (!(await confirmDialog())) return;
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

  async function playAudioFile(record: FileRecord) {
    console.log("last played: ", lastPlayed);
    let filePath = record.path + "/" + record.root;
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

  function item(value: FileRecord, index: number, array: FileRecord[]): void {
    throw new Error("Function not implemented.");
  }
</script>

<div class="block">
  <div class="header">
    <h2>Search Results:</h2>
    <span style="font-size: 18px">
      {#if isRemove}
        {getTotalChecks()} of {results.length} Records marked for Removal
      {:else}
        {results.length} Records found
      {/if}
    </span>

    <div style="margin-left: auto; display: flex; gap: 20px;">
      {#if isRemove}
        <!-- {#if enableSelections}
          <button
            class="cta-button cancel"
            style="margin-right: 8px"
            on:click={removeSelected}
          >
            <OctagonX size="18" />
            Remove Selected
          </button>
        {/if} -->

        <button class="cta-button cancel" on:click={removeRecords}>
          <OctagonX size="18" />
          Remove Checked Records
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

  <!-- <div class="header" style="margin-bottom: 20px; margin-top: 10px;">
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
  </div> -->
  <div class="bar" style="margin-top: 10px; margin-bottom: 20px; padding: 0px;">
    <!-- <div class="button-group"> -->
    <!-- <button type="button" class="grid item" on:click={toggle_enable_selections}>
      {#if enableSelections}
        <CheckSquare size={20} class="checkbox checked" />
      {:else}
        <Square size={20} class="checkbox" />
      {/if}
      <span>Enable Selections</span>
    </button> -->

    <!-- <button class="small-button" on:click={() => checkSelected([...selectedItems])}>Check Selected</button>
      <button class="small-button" on:click={() => uncheckSelected([...selectedItems])}>Uncheck Selected</button> -->

    {#if enableSelections}
      <button class="small-button" on:click={toggleChecksSelected}
        >Toggle Selected</button
      >
      <button class="small-button" on:click={checkSelected}
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
          <div class="grid-item header" on:click={() => stopAudioFile()}>
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
              <!-- <Volume size={14} /> -->
              {#if !item.algorithm.includes("Keep")}
                <CheckSquare size={14} />
              {:else}
                <Square size={14} />
              {/if}
            </div>

            <!-- Root Column -->
            <div
              class="grid-item bold"
              on:click={(event) =>
                enableSelections
                  ? toggleSelect(item, event)
                  : toggleChecked(item)}
            >
              {item.root}
            </div>

            <div
              class="grid-item"
              on:click={(event) =>
                enableSelections
                  ? toggleSelect(item, event)
                  : toggleChecked(item)}
            >
              {item.path}
            </div>

            <!-- Algorithm Column -->
            <div class="grid-item" on:click={() => playAudioFile(item)}>
              {item.algorithm
                .filter(
                  (algo: string) =>
                    algo !== "Keep" || item.algorithm.length === 1,
                )
                .join(", ")}
            </div>
          </div>
        </div>
      </VirtualList>
    {/if}
  </div>
  <div class="header" style="margin-bottom: 0px; margin-top: 0px;">
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
    user-select: none; /* Standard syntax */
    -webkit-user-select: none; /* Safari */
    -moz-user-select: none; /* Firefox */
    -ms-user-select: none; /* IE10+/Edge */
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

  .list-item {
    user-select: none;
    -webkit-user-select: none;
    -moz-user-select: none;
    -ms-user-select: none;
  }
</style>
