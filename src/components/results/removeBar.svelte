<!-- filepath: /Users/tfarrell/Documents/CODE/SMDB_Companion/src/components/ResultsSkinny.svelte -->
<script lang="ts">
  import { preferencesStore } from "../../stores/preferences";
  import { algoEnabled } from "../../stores/algorithms";
  import { TriangleAlert } from "lucide-svelte";

  $: pref = $preferencesStore;
</script>

<div class="header" style="margin-bottom: 0px; margin-top: 15px;">
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
    <select class="select-field" bind:value={$preferencesStore.erase_files}>
      {#each [{ id: "Keep", text: "Keep Files on Disk" }, { id: "Trash", text: "Move Files To Trash" }, { id: "Delete", text: "Permanently Delete Files" }] as option}
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
