<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";

  import type { FileRecord } from "../../stores/types";
  import {
    filteredItemsStore,
    selectedItemsStore,
    enableSelectionsStore,
    toggleSelect,
    toggleChecked,
    lastSelectedIndexStore,
  } from "../../stores/results";
  import { createVirtualizer } from "@tanstack/svelte-virtual";

  $: filteredItems = $filteredItemsStore;
  $: selectedItems = $selectedItemsStore;
  $: enableSelections = $enableSelectionsStore;

  let processing = false;
  let loading = true;
  let lastPlayed = "Timbo";

  // Drag selection variables
  let isDragging = false;
  let dragStartIndex = -1;
  let lastDragIndex = -1;
  let dragSelectionState: Map<number, boolean> = new Map();
  let previouslySelectedItems: Set<number> = new Set();

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

  // Drag selection functions
  function handleMouseDown(rowIndex: number, event: MouseEvent) {
    if (!enableSelections || event.button !== 0) return; // Only handle left mouse button

    isDragging = true;
    dragStartIndex = rowIndex;
    lastDragIndex = rowIndex;
    previouslySelectedItems = new Set(selectedItems);

    // Store the current selection state of the clicked item to determine
    // if we should select or deselect during drag
    const itemId = filteredItems[rowIndex].id;

    // Update the selection state of the initial item
    selectedItemsStore.update((currentSelected) => {
      const newSelected = new Set(currentSelected);
      if (newSelected.has(itemId)) {
        newSelected.delete(itemId);
      } else {
        newSelected.add(itemId);
      }
      lastSelectedIndexStore.set(rowIndex);
      return newSelected;
    });

    // Set up global mouse move and mouse up handlers
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
  }

  function handleMouseMove(event: MouseEvent) {
    if (!isDragging || !enableSelections) return;

    // Find the row element under the mouse
    const elementUnderMouse = document.elementFromPoint(
      event.clientX,
      event.clientY
    );
    if (!elementUnderMouse) return;

    // Find the virtual row containing this element
    const virtualRow = elementUnderMouse.closest(".virtual-row");
    if (!virtualRow) return;

    // Get the index from the dataset or other attribute
    const rowIndexAttr = virtualRow.getAttribute("data-index");
    if (!rowIndexAttr) return;

    const currentIndex = parseInt(rowIndexAttr, 10);
    if (
      isNaN(currentIndex) ||
      currentIndex < 0 ||
      currentIndex >= filteredItems.length
    )
      return;

    // If we've moved to a different row
    if (currentIndex !== lastDragIndex) {
      // Determine range between the current position and the last processed position
      const start = Math.min(dragStartIndex, currentIndex);
      const end = Math.max(dragStartIndex, currentIndex);
      const lastStart = Math.min(dragStartIndex, lastDragIndex);
      const lastEnd = Math.max(dragStartIndex, lastDragIndex);

      // Update selection based on direction
      selectedItemsStore.update((currentSelected) => {
        const newSelected = new Set(currentSelected);

        // Reset any selections that are no longer in the range
        for (let i = 0; i < filteredItems.length; i++) {
          // If it's in the old range but not in the new range
          if (i >= lastStart && i <= lastEnd && !(i >= start && i <= end)) {
            const itemId = filteredItems[i].id;
            // Restore to its original state
            if (previouslySelectedItems.has(itemId)) {
              newSelected.add(itemId);
            } else {
              newSelected.delete(itemId);
            }
          }
        }

        // For items in the new range that are not the start item
        for (let i = start; i <= end; i++) {
          // Skip the drag start item, which was already toggled on mousedown
          if (i === dragStartIndex) continue;

          const itemId = filteredItems[i].id;

          // If moving away from start point, add to selection
          if (
            (dragStartIndex < currentIndex && i > dragStartIndex) ||
            (dragStartIndex > currentIndex && i < dragStartIndex)
          ) {
            newSelected.add(itemId);
          }
          // If moving back toward start point, restore original selection state
          else if (
            (dragStartIndex < currentIndex && i < dragStartIndex) ||
            (dragStartIndex > currentIndex && i > dragStartIndex)
          ) {
            if (previouslySelectedItems.has(itemId)) {
              newSelected.add(itemId);
            } else {
              newSelected.delete(itemId);
            }
          }
        }

        return newSelected;
      });

      lastDragIndex = currentIndex;
    }
  }

  function handleMouseUp() {
    if (isDragging) {
      isDragging = false;
      dragSelectionState.clear();

      // Remove global event listeners
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);

      // Update the virtualizer to reflect any changes in the selection
      updateVirtualizer();
    }
  }

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
    fetchData();
  });

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

    // Clean up any event listeners
    window.removeEventListener("mousemove", handleMouseMove);
    window.removeEventListener("mouseup", handleMouseUp);
  });

  // Helper function to safely access record properties
  function getRecordValue(record: FileRecord, key: string): string {
    return (record[key as keyof FileRecord] as string) || "";
  }
  import {
    CheckSquare,
    Square,
    SquareEqual,
    OctagonX,
    Volume,
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
</script>

<div class="virtual-table-container" bind:this={containerElement}>
  <div
    bind:this={parentRef}
    class="virtual-table-viewport"
    on:wheel={(e) => {
      // Let the browser handle the native scrolling behavior
      // This approach is more compatible across different platforms
      // No need to preventDefault() or manually adjust scrollTop

      // Windows users often experience issues with custom wheel handlers
      // The default browser behavior works better in most cases

      // If scrolling issues persist, we'll update rowVirtualizer as a fallback
      if (e.deltaY !== 0 && $rowVirtualizer) {
        // Schedule a microtask to ensure smooth virtual list updates
        queueMicrotask(() => {
          if ($rowVirtualizer) $rowVirtualizer.measure();
        });
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
          data-index={virtualRow.index}
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
              style="{selectedItems.has(filteredItems[virtualRow.index].id) &&
              enableSelections
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
                  <div
                    class="grid-item"
                    on:click={(event) =>
                      enableSelections
                        ? toggleSelect(filteredItems[virtualRow.index], event)
                        : toggleChecked(filteredItems[virtualRow.index])}
                    on:mousedown={(event) =>
                      enableSelections
                        ? handleMouseDown(virtualRow.index, event)
                        : null}
                  >
                    <div class="algorithm-icons">
                      {#each filteredItems[virtualRow.index].algorithm.filter((algo) => algo !== "Keep" || filteredItems[virtualRow.index].algorithm.length === 1) as algo}
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
                    class="grid-item {column.name === 'filename' ? 'bold' : ''}"
                    on:click={(event) =>
                      enableSelections
                        ? toggleSelect(filteredItems[virtualRow.index], event)
                        : toggleChecked(filteredItems[virtualRow.index])}
                    on:mousedown={(event) =>
                      enableSelections
                        ? handleMouseDown(virtualRow.index, event)
                        : null}
                  >
                    {getRecordValue(
                      filteredItems[virtualRow.index],
                      column.name
                    )}
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

<style>
  .virtual-table-container {
    position: relative;
    overflow: hidden;
    /* width: 100vw; */
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
    border-top: 1px solid var(--inactive-color);
    margin-top: 0px;
  }
  .virtual-table-header2 {
    width: max(var(--total-width), 100vw);
    position: sticky;
    top: 0;
    z-index: 10;
    background-color: var(--primary-bg);
    box-shadow: 0 1px 0 rgba(0, 0, 0, 0.1);
    border-top: 1px solid var(--inactive-color);
  }

  .virtual-table-body {
    position: relative;
    width: max(var(--total-width), 100vw);
  }

  .virtual-row {
    position: absolute;
    top: 0;
    left: 0;
    width: max(var(--total-width), 100vw);
    user-select: none;
    cursor: pointer;
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
    height: 45px;
    background-color: var(--inactive-color);
    position: absolute;
    right: -18px; /* Change from -20px to 0 */
    transform: translateX(50%); /* Center on the boundary */
    top: -45px;
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
    width: max(var(--total-width), 100vw);
  }

  .grid-item.header {
    font-size: 16px;
    background-color: var(--secondary-bg);
    margin-top: 0px;
    width: max(var(--total-width), 100vw);
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

  .checked-item {
    color: var(--warning-hover);
    background-color: var(--primary-bg);
  }

  /* Style for when drag selection is active */
  .virtual-row:active .grid-item {
    cursor: grabbing;
  }
</style>
