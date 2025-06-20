<script lang="ts">
  import VirtualList from "svelte-virtual-list";
  import { OctagonX, Folder } from "lucide-svelte";
  import {
    searchFoldersStore,
    addSearchFolders,
    removeSearchFolders,
  } from "../../stores/search";

  // Export props so parent can access the selection
  export let selectedFolders: Set<string> = new Set<string>();

  function toggleFolder(item: string) {
    if (selectedFolders.has(item)) {
      selectedFolders.delete(item);
    } else {
      selectedFolders.add(item);
    }
    selectedFolders = new Set(selectedFolders); // Ensure reactivity
  }
</script>

<div class="block inner" style="height: 90%; overflow: hidden;">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <VirtualList items={Array.from($searchFoldersStore)} let:item>
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <div
      on:click={() => toggleFolder(item)}
      class="list-item"
      class:selected-item={selectedFolders.has(item)}
      class:unselected-item={!selectedFolders.has(item)}
    >
      {item}
    </div>
  </VirtualList>
</div>

<div class="header" style="margin-top: 15px;">
  <div class="button-group">
    <button class="cta-button small" on:click={addSearchFolders}>
      <Folder size="12" />
      Add Folders to Search
    </button>
    <button
      class="cta-button cancel small"
      on:click={() => removeSearchFolders([...selectedFolders])}
    >
      <OctagonX size="12" />
      Remove Selected
    </button>
  </div>
</div>
