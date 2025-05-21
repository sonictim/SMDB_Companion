<script lang="ts">
  import { Square, CheckSquare } from "lucide-svelte";

  import { preferencesStore } from "../../stores/preferences";
  import {
    getAlgorithmTooltip,
    toggleAlgorithm,
    getAlgoClass,
  } from "../../stores/algorithms";
  import { openSqliteFile } from "../../stores/database";

  import { getFilenameWithoutExtension } from "../../stores/utils";

  $: isBasicEnabled =
    $preferencesStore?.algorithms?.find((a) => a.id === "basic")?.enabled ||
    false;

  async function getCompareDb() {
    try {
      let compareDb = await openSqliteFile();
      if (compareDb) {
        preferencesStore.update((prefs) => ({
          ...prefs,
          algorithms: prefs.algorithms.map((algo) => {
            if (algo.id === "dbcompare") {
              console.log("Updating dbcompare:", algo, "New DB:", compareDb);
              return { ...algo, enabled: true, db: compareDb };
            }
            return algo;
          }),
        }));
      }
    } catch (error) {
      console.error("Error selecting file:", error);
    }
  }
</script>

{#each $preferencesStore.algorithms as algo}
  <div class="grid item {getAlgoClass(algo, $preferencesStore.algorithms)}">
    <button
      type="button"
      class="grid item"
      on:click={() => toggleAlgorithm(algo.id)}
    >
      {#if algo.id === "audiosuite" || algo.id === "filename"}
        <span style="margin-right: 20px;"></span>
      {/if}

      {#if algo.enabled}
        <CheckSquare
          size={20}
          class="checkbox {(algo.id === 'audiosuite' ||
            algo.id === 'filename') &&
          !isBasicEnabled
            ? 'inactive'
            : 'checked'}"
        />
      {:else}
        <Square size={20} class="checkbox inactive" />
      {/if}

      <span
        class="tooltip-trigger {(algo.id === 'audiosuite' ||
          algo.id === 'filename') &&
        !isBasicEnabled
          ? 'inactive'
          : ''}"
      >
        {algo.name}
        <span class="tooltip-text">{getAlgorithmTooltip(algo.id)} </span>
      </span>
    </button>

    {#if algo.id === "dbcompare"}
      {#if algo.db !== null && algo.db !== undefined}
        {#await getFilenameWithoutExtension(algo.db) then filename}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <span class="clickable" on:click={getCompareDb}>{filename}</span>
        {/await}
      {:else}
        <button type="button" class="small-button" on:click={getCompareDb}
          >Select DB</button
        >
      {/if}
    {/if}

    {#if algo.id === "duration"}
      <input
        type="number"
        min="0"
        step="0.1"
        bind:value={algo.min_dur}
        class="duration-input"
        style="width: 55px; background-color: var(--primary-bg)"
      />
      s
    {/if}
  </div>
{/each}

<style>
  :global(.checkbox.checked) {
    color: var(--accent-color);
  }

  :global(.checkbox.inactive) {
    color: var(--inactive-color);
  }
</style>
