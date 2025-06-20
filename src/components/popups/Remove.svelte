<script lang="ts">
  import { get } from "svelte/store";
  import { preferencesStore } from "../../stores/preferences";
  import { algoEnabled } from "../../stores/algorithms";
  import { TriangleAlert, OctagonX } from "lucide-svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { isMacOS } from "../../stores/utils";
  import { isFilesOnly } from "../../stores/menu";
  import RemoveButton from "../results/removeButton.svelte";
  import { selectedItemsStore, removeSelected } from "../../stores/results";
  import { removeRecords, removeSelectedRecords } from "../../stores/remove";

  $: processSelected = $removeSelected;

  $: isSelected = $selectedItemsStore.size > 0;

  $: pref = $preferencesStore;

  // Make archiveFolderName a reactive variable that properly awaits the async function
  let archiveFolderName = "Archive Folder";

  // Update archive folder name when preferences change
  $: if (pref) {
    updateArchiveFolderName();
  }

  async function updateArchiveFolderName() {
    archiveFolderName = await getArchiveFolderName();
  }

  async function selectArchiveFolder() {
    if (!pref) return;

    const selectedFolder = await open({
      multiple: false,
      directory: true,
      defaultPath: pref.archive_folder,
      title: "Select Archive Folder",
    });
    if (selectedFolder) {
      pref.archive_folder = selectedFolder;
      preferencesStore.set(pref);
      console.log("Selected archive folder:", selectedFolder);
      // Update the display name after selecting a new folder
      archiveFolderName = await getArchiveFolderName();
    }
  }

  async function getArchiveFolderName() {
    const p = get(preferencesStore);
    if (
      p.archive_folder === "Archive Folder" ||
      !p.archive_folder ||
      p.archive_folder === ""
    ) {
      console.warn("Archive folder is not set. Using default name.");
      return "Archive Folder";
    }

    console.log("Archive folder path:", p.archive_folder);
    if (await isMacOS()) {
      // For macOS, we can use the last part of the path
      let folder = p.archive_folder.split("/").pop();
      console.log("Archive folder name:", folder);
      return folder ? '"' + folder + '"' : "Archive Folder";
    } else {
      let folder = p.archive_folder.split("\\").pop();
      console.log("Archive folder name:", folder);
      return folder ? '"' + folder + '"' : "Archive Folder";
    }
  }

  // Initialize the archive folder name when component mounts
  import { onMount } from "svelte";
  onMount(async () => {
    archiveFolderName = await getArchiveFolderName();

    // Set processSelected based on whether there are selected records
    if ($selectedItemsStore.size > 0) {
      processSelected = true;
      removeSelected.set(true);
    } else {
      processSelected = false;
      removeSelected.set(false);
    }
  });
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div class="popup" on:click|stopPropagation>
  <div class="block">
    <!-- Header Section -->
    <div class="header">
      <h2>Process Records</h2>
      <span class="record-count">
        {#if $selectedItemsStore.size > 0}
          {$selectedItemsStore.size} selected
        {:else}
          All records
        {/if}
      </span>
    </div>

    <!-- Main Action Button -->
    <div class="action-section">
      {#if processSelected}
        <button
          class="cta-button cancel large"
          on:click={removeSelectedRecords}
        >
          <OctagonX size="20" />
          Process Selected Records
        </button>
      {:else}
        <button class="cta-button cancel large" on:click={removeRecords}>
          <OctagonX size="20" />
          Process All Records
        </button>
      {/if}
    </div>

    <!-- Settings Section -->
    <div class="settings-section">
      <!-- Settings Section -->
      <div class="settings-section">
        {#if $selectedItemsStore.size > 0}
          <div class="setting-group">
            <label class="setting-label">Process Scope:</label>
            <select
              class="select-field {isSelected ? '' : 'disabled'}"
              disabled={!isSelected}
              bind:value={processSelected}
              on:change={() => removeSelected.set(processSelected)}
            >
              {#each [{ bool: false, text: "All Records in Results" }, { bool: true, text: "Only Selected Records" }] as option}
                <option value={option.bool}>{option.text}</option>
              {/each}
            </select>
          </div>
        {/if}

        {#if !$isFilesOnly}
          <div class="setting-group">
            <label class="setting-label">Database:</label>
            <select
              class="select-field"
              bind:value={pref.safety_db}
              on:change={() => preferencesStore.set(pref)}
            >
              {#each [{ bool: true, text: "Create Safety Database" }, { bool: false, text: "Modify Current Database ❌" }] as option}
                <option value={option.bool}>{option.text}</option>
              {/each}
            </select>
            {#if pref.safety_db}
              <div class="sub-setting">
                <label class="sub-label">Database tag:</label>
                <input
                  class="input-field"
                  placeholder="thinned"
                  type="text"
                  id="new_db_tag"
                  bind:value={pref.safety_db_tag}
                  on:change={() => preferencesStore.set(pref)}
                />
              </div>
            {/if}
          </div>
        {/if}

        {#if algoEnabled("dual_mono")}
          <div class="setting-group">
            <label class="setting-label">Dual Mono Audio Files:</label>
            <select
              class="select-field"
              bind:value={pref.strip_dual_mono}
              on:change={() => preferencesStore.set(pref)}
            >
              {#each [{ id: false, text: "Leave Alone/Ignore" }, { id: true, text: "Convert Files to Mono ❌" }] as option}
                <option value={option.id}>{option.text}</option>
              {/each}
            </select>
          </div>
        {/if}

        <div class="setting-group">
          <label class="setting-label">Checked Audio Files:</label>
          <select
            class="select-field"
            bind:value={$preferencesStore.erase_files}
          >
            {#each [{ id: "Keep", text: "Keep Files on Disk" }, { id: "Archive", text: `Move to ${archiveFolderName} ⚠️` }, { id: "Trash", text: "Move Files To Trash ⚠️" }, { id: "Delete", text: "Permanently Delete Files ❌" }] as option}
              <option value={option.id}>{option.text}</option>
            {/each}
          </select>
          {#if pref.erase_files === "Archive"}
            <div class="sub-setting">
              <button class="cta-button small" on:click={selectArchiveFolder}>
                Set Archive Folder
              </button>
            </div>
          {/if}
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .popup {
    background-color: var(--primary-bg);
    border: 1px solid var(--inactive-color);
    border-radius: 12px;
    padding: 28px;
    max-width: 500px;
    width: 90vw;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
    color: var(--text-color);
    backdrop-filter: blur(10px);
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 24px;
    padding-bottom: 16px;
    border-bottom: 1px solid var(--inactive-color);
  }

  .header h2 {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 600;
    color: var(--accent-color);
  }

  .record-count {
    font-size: 0.875rem;
    color: var(--inactive-color);
    background: var(--secondary-bg, rgba(255, 255, 255, 0.1));
    padding: 4px 12px;
    border-radius: 12px;
    font-weight: 500;
  }

  .action-section {
    margin-bottom: 28px;
  }

  .cta-button.large {
    width: 100%;
    padding: 16px 24px;
    font-size: 1.1rem;
    font-weight: 600;
    gap: 12px;
    border-radius: 10px;
    transition: all 0.2s ease;
  }

  .cta-button.large:hover {
    transform: translateY(-1px);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
  }

  .settings-section {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .setting-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 16px;
    background: var(--secondary-bg, rgba(255, 255, 255, 0.05));
    border-radius: 10px;
    border: 1px solid rgba(255, 255, 255, 0.1);
  }

  .setting-label {
    font-weight: 600;
    font-size: 0.95rem;
    color: var(--text-color);
    margin: 0;
  }

  .sub-setting {
    margin-top: 12px;
    padding-top: 12px;
    border-top: 1px solid rgba(255, 255, 255, 0.1);
  }

  .sub-label {
    font-size: 0.875rem;
    color: var(--inactive-color);
    margin-bottom: 6px;
    display: block;
    font-weight: 500;
  }

  .select-field:focus {
    outline: none;
    border-color: var(--primary-color, #007acc);
    box-shadow: 0 0 0 3px rgba(0, 122, 204, 0.1);
  }

  .select-field:hover:not(:disabled) {
    border-color: var(--text-color);
  }

  .select-field.disabled {
    opacity: 0.6;
    background-color: var(--inactive-color);
    color: var(--text-color);
    cursor: not-allowed;
    border-color: var(--inactive-color);
  }

  .input-field {
    width: 100%;
    padding: 12px 16px;
    border: 1px solid var(--inactive-color);
    border-radius: 8px;
    background-color: var(--primary-bg);
    color: var(--text-color);
    font-size: 0.95rem;
    transition: all 0.2s ease;
  }

  .input-field:focus {
    outline: none;
    border-color: var(--primary-color, #007acc);
    box-shadow: 0 0 0 3px rgba(0, 122, 204, 0.1);
  }

  .input-field:hover {
    border-color: var(--text-color);
  }

  .input-field::placeholder {
    color: var(--inactive-color);
    opacity: 0.7;
  }

  .cta-button.small {
    padding: 10px 16px;
    font-size: 0.875rem;
    border-radius: 6px;
    margin-top: 8px;
  }

  /* Add subtle animations */
  .setting-group {
    transition: all 0.2s ease;
  }

  .setting-group:hover {
    background: var(--secondary-bg, rgba(255, 255, 255, 0.08));
    border-color: rgba(255, 255, 255, 0.15);
  }

  /* Responsive adjustments */
  @media (max-width: 600px) {
    .popup {
      padding: 20px;
      margin: 10px;
    }

    .header h2 {
      font-size: 1.25rem;
    }

    .cta-button.large {
      padding: 14px 20px;
      font-size: 1rem;
    }
  }
</style>
