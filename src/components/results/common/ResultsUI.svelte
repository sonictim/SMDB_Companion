<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { FileRecord } from "../../../stores/types";
  import {
    toggleChecked,
    toggleSelect,
    filteredItemsStore,
  } from "../../../stores/results";
  import { preferencesStore } from "../../../stores/preferences";

  // Props
  export let items: FileRecord[] = [];
  export let selectedItems: Set<number> = new Set();
  export let mode: "standard" | "skinny" | "advanced" = "standard";
  export let isVirtual: boolean = true;

  // Event dispatcher
  const dispatch = createEventDispatcher<{
    itemClick: { item: FileRecord };
    checkToggle: { item: FileRecord; checked: boolean };
    playAudio: { item: FileRecord };
  }>();

  $: pref = $preferencesStore;

  // Item click handler
  function handleItemClick(item: FileRecord) {
    toggleSelect(item.id);
    dispatch("itemClick", { item });
  }

  // Checkbox toggle handler
  function handleCheckToggle(item: FileRecord) {
    toggleChecked(item.id);
    dispatch("checkToggle", { item, checked: !item.checked });
  }

  // Play audio handler
  function handlePlayAudio(item: FileRecord) {
    dispatch("playAudio", { item });
  }

  // Format file size for display
  function formatFileSize(size: number): string {
    if (size < 1024) return `${size} B`;
    if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
    if (size < 1024 * 1024 * 1024)
      return `${(size / 1024 / 1024).toFixed(1)} MB`;
    return `${(size / 1024 / 1024 / 1024).toFixed(1)} GB`;
  }

  // Format duration for display
  function formatDuration(seconds: number): string {
    if (isNaN(seconds)) return "--:--";

    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, "0")}`;
  }
</script>

<div class="results-ui">
  <slot name="header">
    <!-- Header slot for custom headers -->
  </slot>

  <div class="results-table-container">
    <table class="results-table" class:skinny-mode={mode === "skinny"}>
      <thead>
        <tr>
          <th class="checkbox-column">
            <slot name="select-all-checkbox">
              <!-- Checkbox for select all -->
            </slot>
          </th>
          <th class="name-column">Name</th>
          <th class="algorithm-column">Match</th>
          {#if mode !== "skinny"}
            <th class="duration-column">Length</th>
            <th class="size-column">Size</th>
          {/if}
          <th class="actions-column"></th>
        </tr>
      </thead>
      <tbody>
        {#if items.length === 0}
          <tr class="empty-row">
            <td colspan={mode === "skinny" ? 4 : 6}>
              <div class="empty-message">
                <slot name="empty-message">No results found</slot>
              </div>
            </td>
          </tr>
        {:else}
          {#each items as item (item.id)}
            <tr
              class="result-row"
              class:selected={selectedItems.has(item.id)}
              class:keep={item.algorithm?.includes("Keep")}
              on:click={() => handleItemClick(item)}
            >
              <td class="checkbox-column">
                <input
                  type="checkbox"
                  checked={item.checked}
                  on:change={() => handleCheckToggle(item)}
                  on:click={(e) => e.stopPropagation()}
                />
              </td>
              <td class="name-column">
                <div class="file-name">
                  {item.filename}
                </div>
                {#if mode !== "skinny"}
                  <div class="file-path text-subtle">
                    {item.path}
                  </div>
                {/if}
              </td>
              <td class="algorithm-column">
                <span class="algorithm-tag {item.algorithm?.toLowerCase()}">
                  {item.algorithm || "Unknown"}
                </span>
              </td>
              {#if mode !== "skinny"}
                <td class="duration-column">
                  {formatDuration(item.duration || 0)}
                </td>
                <td class="size-column">
                  {formatFileSize(item.filesize || 0)}
                </td>
              {/if}
              <td class="actions-column">
                <button
                  class="play-button"
                  title="Play audio"
                  on:click={(e) => {
                    e.stopPropagation();
                    handlePlayAudio(item);
                  }}
                >
                  ▶
                </button>
              </td>
            </tr>
          {/each}
        {/if}
      </tbody>
    </table>
  </div>

  <slot name="footer">
    <!-- Footer slot for custom footers -->
  </slot>
</div>

<style>
  .results-ui {
    display: flex;
    flex-direction: column;
    height: 100%;
    width: 100%;
    overflow: hidden;
  }

  .results-table-container {
    flex: 1;
    overflow-y: auto;
    border: 1px solid var(--inactive-color);
    border-radius: 4px;
  }

  .results-table {
    width: 100%;
    border-collapse: collapse;
    table-layout: fixed;
  }

  .results-table th {
    position: sticky;
    top: 0;
    background-color: var(--secondary-bg);
    color: var(--text-color);
    text-align: left;
    padding: 8px 12px;
    font-weight: 600;
    border-bottom: 2px solid var(--accent-color);
    z-index: 1;
  }

  .result-row {
    border-bottom: 1px solid var(--inactive-color);
    transition: background-color 0.15s;
    cursor: pointer;
  }

  .result-row:hover {
    background-color: rgba(255, 255, 255, 0.05);
  }

  .result-row.selected {
    background-color: rgba(var(--accent-color-rgb), 0.2);
  }

  .result-row.keep {
    color: var(--inactive-color);
  }

  .results-table td {
    padding: 8px 12px;
    vertical-align: middle;
  }

  .checkbox-column {
    width: 40px;
    text-align: center;
  }

  .name-column {
    min-width: 250px;
    max-width: 40%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .algorithm-column {
    width: 120px;
  }

  .duration-column,
  .size-column {
    width: 80px;
    text-align: right;
  }

  .actions-column {
    width: 50px;
    text-align: center;
  }

  .file-name {
    font-weight: 500;
  }

  .file-path {
    font-size: 0.8rem;
    opacity: 0.7;
  }

  .text-subtle {
    color: var(--inactive-color);
  }

  .algorithm-tag {
    display: inline-block;
    padding: 2px 6px;
    border-radius: 3px;
    font-size: 0.8rem;
    background-color: var(--accent-color);
    color: white;
  }

  .algorithm-tag.keep {
    background-color: #4caf50;
  }

  .algorithm-tag.exact {
    background-color: #f44336;
  }

  .algorithm-tag.similar {
    background-color: #ff9800;
  }

  .algorithm-tag.dualmono {
    background-color: #9c27b0;
  }

  .play-button {
    background-color: var(--accent-color);
    color: white;
    border: none;
    border-radius: 50%;
    width: 24px;
    height: 24px;
    font-size: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: background-color 0.2s;
  }

  .play-button:hover {
    background-color: var(--hover-color);
  }

  .empty-row td {
    height: 200px;
    text-align: center;
  }

  .empty-message {
    color: var(--inactive-color);
    font-style: italic;
  }

  /* Adjustments for skinny mode */
  .results-table.skinny-mode .name-column {
    max-width: 60%;
  }

  @media (max-width: 768px) {
    .duration-column,
    .size-column {
      display: none;
    }
  }
</style>
