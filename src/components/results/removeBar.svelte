<script lang="ts">
  import { get } from "svelte/store";
  import { preferencesStore } from "../../stores/preferences";
  import { algoEnabled } from "../../stores/algorithms";
  import { TriangleAlert } from "lucide-svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { isMacOS } from "../../stores/utils";

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
  });
</script>

{#if pref}
  <div class="header" style="margin-top: 15px; margin-bottom: 0px">
    <span>
      <select
        class="select-field"
        bind:value={pref.safety_db}
        on:change={() => preferencesStore.set(pref)}
      >
        {#each [{ bool: true, text: "Create Safety Database" }, { bool: false, text: "Modify Current Database" }] as option}
          <option value={option.bool}>{option.text}</option>
        {/each}
      </select>
      {#if pref.safety_db}
        with tag:
        <input
          class="input-field"
          style="width: 100px"
          placeholder="thinned"
          type="text"
          id="new_db_tag"
          bind:value={pref.safety_db_tag}
          on:change={() => preferencesStore.set(pref)}
        />
      {:else}
        <TriangleAlert
          size="30"
          class="blinking"
          style="color: var(--warning-hover); margin-bottom: -10px"
        />
      {/if}
    </span>
    {#if algoEnabled("dual_mono")}
      <span>
        <select
          class="select-field"
          bind:value={pref.strip_dual_mono}
          on:change={() => preferencesStore.set(pref)}
        >
          {#each [{ id: false, text: "Preserve Dual Mono" }, { id: true, text: "Strip Dual Mono" }] as option}
            <option value={option.id}>{option.text}</option>
          {/each}
        </select>
        {#if pref.strip_dual_mono}
          <TriangleAlert
            size="30"
            class="blinking"
            style="color: var(--warning-hover); margin-bottom: -10px"
          />
        {/if}
      </span>
    {/if}
    <span>
      {#if pref.erase_files === "Archive"}
        <button class="cta-button small" on:click={selectArchiveFolder}
          >Set</button
        >
      {/if}
      <select class="select-field" bind:value={pref.erase_files}>
        {#each [{ id: "Keep", text: "Keep Files on Disk" }, { id: "Archive", text: `Move to ${archiveFolderName}` }, { id: "Trash", text: "Move Files To Trash" }, { id: "Delete", text: "Permanently Delete Files" }] as option}
          <option value={option.id}>{option.text}</option>
        {/each}
      </select>
      {#if pref.erase_files !== "Keep"}
        <TriangleAlert
          size="30"
          class={pref.erase_files == "Delete" ? "blinking" : ""}
          style="color: var(--warning-hover); margin-bottom: -10px"
        />
      {/if}
    </span>
  </div>
{/if}
