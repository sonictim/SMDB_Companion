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
  import { getIsMac } from "../../stores/utils";

  // Local state for managing hotkeys
  let localHotkeys: HashMap[] = [];
  let editing: string | null = null;
  let currentKey = "";
  let isRecordingKey = false;
  let searchTerm = "";

  // Platform detection flag - initialize to null to help with debugging
  let isMac: boolean | null = null;

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

  // Subscribe to hotkeys store changes and detect platform
  onMount(async () => {
    checkForNewDefaults();
    const unsubscribe = hotkeysStore.subscribe((value: HashMap[]) => {
      localHotkeys = JSON.parse(JSON.stringify(value)); // Deep copy of array
      flatHotkeys = convertToFlatObject(localHotkeys); // Convert to flat object for UI
    });

    // Detect platform using our helper function
    try {
      // Set isMac once and never change it again during the component's lifecycle
      isMac = await getIsMac();
      console.log("Platform detection result: isMac =", isMac);

      // Force a refresh of the component
      flatHotkeys = { ...flatHotkeys };
    } catch (err) {
      console.error("Failed to detect platform:", err);
      // Default to false if we can't detect
      isMac = false;
    }

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

  // Special method for mouse action key recording
  function handleMouseModifierEdit(action: string) {
    editing = action;
    let baseAction = action.includes("lasso") ? "Drag" : "Click";

    // Extract current modifier from the full hotkey value
    let currentModifier = flatHotkeys[action].replace(`+${baseAction}`, "");
    if (currentModifier === baseAction) currentModifier = "None"; // Handle case where there's no modifier

    currentKey = flatHotkeys[action];
    isRecordingKey = true;
  }

  // Handle key press during mouse modifier recording
  function handleKeyDownForMouseModifiers(event: KeyboardEvent) {
    if (!isRecordingKey || !editing || !isMouseAction(editing)) return;

    event.preventDefault();
    event.stopPropagation();

    const baseAction = editing.includes("lasso") ? "Drag" : "Click";
    let keys: string[] = [];

    // Add modifiers
    if (event.metaKey || event.ctrlKey) keys.push("CmdOrCtrl");
    if (event.shiftKey) keys.push("Shift");
    if (event.altKey) keys.push("Alt");

    // If no modifiers were pressed, set to None/base action
    if (keys.length === 0) {
      currentKey = baseAction;
    } else {
      // Format the key combination
      currentKey = `${keys.join("+")}+${baseAction}`;
    }
  }

  // Reset all hotkeys to default
  async function resetAllHotkeys() {
    if (await confirm("Reset all hotkeys to default values?")) {
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

  // Helper function to check if a key is a mouse action
  function isMouseAction(key: string): boolean {
    const mouseActions = [
      "toggleRowSelect",
      "toggleSelectAll",
      "selectRange",
      "unselectRange",
      "lassoSelect",
      "lassoUnselect",
    ];
    return mouseActions.includes(key);
  }

  // Get a more user-friendly description for mouse actions
  function getMouseActionDescription(key: string): string {
    const descriptions: Record<string, string> = {
      toggleRowSelect: "Select/Deselect Single Row",
      toggleSelectAll: "Toggle Select All Rows",
      selectRange: "Select Range of Rows",
      unselectRange: "Unselect Range of Rows",
      lassoSelect: "Drag to Select Multiple Rows",
      lassoUnselect: "Drag to Unselect Multiple Rows",
    };
    return descriptions[key] || getReadableName(key);
  }

  // Filter hotkeys based on search term
  $: filteredHotkeys = searchTerm
    ? Object.entries(flatHotkeys).filter(
        ([key, value]) =>
          key.toLowerCase().includes(searchTerm.toLowerCase()) ||
          getReadableName(key)
            .toLowerCase()
            .includes(searchTerm.toLowerCase()) ||
          (isMouseAction(key) &&
            getMouseActionDescription(key)
              .toLowerCase()
              .includes(searchTerm.toLowerCase()))
      )
    : Object.entries(flatHotkeys);

  // Separate keyboard shortcuts from mouse actions for display
  $: keyboardHotkeys = filteredHotkeys.filter(([key]) => !isMouseAction(key));
  $: mouseHotkeys = filteredHotkeys.filter(([key]) => isMouseAction(key));

  // React to platform changes and force update of the UI
  $: {
    if (isMac !== null) {
      console.log("isMac changed in reactive statement:", isMac);
      // Force a refresh of the component when isMac changes
      flatHotkeys = { ...flatHotkeys };
    }
  }

  // Helper function to convert camelCase or snake_case to readable format
  function getReadableName(key: string): string {
    return key
      .replace(/([A-Z])/g, " $1") // Insert space before capital letters
      .replace(/_/g, " ") // Replace underscores with spaces
      .replace(/^\w/, (c) => c.toUpperCase()); // Capitalize first letter
  }

  // Helper function to extract the modifier part from a mouse hotkey value
  function getMouseModifier(value: string): string {
    // Check if it's just the base action with no modifiers
    if (value === "Click" || value === "Drag") {
      return "None";
    }

    // Otherwise, extract the modifier part
    return value.replace("+Click", "").replace("+Drag", "");
  }

  // Helper function to get the mouse action type (Click or Drag)
  function getMouseActionType(key: string): string {
    return key.includes("lasso") ? "Drag" : "Click";
  }

  // Format key string for display (replaces CmdOrCtrl with platform-specific key)
  function formatKeyForDisplay(keyStr: string): string {
    console.log("formatKeyForDisplay called with: ", keyStr, "isMac=", isMac);
    if (keyStr.includes("CmdOrCtrl")) {
      const result = keyStr.replace("CmdOrCtrl", isMac ? "Cmd" : "Ctrl");
      console.log(
        "Replacing CmdOrCtrl with: ",
        isMac ? "Cmd" : "Ctrl",
        "Result:",
        result
      );
      return result;
    }
    return keyStr;
  }

  // Split a key combination into individual parts for box display
  function splitKeyCombination(keyStr: string): string[] {
    // First, format CmdOrCtrl to platform-specific name
    const formattedKey = formatKeyForDisplay(keyStr);

    // Split by '+' and return the array of key parts
    return formattedKey.split("+");
  }

  // Helper function to split mouse modifier into parts for box display
  function splitMouseModifier(value: string): string[] {
    const baseAction = value.includes("Drag") ? "Drag" : "Click";
    const modifier = getMouseModifier(value);

    if (modifier === "None") {
      return [baseAction];
    } else {
      // Format the modifier part and return as array with action
      const formattedModifier = formatKeyForDisplay(modifier);
      return formattedModifier.split("+").concat([baseAction]);
    }
  }
</script>

<svelte:window
  on:keydown={(e) => {
    handleKeyDown(e);
    handleKeyDownForMouseModifiers(e);
  }}
/>

<div class="block">
  <div class="header">
    <h2>Keyboard Shortcuts & Mouse Actions</h2>
    <button class="cta-button cancel" on:click={resetAllHotkeys}>
      <RefreshCcw size={18} />
      Reset All
    </button>
  </div>

  <div class="bar" style="margin-bottom: 1px;">
    <input
      type="text"
      placeholder="Search shortcuts..."
      bind:value={searchTerm}
      class="input-field"
    />
  </div>

  <!-- Mouse Actions Section -->
  <div class="section-header mouse-header">
    <h3>Mouse Selection Actions</h3>
  </div>
  <div class="block inner mouse-section">
    {#each mouseHotkeys as [key, value]}
      <div class="item-container mouse-action">
        <div class="list-item">
          <div class="item-content">
            <span title={key}>{getMouseActionDescription(key)}</span>

            {#if editing === key}
              <div class="key-edit-container editing">
                <input
                  type="text"
                  readonly
                  placeholder="Press modifier keys..."
                  value={formatKeyForDisplay(currentKey)}
                  class="input-field edit-input"
                />
                <button
                  on:click={() => {
                    const baseAction =
                      editing && editing.includes("lasso") ? "Drag" : "Click";
                    currentKey = baseAction;
                  }}
                  class="cta-button small"
                  title="Clear modifier"
                >
                  Clear
                </button>
                <button on:click={saveHotkey} class="cta-button small">✓</button
                >
                <button
                  on:click={cancelRecording}
                  class="cta-button small cancel">✕</button
                >
              </div>
            {:else}
              <div class="key-display">
                <div class="key-boxes-container">
                  {#each splitMouseModifier(value) as keyPart}
                    <span class="key-box">{keyPart}</span>
                  {/each}
                </div>
                <button
                  on:click={() => handleMouseModifierEdit(key)}
                  class="cta-button small edit-button"
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

  <!-- Keyboard Shortcuts Section -->
  <div class="section-header keyboard-header">
    <h3>Keyboard Shortcuts</h3>
  </div>
  <div class="block inner keyboard-section">
    {#each keyboardHotkeys as [key, value]}
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
                  value={formatKeyForDisplay(currentKey)}
                  class="input-field edit-input"
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
                <div class="key-boxes-container">
                  {#each splitKeyCombination(value) as keyPart}
                    <span class="key-box">{keyPart}</span>
                  {/each}
                </div>
                <button
                  on:click={() => startRecording(key)}
                  class="cta-button small edit-button"
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
    /* padding: 8px; */
    border-radius: 4px;
    background-color: var(--primary-bg-color);
    transition: all 0.2s;
    font-size: 14px;
    height: 25px;
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
    display: flex;
    align-items: center;
    padding: 2px 0;
    border-radius: 4px;
  }

  .key-display {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: 16px;
  }

  .key-box {
    padding: 3px 6px;
    background-color: var(--secondary-bg);
    border-radius: 4px;
    font-family: monospace;
    min-width: 20px;
    text-align: center;
    border: 1px solid var(--topbar-color);
    font-size: 14px;
    display: inline-block;
    margin-right: 2px;
  }

  .key-boxes-container {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .mouse-action-type {
    font-family: monospace;
    font-weight: normal;
  }

  .edit-button {
    font-size: 14px;
    padding: 2px 8px;
  }

  .edit-input {
    font-size: 14px;
    background-color: var(--secondary-bg);
    border: 1px solid var(--topbar-color);
    border-radius: 4px;
    padding: 3px 6px;
    min-width: 120px;
    font-family: monospace;
    text-align: center;
  }

  .clear-button {
    font-size: 12px;
    padding: 2px 6px;
    background-color: var(--secondary-bg);
    color: var(--text-color);
    border: 1px solid var(--topbar-color);
  }

  .block {
    height: calc(100vh - 160px);
  }

  .section-header {
    padding: 10px 0 5px 0;
    margin: 10px 0 5px 0;
    border-bottom: 1px solid var(--inactive-color);
  }

  .section-header h3 {
    font-size: 16px;
    color: var(--accent-color);
    margin: 0;
  }

  .mouse-action .item-content span {
    font-weight: 500;
    font-size: 14px;
  }

  /* Adjust section heights */
  .mouse-action {
    height: 30px; /* Reduced height for mouse actions */
  }

  .mouse-action .list-item {
    height: 30px; /* Make mouse action rows smaller */
  }

  .mouse-section {
    max-height: 35%; /* Smaller section for mouse actions */
    overflow-y: auto;
  }

  .keyboard-section {
    max-height: 65%; /* Larger section for keyboard shortcuts */
    overflow-y: auto;
  }
</style>
