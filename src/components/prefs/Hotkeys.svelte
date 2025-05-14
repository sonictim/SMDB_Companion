<script lang="ts">
  import { hotkeysStore, defaultHotKeys } from "../../stores/hotkeys";
  import { onMount } from "svelte";
  import type { HotKeys } from "../../stores/types";

  // Deep copy of the hotkeys store for local state management
  let localHotkeys: HotKeys = { ...defaultHotKeys };
  let editing: keyof HotKeys | null = null;
  let currentKey = "";
  let isRecordingKey = false;
  let searchTerm = "";

  // Subscribe to hotkeys store changes
  onMount(() => {
    const unsubscribe = hotkeysStore.subscribe((value: any) => {
      localHotkeys = JSON.parse(JSON.stringify(value)); // Deep copy
    });

    return unsubscribe;
  });

  // Start recording a new hotkey
  function startRecording(key: keyof HotKeys) {
    editing = key;
    currentKey = localHotkeys[key];
    isRecordingKey = true;
  }

  // Save the current recorded key
  function saveHotkey() {
    if (editing) {
      localHotkeys[editing] = currentKey;
      hotkeysStore.set(localHotkeys);
      editing = null;
      isRecordingKey = false;
    }
  }

  // Cancel current recording
  function cancelRecording() {
    if (editing) {
      currentKey = localHotkeys[editing];
    }
    editing = null;
    isRecordingKey = false;
  }

  // Handle key press during recording
  function handleKeyDown(event: KeyboardEvent) {
    if (!isRecordingKey) return;

    event.preventDefault();
    event.stopPropagation();

    let keys: string[] = [];

    // Add modifiers
    if (event.metaKey || event.ctrlKey) keys.push("CmdOrCtrl");
    if (event.shiftKey) keys.push("Shift");
    if (event.altKey) keys.push("Alt");

    // Add the key if it's not a modifier
    if (!["Meta", "Control", "Shift", "Alt"].includes(event.key)) {
      // Convert key name to appropriate format
      let key = event.key;

      // Handle special keys
      if (key === " ") key = "Space";
      else if (key === "Escape") key = "Esc";
      else if (key === "ArrowUp") key = "Up";
      else if (key === "ArrowDown") key = "Down";
      else if (key === "ArrowLeft") key = "Left";
      else if (key === "ArrowRight") key = "Right";
      else if (key.length === 1) key = key.toUpperCase();

      keys.push(key);
    }

    // Format the key combination
    currentKey = keys.join("+");
  }

  // Reset all hotkeys to default
  function resetAllHotkeys() {
    if (confirm("Reset all hotkeys to default values?")) {
      localHotkeys = JSON.parse(JSON.stringify(defaultHotKeys));
      hotkeysStore.set(localHotkeys);
      editing = null;
      isRecordingKey = false;
    }
  }

  // Filter hotkeys based on search term
  $: filteredHotkeys = searchTerm
    ? Object.entries(localHotkeys).filter(
        ([key, value]) =>
          key.toLowerCase().includes(searchTerm.toLowerCase()) ||
          getReadableName(key).toLowerCase().includes(searchTerm.toLowerCase())
      )
    : Object.entries(localHotkeys);

  // Helper function to convert camelCase or snake_case to readable format
  function getReadableName(key: string): string {
    return key
      .replace(/([A-Z])/g, " $1") // Insert space before capital letters
      .replace(/_/g, " ") // Replace underscores with spaces
      .replace(/^\w/, (c) => c.toUpperCase()); // Capitalize first letter
  }
</script>

<svelte:window on:keydown={handleKeyDown} />

<div class="hotkeys-container">
  <div class="hotkeys-header">
    <h2>Keyboard Shortcuts</h2>
    <div class="search-reset">
      <div class="search-container">
        <input
          type="text"
          placeholder="Search shortcuts..."
          bind:value={searchTerm}
        />
      </div>
      <button class="reset-button" on:click={resetAllHotkeys}>
        Reset All
      </button>
    </div>
  </div>

  <div class="hotkeys-list">
    {#each filteredHotkeys as [key, value]}
      <div class="hotkey-item">
        <div class="hotkey-name">{getReadableName(key)}</div>
        <div class="hotkey-actions">
          {#if editing === key}
            <div class="hotkey-recording">
              <input
                type="text"
                readonly
                placeholder="Press keys..."
                value={currentKey}
                class="recording-input"
              />
              <div class="recording-actions">
                <button on:click={saveHotkey} class="save-button">✓</button>
                <button on:click={cancelRecording} class="cancel-button"
                  >✕</button
                >
              </div>
            </div>
          {:else}
            <div class="hotkey-shortcut">
              <span class="key-combo">{value}</span>
              <button
                on:click={() => startRecording(key as keyof HotKeys)}
                class="edit-button"
              >
                Edit
              </button>
            </div>
          {/if}
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  .hotkeys-container {
    height: 100%;
    display: flex;
    flex-direction: column;
    padding: 0 16px;
  }

  .hotkeys-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--color-border, #444);
  }

  .hotkeys-header h2 {
    margin: 0;
    font-size: 1.5rem;
  }

  .search-reset {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .search-container {
    position: relative;
  }

  .search-container input {
    padding: 8px 12px;
    border-radius: 4px;
    border: 1px solid var(--color-border, #444);
    background-color: var(--color-input-bg, #333);
    color: var(--color-text, #fff);
    width: 200px;
  }

  .reset-button {
    background-color: var(--color-warning, #b33);
    color: white;
    border: none;
    border-radius: 4px;
    padding: 8px 12px;
    cursor: pointer;
    font-weight: 500;
    transition: background-color 0.2s;
  }

  .reset-button:hover {
    background-color: var(--color-warning-hover, #c44);
  }

  .hotkeys-list {
    flex: 1;
    overflow-y: auto;
    border: 1px solid var(--color-border, #444);
    border-radius: 4px;
    background-color: var(--color-bg-secondary, #222);
  }

  .hotkey-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 16px;
    border-bottom: 1px solid var(--color-border, #333);
  }

  .hotkey-item:last-child {
    border-bottom: none;
  }

  .hotkey-name {
    flex: 1;
    font-weight: 500;
  }

  .hotkey-actions {
    display: flex;
    align-items: center;
    min-width: 200px;
    justify-content: flex-end;
  }

  .hotkey-shortcut {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .key-combo {
    padding: 4px 8px;
    background-color: var(--color-bg-accent, #333);
    border-radius: 4px;
    font-family: monospace;
    border: 1px solid var(--color-border, #555);
    min-width: 80px;
    text-align: center;
  }

  .edit-button {
    background-color: var(--color-accent, #0078d7);
    color: white;
    border: none;
    border-radius: 4px;
    padding: 4px 8px;
    cursor: pointer;
    transition: background-color 0.2s;
  }

  .edit-button:hover {
    background-color: var(--color-hover, #2089e8);
  }

  .hotkey-recording {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .recording-input {
    padding: 4px 8px;
    border: 1px solid var(--color-accent, #0078d7);
    border-radius: 4px;
    background-color: var(--color-input-bg, #333);
    color: var(--color-text, #fff);
    width: 150px;
  }

  .recording-actions {
    display: flex;
    gap: 4px;
  }

  .save-button,
  .cancel-button {
    border: none;
    border-radius: 4px;
    padding: 4px 8px;
    cursor: pointer;
    transition: background-color 0.2s;
  }

  .save-button {
    background-color: var(--color-success, #2a2);
    color: white;
  }

  .save-button:hover {
    background-color: var(--color-success-hover, #3b3);
  }

  .cancel-button {
    background-color: var(--color-warning, #b33);
    color: white;
  }

  .cancel-button:hover {
    background-color: var(--color-warning-hover, #c44);
  }
</style>
