<script lang="ts">
  import VirtualList from "svelte-virtual-list";
  import { onMount } from "svelte";
  import { Folder } from "lucide-svelte";
  import {
    recentDbStore,
    openDatabase,
    databaseStore,
  } from "../../stores/database";

  // Export props so parent can access the selection
  export let selectedDb: { url: string; name: string } | null = null;

  onMount(() => {
    // Initialize from current database store
    selectedDb =
      $databaseStore && $databaseStore.name
        ? { url: $databaseStore.url, name: $databaseStore.name }
        : null;
  });

  function toggleDb(item: { url: string; name: string }) {
    // If clicking the already selected item, deselect it
    if (
      selectedDb &&
      selectedDb.name === item.name &&
      selectedDb.url === item.url
    ) {
      selectedDb = null;
    } else {
      selectedDb = item;
    }
  }
</script>

<div class="block inner" style="height: 90%; overflow: hidden;">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <VirtualList items={Array.from($recentDbStore)} let:item>
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <div
      on:click={() => toggleDb(item)}
      class="list-item"
      class:selected-item={selectedDb && selectedDb.name === item.name}
      class:unselected-item={!selectedDb || selectedDb.name !== item.name}
    >
      {item.name}
    </div>
  </VirtualList>
</div>

<div class="header" style="margin-top: 15px;">
  <div class="button-group">
    <button
      class="cta-button small"
      on:click={async () => {
        let url = await openDatabase(false);
        if (!url) return;
        let name = url.split("/").pop();
        name = name ? name.replace(".sqlite", "") : "New Database";
        selectedDb = {
          name,
          url,
        };
      }}
    >
      <Folder size="12" />
      Add Database
    </button>
  </div>
</div>
