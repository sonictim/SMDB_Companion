<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { Square, CheckSquare, OctagonX, GripVertical } from "lucide-svelte";

  import { preferencesStore } from "../../store";
  import { get } from "svelte/store";
  import type { PreservationLogic } from "../../store";
  import { onMount } from "svelte";

  onMount(() => {
    // Minimal logging for initialization
    console.log("Drag component mounted");
  });

  $: pref = $preferencesStore;
  let selectedItems = new Set<PreservationLogic>();
  let newOption: string = "";
  let currentOperator = "Contains";
  let currentColumn = "FilePath";

  // Drag and drop state
  let dragSourceIndex: number | null = null;
  let dropTargetIndex: number | null = null;
  let draggedItem: PreservationLogic | null = null;

  let operators = [
    { id: "Is", name: "is" },
    { id: "IsNot", name: "is NOT" },
    { id: "Largest", name: "is largest" },
    { id: "Smallest", name: "is smallest" },
    { id: "IsEmpty", name: "is empty" },
    { id: "IsNotEmpty", name: "is NOT empty" },
    { id: "Contains", name: "contains" },
    { id: "DoesNotContain", name: "does NOT contain" },
  ];

  function toggleSelected(item: PreservationLogic) {
    if (selectedItems.has(item)) {
      selectedItems.delete(item);
    } else {
      selectedItems.add(item);
    }
    selectedItems = new Set(selectedItems); // Ensure reactivity
  }

  function removeSelected(list: PreservationLogic[]) {
    preferencesStore.update((pref) => ({
      ...pref,
      preservation_order: pref.preservation_order.filter(
        (item) => !list.includes(item),
      ),
    }));
    clearSelected();
  }

  function clearSelected() {
    selectedItems.clear();
    selectedItems = new Set(); // Ensure reactivity
  }

  function handleOperatorChange(event: Event) {
    currentOperator = (event.target as HTMLSelectElement).value;
  }

  function handleColumnChange(event: Event) {
    currentColumn = (event.target as HTMLSelectElement).value;
  }

  function addOrder() {
    if (
      !pref.preservation_order.some(
        (item) =>
          item.column === currentColumn &&
          item.operator === currentOperator && // Using operator.id here
          item.variable === newOption,
      )
    ) {
      preferencesStore.update((pref) => ({
        ...pref,
        preservation_order: [
          {
            column: currentColumn,
            operator: currentOperator,
            variable: newOption,
          },
          ...pref.preservation_order,
        ],
      }));
      newOption = "";
    }
  }

  function onDragStart(event: DragEvent, index: number) {
    if (!event.dataTransfer) return;

    // Store the dragged item instead of removing it immediately
    draggedItem = pref.preservation_order[index];
    dragSourceIndex = index;

    event.dataTransfer.setData("text/plain", index.toString());
    event.dataTransfer.effectAllowed = "move";

    // Add visual indicator for the dragged item
    if (event.currentTarget instanceof HTMLElement) {
      event.currentTarget.classList.add("dragging");
    }
  }

  function onDragOver(event: DragEvent, index: number) {
    event.preventDefault();
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = "move";
    }

    if (dragSourceIndex !== null && dragSourceIndex !== index) {
      dropTargetIndex = index;
    }
  }

  function onDragEnter(event: DragEvent, index: number) {
    event.preventDefault();

    if (dragSourceIndex !== null && dragSourceIndex !== index) {
      dropTargetIndex = index;
    }
  }

  function onDrop(event: DragEvent) {
    event.preventDefault();

    if (dragSourceIndex !== null && dropTargetIndex !== null && draggedItem) {
      preferencesStore.update((pref) => {
        const newOrder = [...pref.preservation_order];

        // First remove the item from its original position
        newOrder.splice(dragSourceIndex!, 1);

        // Then insert it at the new position, adjusting for the removal
        const adjustedTargetIndex =
          dropTargetIndex! > dragSourceIndex!
            ? dropTargetIndex! - 1
            : dropTargetIndex!;

        newOrder.splice(adjustedTargetIndex, 0, draggedItem!);

        return { ...pref, preservation_order: newOrder };
      });
    }

    // Clean up
    dragSourceIndex = null;
    dropTargetIndex = null;
    draggedItem = null;

    // Remove any drag-related classes
    document.querySelectorAll(".dragging").forEach((el) => {
      el.classList.remove("dragging");
    });
  }

  function onDragEnd(event: DragEvent) {
    // Clean up if drag operation was cancelled
    dragSourceIndex = null;
    dropTargetIndex = null;
    draggedItem = null;

    document.querySelectorAll(".dragging").forEach((el) => {
      el.classList.remove("dragging");
    });
  }
</script>

<div class="block">
  <div class="header">
    <h2>Duplicate Selection Logic</h2>
    <button
      class="cta-button cancel"
      on:click={() => removeSelected([...selectedItems])}
    >
      <OctagonX size={18} />
      Remove Selected
    </button>
  </div>

  <div class="bar">
    <button class="cta-button small" on:click={addOrder}>Add</button>
    <select
      class="select-field"
      style="flex-grow: 0"
      bind:value={currentColumn}
      on:change={handleColumnChange}
    >
      {#each pref.columns as option}
        <option value={option}>{option}</option>
      {/each}
    </select>
    <select
      class="select-field"
      style="flex-grow: 0"
      bind:value={currentOperator}
      on:change={handleOperatorChange}
    >
      {#each operators as option}
        <option value={option.id}>{option.name}</option>
      {/each}
    </select>

    {#if ["Contains", "DoesNotContain", "Is", "IsNot"].includes(currentOperator)}
      <input
        type="text"
        bind:value={newOption}
        placeholder="Enter New Option"
        class="input-field"
      />
    {/if}
  </div>
  <div class="block inner">
    {#each pref.preservation_order as item, i}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="item-container"
        class:drop-target={dropTargetIndex === i}
        on:dragenter|capture={(e) => onDragEnter(e, i)}
        on:dragover={(e) => onDragOver(e, i)}
      >
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <div
          class="list-item"
          class:selected-item={selectedItems.has(item)}
          class:unselected-item={!selectedItems.has(item)}
          draggable="true"
          role="listitem"
          on:dragstart={(e) => onDragStart(e, i)}
          on:dragend={onDragEnd}
          on:drop={onDrop}
          on:click={() => toggleSelected(item)}
        >
          <div class="drag-handle">
            <GripVertical size={18} />
          </div>
          <div class="item-content">
            {i + 1}. {item.column}
            {#if operators.some((op) => op.id === item.operator)}
              {operators.find((op) => op.id === item.operator)?.name}
            {/if}
            {#if ["Contains", "DoesNotContain", "Is", "IsNot"].includes(item.operator)}
              `{item.variable}`
            {/if}
          </div>
        </div>
        {#if dropTargetIndex === i && dragSourceIndex !== i}
          <div class="drop-indicator"></div>
        {/if}
      </div>
    {/each}

    <!-- End drop area -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="end-drop-area"
      class:drop-target={dropTargetIndex === pref.preservation_order.length}
      on:dragenter={(e) => {
        e.preventDefault();
        dropTargetIndex = pref.preservation_order.length;
      }}
      on:dragover={(e) => {
        e.preventDefault();
        dropTargetIndex = pref.preservation_order.length;
      }}
      on:drop={onDrop}
    >
      {#if dropTargetIndex === pref.preservation_order.length}
        <div class="drop-indicator"></div>
      {/if}
    </div>
  </div>
</div>

<style>
  .item-container {
    position: relative;
    margin: 2px 0;
  }

  .list-item {
    display: flex;
    align-items: center;
    padding: 8px;
    border-radius: 4px;
    background-color: var(--primary-bg-color);
    transition: all 0.2s;
  }

  .selected-item {
    background-color: var(--accent-color);
  }

  .drag-handle {
    cursor: grab;
    margin-right: 10px;
    display: flex;
    align-items: center;
  }

  .item-content {
    flex-grow: 1;
    cursor: pointer;
  }

  .dragging {
    opacity: 0.5;
  }

  .drop-target {
    background-color: rgba(59, 130, 246, 0.1);
  }

  .drop-indicator {
    height: 3px;
    background-color: #c29a07;
    width: 100%;
    position: absolute;
    top: -2px;
    left: 0;
    z-index: 10;
  }

  .end-drop-area {
    height: 20px;
    position: relative;
  }

  .grid-container {
    display: grid;
    grid-template-columns: 1fr 2fr; /* First column 1/3, second column 2/3 */
    gap: 10px;
  }

  .block {
    height: calc(100vh - 160px);
  }
</style>
