<script lang="ts">
  import {
    hotkeysStore,
    defaultHotKeys,
    getHotkey,
    setHotkey,
    notifyHotkeyChange,
    checkForNewDefaults,
  } from "../../stores/hotkeys";
  import { onMount } from "svelte";
  import { OctagonX, RefreshCcw } from "lucide-svelte";
  import type { HashMap } from "../../stores/types";
  import { get } from "svelte/store";

  // Local state for managing hotkeys
  let localHotkeys: HashMap[] = [];
  let editing: string | null = null;
  let currentKey = "";
  let isRecordingKey = false;
  let searchTerm = "";

  // Convert array of HashMaps to flat object for easier UI manipulation
  function convertToFlatObject(
    hotkeysArray: HashMap[]
  ): Record<string, string> {
    const flatObject: Record<string, string> = {};
    hotkeysArray.forEach((item) => {
      const key = Object.keys(item)[0];
      flatObject[key] = item[key];
    });
    return flatObject;
  }

  // Convert flat object back to array of HashMaps
  function convertToHashMapArray(
    flatObject: Record<string, string>
  ): HashMap[] {
    return Object.entries(flatObject).map(([key, value]) => ({ [key]: value }));
  }

  // For UI display, we'll use a flat object representation
  let flatHotkeys: Record<string, string> = {};

  // Subscribe to hotkeys store changes
  onMount(() => {
    checkForNewDefaults();
    const unsubscribe = hotkeysStore.subscribe((value: HashMap[]) => {
      localHotkeys = JSON.parse(JSON.stringify(value)); // Deep copy of array
      flatHotkeys = convertToFlatObject(localHotkeys); // Convert to flat object for UI
    });

    return unsubscribe;
  });

  // Start recording a new hotkey
  function startRecording(key: string) {
    editing = key;
    currentKey = flatHotkeys[key];
    isRecordingKey = true;
  } // Save the current recorded key
  async function saveHotkey() {
    if (editing) {
      flatHotkeys[editing] = currentKey;
      await setHotkey(editing, currentKey); // Use the async helper function to update the store

      // No need to manually refresh now - the event system will handle it

      editing = null;
      isRecordingKey = false;
    }
  }

  // Cancel current recording
  function cancelRecording() {
    if (editing) {
      currentKey = flatHotkeys[editing];
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
  async function resetAllHotkeys() {
    if (confirm("Reset all hotkeys to default values?")) {
      // Reset the store to default values
      hotkeysStore.set(JSON.parse(JSON.stringify(defaultHotKeys)));

      // Update our local state
      localHotkeys = JSON.parse(JSON.stringify(defaultHotKeys));
      flatHotkeys = convertToFlatObject(localHotkeys);

      // Notify about the change to trigger menu refresh
      await notifyHotkeyChange();

      editing = null;
      isRecordingKey = false;
    }
  }

  // Filter hotkeys based on search term
  $: filteredHotkeys = searchTerm
    ? Object.entries(flatHotkeys).filter(
        ([key, value]) =>
          key.toLowerCase().includes(searchTerm.toLowerCase()) ||
          getReadableName(key).toLowerCase().includes(searchTerm.toLowerCase())
      )
    : Object.entries(flatHotkeys);

  // Helper function to convert camelCase or snake_case to readable format
  function getReadableName(key: string): string {
    return key
      .replace(/([A-Z])/g, " $1") // Insert space before capital letters
      .replace(/_/g, " ") // Replace underscores with spaces
      .replace(/^\w/, (c) => c.toUpperCase()); // Capitalize first letter
  }
</script>

<svelte:window on:keydown={handleKeyDown} />

<div class="block">
  <div class="header">
    <h2>Keyboard Shortcuts</h2>
    <button class="cta-button cancel" on:click={resetAllHotkeys}>
      <RefreshCcw size={18} />
      Reset All
    </button>
  </div>

  <div class="bar">
    <input
      type="text"
      placeholder="Search shortcuts..."
      bind:value={searchTerm}
      class="input-field"
    />
  </div>

  <div class="block inner">
    {#each filteredHotkeys as [key, value]}
      <div class="item-container">
        <div class="list-item">
          <div class="item-content">
            <span>{getReadableName(key)}</span>

            {#if editing === key}
              <div class="key-edit-container editing">
                <input
                  type="text"
                  readonly
                  placeholder="Press keys..."
                  value={currentKey}
                  class="input-field"
                />
                <button on:click={saveHotkey} class="cta-button small">✓</button
                >
                <button
                  on:click={cancelRecording}
                  class="cta-button small cancel">✕</button
                >
              </div>
            {:else}
              <div class="key-display">
                <span class="key-value">{value}</span>
                <button
                  on:click={() => startRecording(key)}
                  class="cta-button small"
                >
                  Edit
                </button>
              </div>
            {/if}
          </div>
        </div>
      </div>
    {/each}
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

  .item-content {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-grow: 1;
    cursor: pointer;
  }

  .key-edit-container {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: 16px;
  }

  .editing {
    background-color: var(--accent-color);
    padding: 4px 8px;
    border-radius: 4px;
  }

  .key-display {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: 16px;
  }

  .key-value {
    padding: 4px 8px;
    background-color: var(--secondary-bg);
    border-radius: 4px;
    font-family: monospace;
    min-width: 80px;
    text-align: center;
    border: 1px solid var(--topbar-color);
  }

  .block {
    height: calc(100vh - 160px);
  }
</style>
