<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";

  import type { FileRecord } from "../../stores/types";
  import {
    filteredItemsStore,
    filteredGroupsStore,
    selectedItemsStore,
    enableSelectionsStore,
    toggleSelect,
    toggleChecked,
    lastSelectedIndexStore,
    columnConfigStore,
    columnWidthsStore,
    totalWidthStore,
    gridTemplateColumnsStore,
    sortColumnStore,
    sortDirectionStore,
    handleHeaderClick,
  } from "../../stores/results";
  import { getHotkey } from "../../stores/hotkeys";
  import { createVirtualizer } from "@tanstack/svelte-virtual";

  $: filteredItems = $filteredItemsStore;
  $: filteredGroups = $filteredGroupsStore;
  $: selectedItems = $selectedItemsStore;
  $: enableSelections = $enableSelectionsStore;

  // Use column configuration from store
  $: columnConfigs = $columnConfigStore;
  $: columnWidths = $columnWidthsStore;
  $: totalWidth = $totalWidthStore;
  $: gridTemplateColumns = $gridTemplateColumnsStore;

  // Sort state
  $: sortColumn = $sortColumnStore;
  $: sortDirection = $sortDirectionStore;

  // Function to determine if a row is the start of a group
  // This is used to add visual separation between groups
  function isGroupStart(index: number): boolean {
    if (index === 0) return true; // First item gets the class but CSS will hide its border

    // Get the current item from the flat filtered list
    const currentItem = filteredItems[index];

    // Check if this item is the first item in any group
    for (const group of filteredGroups) {
      if (group.length > 0 && group[0].id === currentItem.id) {
        return true;
      }
    }

    return false;
  }

  // Function to determine if a row is the end of a group
  // This is used to add bottom padding to the last item in each group
  function isGroupEnd(index: number): boolean {
    // Last item in the entire list is always a group end
    if (index === filteredItems.length - 1) return true;

    // Get the current item from the flat filtered list
    const currentItem = filteredItems[index];

    // Check if this item is the last item in any group
    for (const group of filteredGroups) {
      if (group.length > 0 && group[group.length - 1].id === currentItem.id) {
        return true;
      }
    }

    return false;
  }

  // Function to determine if a row is part of a single-item group
  // This is used for groups that have only one item (both start and end)
  function isGroupSizeOne(index: number): boolean {
    // Get the current item from the flat filtered list
    const currentItem = filteredItems[index];

    // Check if this item is in a group of size 1
    for (const group of filteredGroups) {
      if (group.length === 1 && group[0].id === currentItem.id) {
        return true;
      }
    }

    return false;
  }

  let processing = false;
  let loading = true;
  let lastPlayed = "Timbo";

  // Drag selection variables
  let isDragging = false;
  let dragStartIndex = -1;
  let lastDragIndex = -1;
  let dragSelectionState: Map<number, boolean> = new Map();
  let previouslySelectedItems: Set<number> = new Set();
  let isDeselectMode = false; // Flag to track if we're in deselect mode (Option/Alt key)

  // Functions for working with custom mouse modifiers
  function getMouseModifierKeys(actionName: string): string {
    const hotkeyValue = getHotkey(actionName);
    if (!hotkeyValue) return "";

    // If there's no plus sign, there's no modifier
    if (!hotkeyValue.includes("+")) return "";

    // Extract everything before the final plus (which separates action)
    const parts = hotkeyValue.split("+");
    if (parts.length <= 1) return "";

    // Remove the action part (last element) and join the rest
    return parts.slice(0, -1).join("+");
  }

  function checkMouseModifier(event: MouseEvent, actionName: string): boolean {
    const hotkeyValue = getHotkey(actionName);
    if (!hotkeyValue) {
      console.log(`${actionName}: No hotkey found`);
      return false;
    }

    // Check if we should skip the base action type check
    // 1. For lasso actions in handleMouseDown (before dragging starts)
    // 2. When we're evaluating a potential drag operation
    const isPotentialDragAction = actionName.includes("lasso");
    const skipBaseActionCheck = !isDragging && isPotentialDragAction;

    // Debug information about the action being checked
    console.log(
      `Checking ${actionName}: isDragging=${isDragging}, skipBaseActionCheck=${skipBaseActionCheck}`
    );

    // Only check the base action type (Click/Drag) if we're not skipping
    if (!skipBaseActionCheck) {
      const isClick = !hotkeyValue.includes("Drag");
      const baseActionMatches = isDragging ? !isClick : isClick;
      if (!baseActionMatches) {
        console.log(
          `${actionName}: Base action mismatch, isDragging=${isDragging}, hotkeyValue=${hotkeyValue}`
        );
        return false;
      }
    }

    // Check if modifiers match
    const hasAlt = hotkeyValue.includes("Alt+");
    const hasShift = hotkeyValue.includes("Shift+");
    const hasCmd = hotkeyValue.includes("CmdOrCtrl+");

    const modifiersMatch =
      event.altKey === hasAlt &&
      event.shiftKey === hasShift &&
      (event.metaKey || event.ctrlKey) === hasCmd;

    if (!modifiersMatch) {
      console.log(
        `${actionName}: Modifiers don't match, expected Alt=${hasAlt}, Shift=${hasShift}, Cmd/Ctrl=${hasCmd}, got Alt=${event.altKey}, Shift=${event.shiftKey}, Cmd/Ctrl=${event.metaKey || event.ctrlKey}`
      );
    }

    return modifiersMatch;
  }

  let containerElement: HTMLElement;

  // Track mousedown for possible drag operation
  let mouseDownTime = 0;
  let mouseDownPosition = { x: 0, y: 0 };

  // Drag selection functions
  function handleMouseDown(rowIndex: number, event: MouseEvent) {
    if (!enableSelections || event.button !== 0) return; // Only handle left mouse button

    // Update UI feedback for hotkey modifiers
    updateHotkeyFeedback(event);

    // Store the time and position when mouse is pressed down
    mouseDownTime = Date.now();
    mouseDownPosition = { x: event.clientX, y: event.clientY };

    dragStartIndex = rowIndex;
    lastDragIndex = rowIndex;

    // Store current selection state before any changes
    previouslySelectedItems = new Set(selectedItems);

    // Get hotkeys for lasso operations - we need to evaluate this correctly
    // before the drag operation actually starts
    const lassoUnselectHotkey = getHotkey("lassoUnselect");
    console.log("lassoUnselect hotkey:", lassoUnselectHotkey);

    // Check all modifiers that could be relevant for a potential drag action
    const isLassoUnselect = checkMouseModifier(event, "lassoUnselect");
    const isLassoSelect = checkMouseModifier(event, "lassoSelect");

    // Also check click-based operations
    const isUnselectRange = checkMouseModifier(event, "unselectRange");
    const isSelectRange = checkMouseModifier(event, "selectRange");

    // Set deselect mode if any deselect operation is active
    isDeselectMode = isLassoUnselect || isUnselectRange;

    // Log detailed detection status for debugging
    console.log("Modifier detection:", {
      isLassoUnselect,
      isLassoSelect,
      isUnselectRange,
      isSelectRange,
      isDeselectMode,
      modifierKeys: {
        alt: event.altKey,
        shift: event.shiftKey,
        ctrl: event.ctrlKey,
        meta: event.metaKey,
      },
    });

    // No longer apply global classes to the body element
    // The mode is tracked with the isDeselectMode variable

    // Set up global mouse move and mouse up handlers
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);

    // Important: Don't update selections here - wait for mouseMove (drag) or mouseUp (click)
    // This prevents interfering with the toggle/shift-click behavior
  }

  function handleMouseMove(event: MouseEvent) {
    if (!enableSelections) return;

    // Update UI feedback for hotkey modifiers
    updateHotkeyFeedback(event);

    // Determine if we're just now starting a drag operation
    const distanceMoved = Math.sqrt(
      Math.pow(event.clientX - mouseDownPosition.x, 2) +
        Math.pow(event.clientY - mouseDownPosition.y, 2)
    );

    const startingDrag = !isDragging && distanceMoved > 5;

    if (startingDrag) {
      // Re-evaluate modifiers at the moment drag starts
      // This ensures we correctly identify lasso operations
      const isLassoUnselect = checkMouseModifier(event, "lassoUnselect");

      // If deselect mode was not set in mouseDown but should be active for this drag,
      // update it now that we know it's a drag operation
      if (isLassoUnselect) {
        isDeselectMode = true;
      }

      console.log(
        "Starting drag, deselect mode:",
        isDeselectMode,
        "lasso unselect:",
        isLassoUnselect,
        "distance:",
        distanceMoved
      );
    }

    // If we haven't started dragging yet, check if we should start
    if (!isDragging) {
      // Start dragging if the mouse has moved more than 5 pixels
      if (distanceMoved > 5) {
        isDragging = true;

        // When drag starts, we need to verify which modifier is active
        // and update both the deselect mode and hotkey feedback
        const isLassoUnselect = checkMouseModifier(event, "lassoUnselect");
        const isLassoSelect = checkMouseModifier(event, "lassoSelect");

        // Log detailed information about the drag start
        console.log("Drag started with modifiers:", {
          isLassoUnselect,
          isLassoSelect,
          currentModifiers: {
            alt: event.altKey,
            shift: event.shiftKey,
            ctrl: event.ctrlKey,
            meta: event.metaKey,
          },
        });

        // Set deselect mode if lasso unselect is active
        if (isLassoUnselect) {
          isDeselectMode = true;
          console.log("Deselect mode activated at drag start");

          // Add visual indicator for deselect mode
          document.body.classList.add("deselect-drag-active");
        }

        // Handle the initial item selection when drag starts
        const itemId = filteredItems[dragStartIndex].id;

        // When starting a drag, we handle the initial selection differently
        selectedItemsStore.update((currentSelected) => {
          const newSelected = new Set(currentSelected);

          // Check if this is a deselect drag (using customized hotkey setting)
          if (isDeselectMode) {
            // In deselect mode, always remove the clicked item
            newSelected.delete(itemId);
          } else {
            // In select mode, always add the clicked item (don't toggle)
            // For drag operations, we always want to select the start item
            // rather than toggle it, to make the operation predictable
            newSelected.add(itemId);
          }

          // Update the last selected index for potential future shift-clicks
          lastSelectedIndexStore.set(dragStartIndex);
          return newSelected;
        });
      } else {
        // Still not dragging, just return
        return;
      }
    }

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

          // If moving away from start point, apply selection action based on mode
          if (
            (dragStartIndex < currentIndex && i > dragStartIndex) ||
            (dragStartIndex > currentIndex && i < dragStartIndex)
          ) {
            if (isDeselectMode) {
              // In deselect mode, remove items from selection
              newSelected.delete(itemId);
            } else {
              // In select mode, add items to selection
              newSelected.add(itemId);
            }
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

  function handleMouseUp(event: MouseEvent) {
    // Update UI feedback for hotkey modifiers
    updateHotkeyFeedback(event);

    // Calculate time from mouseDown to mouseUp
    const clickDuration = Date.now() - mouseDownTime;

    // Calculate distance moved between mouseDown and mouseUp
    const distanceMoved = Math.sqrt(
      Math.pow(event.clientX - mouseDownPosition.x, 2) +
        Math.pow(event.clientY - mouseDownPosition.y, 2)
    );

    // If it wasn't a drag operation (short duration and minimal movement), treat as a click
    if (!isDragging && clickDuration < 300 && distanceMoved < 5) {
      // This was a click, not a drag - handle the click according to the hotkey configuration
      const itemClicked = filteredItems[dragStartIndex];
      if (itemClicked) {
        // Create a new MouseEvent that preserves the modifier keys
        const clickEvent = new MouseEvent("click", {
          altKey: event.altKey,
          shiftKey: event.shiftKey,
          ctrlKey: event.ctrlKey,
          metaKey: event.metaKey,
          bubbles: true,
          cancelable: true,
          view: window,
        });

        // Use the toggleSelect function, but let's adapt the mouse event
        // to the custom hotkey configurations
        toggleSelect(itemClicked, clickEvent);
      }
    }

    // Clean up regardless of whether it was a drag or click
    const wasInDeselectMode = isDeselectMode;
    const wasDragging = isDragging;

    // Reset all state variables
    isDragging = false;
    isDeselectMode = false;
    dragSelectionState.clear();

    // Remove any visual indicators
    document.body.classList.remove("deselect-drag-active");
    document.body.classList.remove("deselect-mode");

    // Log the end of the operation for debugging
    if (wasDragging) {
      console.log(
        "Drag operation ended. Was in deselect mode:",
        wasInDeselectMode
      );
    }

    // Remove global event listeners
    window.removeEventListener("mousemove", handleMouseMove);
    window.removeEventListener("mouseup", handleMouseUp);
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

      // Update column width using store method
      columnConfigStore.updateColumnWidth(index, newWidth);
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

  async function fetchData() {
    try {
      loading = true;
    } catch (error) {
      console.error("Failed to fetch data:", error);
    } finally {
      loading = false;
    }
  }

  onMount(async () => {
    loading = false;
    fetchData();

    // Listen for font-size changes and update the virtualizer
    const fontSizeListener = listen("font-size-updated", () => {
      if ($rowVirtualizer) {
        $rowVirtualizer.measure();
      }
    });

    // Add keyboard event listeners
    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("keyup", handleKeyUp);

    // Listen for remove status updates
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

    // Set up ResizeObserver for the virtual table
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
      fontSizeListener.then((unsubscribe) => unsubscribe());
      resizeObserver.disconnect();
    };
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

  onDestroy(() => {
    if (unlistenRemoveFn) unlistenRemoveFn();

    // Clean up any event listeners
    window.removeEventListener("mousemove", handleMouseMove);
    window.removeEventListener("mouseup", handleMouseUp);
    window.removeEventListener("keydown", handleKeyDown);
    window.removeEventListener("keyup", handleKeyUp);
  });

  // Track Alt+Shift key state for UI feedback
  let isAltShiftPressed = false;

  function handleKeyDown(event: KeyboardEvent) {
    updateHotkeyFeedback(event);

    // For backward compatibility, still track Alt+Shift specifically
    isAltShiftPressed = event.altKey && event.shiftKey;
  }

  function handleKeyUp(event: KeyboardEvent) {
    updateHotkeyFeedback(event);

    // For backward compatibility, still track Alt+Shift specifically
    isAltShiftPressed = event.altKey && event.shiftKey;
  }

  // Track key states for UI feedback
  let activeHotkeyModifiers = {
    toggleSelectAll: false,
    selectRange: false,
    unselectRange: false,
    lassoSelect: false,
    lassoUnselect: false,
  };

  // Update UI feedback based on current key state
  function updateHotkeyFeedback(event: KeyboardEvent | MouseEvent) {
    const toggleSelectAllMods = getModifiersFromHotkey("toggleSelectAll");
    const selectRangeMods = getModifiersFromHotkey("selectRange");
    const unselectRangeMods = getModifiersFromHotkey("unselectRange");
    const lassoSelectMods = getModifiersFromHotkey("lassoSelect");
    const lassoUnselectMods = getModifiersFromHotkey("lassoUnselect");

    // Calculate each modifier match separately
    const toggleSelectAllMatch =
      toggleSelectAllMods.alt === event.altKey &&
      toggleSelectAllMods.shift === event.shiftKey &&
      toggleSelectAllMods.meta === (event.metaKey || event.ctrlKey);

    const selectRangeMatch =
      selectRangeMods.alt === event.altKey &&
      selectRangeMods.shift === event.shiftKey &&
      selectRangeMods.meta === (event.metaKey || event.ctrlKey);

    const unselectRangeMatch =
      unselectRangeMods.alt === event.altKey &&
      unselectRangeMods.shift === event.shiftKey &&
      unselectRangeMods.meta === (event.metaKey || event.ctrlKey);

    const lassoSelectMatch =
      lassoSelectMods.alt === event.altKey &&
      lassoSelectMods.shift === event.shiftKey &&
      lassoSelectMods.meta === (event.metaKey || event.ctrlKey);

    const lassoUnselectMatch =
      lassoUnselectMods.alt === event.altKey &&
      lassoUnselectMods.shift === event.shiftKey &&
      lassoUnselectMods.meta === (event.metaKey || event.ctrlKey);

    // Update active modifiers
    activeHotkeyModifiers = {
      toggleSelectAll: toggleSelectAllMatch,
      selectRange: selectRangeMatch,
      unselectRange: unselectRangeMatch,
      lassoSelect: lassoSelectMatch,
      lassoUnselect: lassoUnselectMatch,
    };

    // We no longer apply any global overlay classes
    // Just track the state internally for cursor changes

    // Debug log for hotkey detection if needed
    if (event.altKey || event.shiftKey || event.metaKey || event.ctrlKey) {
      console.log("Active modifiers:", {
        ...activeHotkeyModifiers,
        keys: {
          alt: event.altKey,
          shift: event.shiftKey,
          meta: event.metaKey || event.ctrlKey,
        },
        isDragging,
      });
    }
  }

  // Helper function to get modifiers from a hotkey name
  function getModifiersFromHotkey(hotkeyName: string): {
    alt: boolean;
    shift: boolean;
    meta: boolean;
  } {
    const hotkey = getHotkey(hotkeyName);
    return {
      alt: hotkey.includes("Alt+"),
      shift: hotkey.includes("Shift+"),
      meta: hotkey.includes("CmdOrCtrl+"),
    };
  }

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
    ChevronUp,
    ChevronDown,
    Clock,
    GitCompareArrowsIcon,
    Hash,
    ShieldCheck,
    Search,
    Activity,
    ArrowLeftRight,
  } from "lucide-svelte";
  import { preferencesStore } from "../../stores/preferences";
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
  let parentHeight = 0; // Base row height calculation responsive to font size
  $: baseItemSize = Math.round(
    parseFloat(
      getComputedStyle(document.documentElement).getPropertyValue("--font-size")
    ) *
      1.0 +
      $preferencesStore.fontSize // Base font size + current font size setting
  );

  // Dynamic row height estimation that accounts for group positioning
  function estimateRowSize(index: number): number {
    let size = baseItemSize;

    // Add extra space for group-end items (8px bottom padding)
    if (isGroupEnd(index)) {
      size += 8;
    }

    // Add extra space for group-start items (2px top padding)
    if (isGroupStart(index)) {
      size += 2;
    }

    return size;
  }

  export let overscan = 5;

  // Extract just the length to avoid virtualizer recreation on array reference changes
  $: itemCount = filteredItems.length;

  // Create vertical virtualizer for rows - only recreate when count actually changes
  $: rowVirtualizer = createVirtualizer({
    count: itemCount,
    estimateSize: estimateRowSize,
    overscan,
    getScrollElement: () => parentRef,
  });
</script>

<div
  class="virtual-table-container"
  style="--total-width: {totalWidth}"
  bind:this={containerElement}
>
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
    <div class="virtual-table-header">
      <div
        class="grid-container rheader"
        style="grid-template-columns: {gridTemplateColumns}"
      >
        {#each columnConfigs as key, i}
          <div
            class="grid-item header {i === 0
              ? 'sticky-column'
              : ''} sortable-header"
            on:click={() => handleHeaderClick(key.name)}
            role="button"
            tabindex="0"
            on:keydown={(e) => e.key === "Enter" && handleHeaderClick(key.name)}
          >
            {#if sortColumn === key.name}
              {#if sortDirection === "asc"}
                <ChevronUp size={16} />
              {:else}
                <ChevronDown size={16} />
              {/if}
            {/if}
            <span>{key.header}</span>
          </div>
        {/each}
      </div>

      <div
        class="resizer-container"
        style="grid-template-columns: {gridTemplateColumns}; display: grid; width: 100%;"
      >
        {#each columnConfigs as column, i}
          <div class="resizer-cell">
            {#if i > 0 && i < 8}
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
      style="height: {$rowVirtualizer.getTotalSize()}px;"
    >
      {#each $rowVirtualizer.getVirtualItems() as virtualRow (virtualRow.index)}
        <div
          class="virtual-row {activeHotkeyModifiers.unselectRange
            ? 'unselect-range-hover'
            : activeHotkeyModifiers.toggleSelectAll
              ? 'toggle-select-all-hover'
              : activeHotkeyModifiers.selectRange
                ? 'select-range-hover'
                : activeHotkeyModifiers.lassoSelect
                  ? 'lasso-select-hover'
                  : activeHotkeyModifiers.lassoUnselect
                    ? 'lasso-unselect-hover'
                    : isAltShiftPressed
                      ? 'alt-shift-range-deselect'
                      : ''}"
          data-index={virtualRow.index}
          style="transform: translateY({virtualRow.start}px); height: {virtualRow.size}px; "
        >
          <div
            class="list-item {filteredItems[
              virtualRow.index
            ].algorithm.includes('Keep')
              ? 'unselected-item'
              : 'checked-item'} {isGroupSizeOne(virtualRow.index)
              ? 'group-size-1'
              : isGroupStart(virtualRow.index)
                ? 'group-start'
                : isGroupEnd(virtualRow.index)
                  ? 'group-end'
                  : ''}"
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
                    on:mousedown={(event) =>
                      enableSelections
                        ? handleMouseDown(virtualRow.index, event)
                        : toggleChecked(filteredItems[virtualRow.index])}
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
                    on:mousedown={(event) =>
                      enableSelections
                        ? handleMouseDown(virtualRow.index, event)
                        : toggleChecked(filteredItems[virtualRow.index])}
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
    min-height: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    font-weight: bold; /* Add this line */
  }

  .virtual-table-viewport {
    overflow: auto;
    flex: 1;
    min-height: 100%;
    will-change: transform;
    position: relative;
  }

  .virtual-table-header {
    width: max(var(--total-width), 100%);
    position: sticky;
    top: 0;
    z-index: 10;
    background-color: var(--primary-bg);
    box-shadow: 0 1px 0 rgba(0, 0, 0, 0.1);
    border-top: 1px solid var(--inactive-color);
    margin-top: 0px;
  }

  .virtual-table-body {
    position: relative;
  }

  .virtual-row {
    position: absolute;
    top: 0;
    left: 0;

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
    height: calc(var(--font-size-xl) * 2);
    background-color: var(--inactive-color);
    position: absolute;
    right: 1px;
    transform: translateX(50%); /* Center on the boundary */
    top: calc(var(--font-size-xl) * -2);
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
    padding: 3px; /* Scale padding based on font size */
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--font-size-md);
    user-select: none;
    -webkit-user-select: none;
    -moz-user-select: none;
    -ms-user-select: none;
    width: 100%;
    min-height: var(--font-size);
    font-weight: bold; /* Add this line */ /* Ensure minimum height for cell contents */
  }

  .grid-item.header {
    font-size: var(--font-size);
    background-color: var(--secondary-bg);
    margin-top: 0px;
    /* text-align: center;
    display: flex;
    align-items: center;
    justify-content: center; */
  }

  .sortable-header {
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: flex-start;
    gap: 6px;
  }

  .rheader {
    font-weight: bold;
    font-size: var(--font-size);
    color: var(--accent-color);
    background-color: var(--secondary-bg);
    border-bottom: 1px solid var(--inactive-color);
    margin-left: 4px;
    margin-top: 0px;
    height: calc(
      var(--font-size-xl) * 2
    ); /* Adjust height based on font size */
    text-align: bottom;
    align-items: end;
    width: max(100%, var(--total-width));
    font-weight: bold; /* Add this line */
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
    width: 100%;
    padding: 0px;
    margin: 0px;
    border-bottom: none;
  }

  .list-item.group-start {
    border-top: 1px solid var(--inactive-color);
    padding-top: 2px;
  }
  .list-item.group-end {
    padding-bottom: 8px;
  }
  .list-item.group-size-1 {
    border-top: 1px solid var(--inactive-color);
    padding-top: 2px;
    padding-bottom: 8px;
  }

  .algorithm-icons {
    display: flex;
    flex-wrap: nowrap;
    gap: 8px;
    overflow: hidden;
    align-items: center;
    height: calc(var(--font-size) * 1.8);
    max-height: calc(var(--font-size) * 1.8);
  }

  .icon-wrapper {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    min-width: 20px;
    height: 20px;
  }

  .icon-wrapper:hover::after {
    content: attr(title);
    position: absolute;
    background: var(--primary-bg);
    border: 1px solid var(--border-color);
    padding: 2px 6px;
    border-radius: 4px;
    font-size: calc(var(--font-size-xs) - 2px);
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

  /* Cursor styles for different mouse modifiers - only change cursor without outlines */
  .alt-shift-range-deselect,
  .unselect-range-hover,
  .lasso-unselect-hover {
    cursor: no-drop;
  }

  .select-range-hover {
    cursor: cell;
  }

  .toggle-select-all-hover {
    cursor: crosshair;
  }

  .lasso-select-hover {
    cursor: grab;
  }

  /* Style for when in deselect mode during a drag */
  /* Use a very subtle indicator in the status bar or other unobtrusive UI element instead */
</style>
