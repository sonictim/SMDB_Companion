<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { FileRecord } from "../../../stores/types";
  import {
    clearSelected,
    invertSelected,
    checkSelected,
    uncheckSelected,
    toggleChecksSelected,
    selectedItemsStore,
  } from "../../../stores/results";

  // Props
  export let mode: "standard" | "skinny" | "advanced" = "standard";
  export let processing: boolean = false;

  // Event dispatcher
  const dispatch = createEventDispatcher<{
    action: { name: string; data?: any };
  }>();

  // Selection actions
  function handleClearSelected() {
    clearSelected();
    dispatch("action", { name: "clearSelected" });
  }

  function handleInvertSelected() {
    invertSelected();
    dispatch("action", { name: "invertSelected" });
  }

  function handleCheckSelected() {
    checkSelected();
    dispatch("action", { name: "checkSelected" });
  }

  function handleUncheckSelected() {
    uncheckSelected();
    dispatch("action", { name: "uncheckSelected" });
  }

  function handleToggleChecksSelected() {
    toggleChecksSelected();
    dispatch("action", { name: "toggleChecksSelected" });
  }

  // Export actions
  function handleExportSelected() {
    dispatch("action", { name: "exportSelected" });
  }

  function handleExportAll() {
    dispatch("action", { name: "exportAll" });
  }

  // Play audio functionality
  function handlePlayAudio(item: FileRecord) {
    dispatch("action", { name: "playAudio", data: item });
  }

  // Sort functionality
  function handleSort(field: string) {
    dispatch("action", { name: "sort", data: { field } });
  }
</script>

<div class="actions-container">
  <slot name="custom-actions-top">
    <!-- Custom actions can be inserted here -->
  </slot>

  <div class="action-group selections">
    <h3>Selection Actions</h3>
    <div class="button-row">
      <button
        class="action-button"
        on:click={handleClearSelected}
        disabled={processing}
        title="Clear all selections"
      >
        Clear
      </button>

      <button
        class="action-button"
        on:click={handleInvertSelected}
        disabled={processing}
        title="Invert selections"
      >
        Invert
      </button>

      <button
        class="action-button"
        on:click={handleCheckSelected}
        disabled={processing}
        title="Check selected items"
      >
        Check
      </button>

      <button
        class="action-button"
        on:click={handleUncheckSelected}
        disabled={processing}
        title="Uncheck selected items"
      >
        Uncheck
      </button>

      <button
        class="action-button"
        on:click={handleToggleChecksSelected}
        disabled={processing}
        title="Toggle checks for selected items"
      >
        Toggle
      </button>
    </div>
  </div>

  <slot name="custom-actions-middle">
    <!-- Custom actions can be inserted here -->
  </slot>

  <div class="action-group exports">
    <h3>Export Actions</h3>
    <div class="button-row">
      <button
        class="action-button export"
        on:click={handleExportSelected}
        disabled={processing || $selectedItemsStore.size === 0}
        title="Export selected items"
      >
        Export Selected
      </button>

      <button
        class="action-button export"
        on:click={handleExportAll}
        disabled={processing}
        title="Export all items"
      >
        Export All
      </button>
    </div>
  </div>

  <slot name="custom-actions-bottom">
    <!-- Custom actions can be inserted here -->
  </slot>
</div>

<style>
  .actions-container {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 16px;
    background-color: var(--secondary-bg);
    border-radius: 4px;
  }

  .action-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .action-group h3 {
    font-size: 1rem;
    margin: 0;
    color: var(--accent-color);
  }

  .button-row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .action-button {
    padding: 6px 12px;
    border-radius: 4px;
    border: none;
    background-color: var(--accent-color);
    color: white;
    cursor: pointer;
    transition: background-color 0.2s;
    font-size: 0.9rem;
  }

  .action-button:hover:not(:disabled) {
    background-color: var(--hover-color);
  }

  .action-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .action-button.export {
    background-color: var(--accent-color);
  }

  .action-button.export:hover:not(:disabled) {
    background-color: var(--hover-color);
  }

  /* Responsive adjustments for different modes */
  @media (max-width: 768px) {
    .button-row {
      flex-direction: column;
    }
  }
</style>
