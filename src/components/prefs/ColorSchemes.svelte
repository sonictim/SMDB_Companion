<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import {
    Save,
    Upload,
    Download,
    Trash2,
    Copy,
    RotateCcw,
  } from "lucide-svelte";
  import {
    colorSchemesStore,
    getColorSchemes,
    getColorScheme,
    saveColorScheme,
    loadColorScheme,
    deleteColorScheme,
    duplicateColorScheme,
    getCustomColorSchemes,
    getDefaultColorSchemes,
    colorSchemeExists,
    exportColorScheme,
    importColorScheme,
    resetColorSchemes,
    getColorSchemeOptions,
  } from "../../stores/colorSchemes";
  import { defaultSchemeNames } from "../../stores/colors";
  import type { ColorScheme } from "../../stores/types";

  const dispatch = createEventDispatcher();

  // Reactive store subscription
  $: schemes = $colorSchemesStore;

  // Make these reactive to the store changes
  $: {
    defaultSchemes = schemes.filter((scheme) =>
      defaultSchemeNames.includes(scheme.name)
    );

    customSchemes = schemes.filter(
      (scheme) => !defaultSchemeNames.includes(scheme.name)
    );
  }

  // Component state
  let selectedScheme = "";
  let newSchemeName = "";
  let showSaveDialog = false;
  let showImportDialog = false;
  let importJson = "";
  let saveError = "";
  let importError = "";
  let customSchemes: ColorScheme[] = [];
  let defaultSchemes: ColorScheme[] = [];
  let saveInputElement: HTMLInputElement;

  // Auto-focus the input when save dialog opens
  $: if (showSaveDialog && saveInputElement) {
    setTimeout(() => saveInputElement.focus(), 10);
  }

  // Handle saving current colors as new scheme
  async function handleSaveScheme() {
    saveError = "";

    if (!newSchemeName.trim()) {
      saveError = "Please enter a scheme name";
      return;
    }

    if (colorSchemeExists(newSchemeName)) {
      saveError = "A scheme with this name already exists";
      return;
    }

    try {
      const savedName = await saveColorScheme(newSchemeName.trim());
      if (savedName) {
        console.log(`Color scheme saved: ${savedName}`);
        newSchemeName = "";
        showSaveDialog = false;
        selectedScheme = savedName; // Select the newly saved scheme
        dispatch("scheme-saved", { name: savedName });
      } else {
        saveError = "Failed to save color scheme";
      }
    } catch (error) {
      console.error("Error saving scheme:", error);
      saveError = "Error saving color scheme";
    }
  }

  // Handle loading a color scheme
  async function handleLoadScheme() {
    if (!selectedScheme) return;

    try {
      const success = await loadColorScheme(selectedScheme);
      if (success) {
        console.log(`Color scheme loaded: ${selectedScheme}`);
        dispatch("scheme-loaded", { name: selectedScheme });
      } else {
        console.error("Failed to load color scheme");
      }
    } catch (error) {
      console.error("Error loading scheme:", error);
    }
  }

  // Handle deleting a custom scheme
  async function handleDeleteScheme() {
    if (!selectedScheme) return;

    // Confirm deletion
    if (
      !confirm(
        `Are you sure you want to delete the color scheme "${selectedScheme}"?`
      )
    ) {
      return;
    }

    try {
      const success = await deleteColorScheme(selectedScheme);
      if (success) {
        console.log(`Color scheme deleted: ${selectedScheme}`);
        const deletedName = selectedScheme;
        selectedScheme = ""; // Clear selection
        dispatch("scheme-deleted", { name: deletedName });
      } else {
        console.error("Failed to delete color scheme");
      }
    } catch (error) {
      console.error("Error deleting scheme:", error);
    }
  }

  // Handle duplicating a scheme
  async function handleDuplicateScheme() {
    if (!selectedScheme) return;

    const newName = prompt(
      `Enter a name for the duplicate of "${selectedScheme}":`
    );
    if (!newName) return;

    try {
      const duplicatedName = await duplicateColorScheme(
        selectedScheme,
        newName
      );
      if (duplicatedName) {
        console.log(`Color scheme duplicated: ${duplicatedName}`);
        selectedScheme = duplicatedName;
        dispatch("scheme-duplicated", {
          originalName: selectedScheme,
          newName: duplicatedName,
        });
      } else {
        alert(
          "Failed to duplicate color scheme. The name might already exist."
        );
      }
    } catch (error) {
      console.error("Error duplicating scheme:", error);
      alert("Error duplicating color scheme");
    }
  }

  // Handle exporting a scheme
  function handleExportScheme() {
    if (!selectedScheme) return;

    const exported = exportColorScheme(selectedScheme);
    if (exported) {
      // Create a downloadable file
      const blob = new Blob([exported], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `${selectedScheme.replace(/[^a-z0-9]/gi, "_").toLowerCase()}_color_scheme.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } else {
      alert("Failed to export color scheme");
    }
  }

  // Handle importing a scheme
  async function handleImportScheme() {
    importError = "";

    if (!importJson.trim()) {
      importError = "Please paste the JSON data";
      return;
    }

    try {
      const importedName = await importColorScheme(importJson.trim());
      if (importedName) {
        console.log(`Color scheme imported: ${importedName}`);
        importJson = "";
        showImportDialog = false;
        selectedScheme = importedName;
        dispatch("scheme-imported", { name: importedName });
      } else {
        importError = "Failed to import color scheme. Check the JSON format.";
      }
    } catch (error) {
      console.error("Error importing scheme:", error);
      importError = "Invalid JSON or missing required properties";
    }
  }

  // Handle file input for import
  function handleFileImport(event: Event) {
    const target = event.target as HTMLInputElement;
    const file = target.files?.[0];
    if (file) {
      const reader = new FileReader();
      reader.onload = (e) => {
        importJson = (e.target?.result as string) || "";
      };
      reader.readAsText(file);
    }
  }

  // Handle resetting to defaults
  async function handleResetSchemes() {
    if (
      !confirm(
        "Are you sure you want to reset all color schemes to defaults? This will remove all custom schemes."
      )
    ) {
      return;
    }

    try {
      await resetColorSchemes();
      selectedScheme = "";
      dispatch("schemes-reset");
    } catch (error) {
      console.error("Error resetting schemes:", error);
    }
  }
</script>

<div class="color-schemes-manager">
  <!-- Simple one-line layout -->
  <div class="schemes-selector-simple">
    <select
      class="select-field"
      bind:value={selectedScheme}
      on:change={handleLoadScheme}
    >
      <option value="">Select a color scheme...</option>
      {#if defaultSchemes.length > 0}
        <optgroup label="Default Schemes">
          {#each defaultSchemes as scheme}
            <option value={scheme.name}>{scheme.name}</option>
          {/each}
        </optgroup>
      {/if}
      {#if customSchemes.length > 0}
        <optgroup label="Custom Schemes">
          {#each customSchemes as scheme}
            <option value={scheme.name}>{scheme.name}</option>
          {/each}
        </optgroup>
      {/if}
    </select>

    <button
      class="cta-button small"
      on:click={() => (showSaveDialog = true)}
      title="Save current colors as new scheme"
    >
      <Save size={16} />
      Save Current
    </button>

    <button
      class="cta-button small cancel"
      disabled={!selectedScheme ||
        defaultSchemes.some((s) => s.name === selectedScheme)}
      on:click={handleDeleteScheme}
      title="Delete custom scheme"
    >
      <Trash2 size={14} />
    </button>
  </div>

  <!-- Hidden advanced features for future use -->
  <!-- 
  <div class="schemes-header" style="display: none;">
    <h3>Color Schemes</h3>
    <div class="schemes-actions">
      <button
        class="cta-button small"
        on:click={() => (showImportDialog = true)}
        title="Import color scheme from file"
      >
        <Upload size={16} />
        Import
      </button>
      <button
        class="cta-button small cancel"
        on:click={handleResetSchemes}
        title="Reset to default schemes"
      >
        <RotateCcw size={16} />
        Reset
      </button>
    </div>
  </div>

  <div class="schemes-selector" style="display: none;">
    <div class="selector-row">
      <div class="scheme-actions">
        <button
          class="cta-button small"
          disabled={!selectedScheme}
          on:click={handleExportScheme}
          title="Export scheme to file"
        >
          <Download size={14} />
        </button>
        <button
          class="cta-button small"
          disabled={!selectedScheme}
          on:click={handleDuplicateScheme}
          title="Duplicate scheme"
        >
          <Copy size={14} />
        </button>
      </div>
    </div>
  </div>
  -->
</div>

<!-- Save Dialog -->
{#if showSaveDialog}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <div class="modal-overlay" on:click={() => (showSaveDialog = false)}>
    <div class="modal" on:click|stopPropagation>
      <h3>Save Color Scheme</h3>
      <div class="modal-content">
        <input
          bind:this={saveInputElement}
          type="text"
          class="input-field"
          placeholder="Enter scheme name..."
          bind:value={newSchemeName}
          on:keydown={(e) => e.key === "Enter" && handleSaveScheme()}
        />
        {#if saveError}
          <div class="error-message">{saveError}</div>
        {/if}
      </div>
      <div class="modal-actions">
        <button
          class="cta-button cancel"
          on:click={() => (showSaveDialog = false)}
        >
          Cancel
        </button>
        <button class="cta-button" on:click={handleSaveScheme}> Save </button>
      </div>
    </div>
  </div>
{/if}

<!-- Import Dialog -->
{#if showImportDialog}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <div class="modal-overlay" on:click={() => (showImportDialog = false)}>
    <div class="modal large" on:click|stopPropagation>
      <h3>Import Color Scheme</h3>
      <div class="modal-content">
        <div class="import-options">
          <label for="file-import" class="file-import-label">
            <Upload size={16} />
            Choose JSON file
          </label>
          <input
            id="file-import"
            type="file"
            accept=".json"
            style="display: none;"
            on:change={handleFileImport}
          />
          <span>or paste JSON below:</span>
        </div>
        <textarea
          class="import-textarea"
          placeholder="Paste color scheme JSON data here..."
          bind:value={importJson}
        ></textarea>
        {#if importError}
          <div class="error-message">{importError}</div>
        {/if}
      </div>
      <div class="modal-actions">
        <button
          class="cta-button cancel"
          on:click={() => (showImportDialog = false)}
        >
          Cancel
        </button>
        <button class="cta-button" on:click={handleImportScheme}>
          Import
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .color-schemes-manager {
    margin-top: 24px;
    padding: 12px 24px;
    background-color: var(--secondary-bg);
    border-radius: 8px;
    width: 80vw;
    align-self: center;
  }

  .schemes-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 16px;
  }

  .schemes-header h3 {
    margin: 0;
    color: var(--text-color);
    font-size: var(--font-size-lg);
  }

  .schemes-actions {
    display: flex;
    gap: 8px;
  }

  .schemes-selector {
    width: 100%;
  }

  .schemes-selector-simple {
    display: flex;
    gap: 8px;
    align-items: center;
    width: 100%;
  }

  .schemes-selector-simple .select-field {
    flex: 1;
  }

  .selector-row {
    display: flex;
    gap: 8px;
    align-items: center;
    width: 100%;
  }

  .selector-row .select-field {
    flex: 1;
  }

  .scheme-actions {
    display: flex;
    gap: 4px;
  }

  .scheme-actions button {
    min-width: 36px;
    height: 32px;
    padding: 4px 6px;
  }

  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: rgba(0, 0, 0, 0.5);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 1000;
  }

  .modal {
    background-color: var(--primary-bg);
    border: 1px solid var(--inactive-color);
    border-radius: 8px;
    padding: 24px;
    max-width: 400px;
    width: 90vw;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  }

  .modal.large {
    max-width: 600px;
  }

  .modal h3 {
    margin: 0 0 16px 0;
    color: var(--text-color);
  }

  .modal-content {
    margin-bottom: 20px;
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .import-options {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;
  }

  .file-import-label {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background-color: var(--secondary-bg);
    border: 1px solid var(--inactive-color);
    border-radius: 4px;
    cursor: pointer;
    color: var(--text-color);
    font-size: var(--font-size-sm);
    transition: background-color 0.2s;
  }

  .file-import-label:hover {
    background-color: var(--hover-color);
  }

  .import-textarea {
    width: 100%;
    height: 200px;
    background-color: var(--secondary-bg);
    border: 1px solid var(--inactive-color);
    border-radius: 4px;
    padding: 8px;
    color: var(--text-color);
    font-family: monospace;
    font-size: var(--font-size-sm);
    resize: vertical;
  }

  .import-textarea:focus {
    outline: none;
    border-color: var(--accent-color);
  }

  .error-message {
    color: var(--warning-color);
    font-size: var(--font-size-sm);
    margin-top: 8px;
    padding: 6px 8px;
    background-color: rgba(185, 28, 28, 0.1);
    border-radius: 4px;
    border: 1px solid var(--warning-color);
  }

  /* Responsive adjustments */
  @media (max-width: 768px) {
    .schemes-header {
      flex-direction: column;
      align-items: flex-start;
      gap: 12px;
    }

    .schemes-actions {
      flex-wrap: wrap;
    }

    .selector-row {
      flex-direction: column;
      align-items: stretch;
      gap: 12px;
    }

    .scheme-actions {
      justify-content: center;
    }
  }
</style>
