<script lang="ts">
    import VirtualList from "svelte-virtual-list"; // Ensure this package is installed
    import { Square, CheckSquare, OctagonX } from "lucide-svelte";

    import { preferencesStore } from "../../store";

    // Use the store directly instead of assigning to `pref`
    let currentColumn = "";
    $: pref = $preferencesStore;
    let selectedMatches = new Set<string>();

    function toggleignore_filetype() {
        preferencesStore.update((p) => ({
            ...p,
            ignore_filetype: !p.ignore_filetype,
        }));
    }
    function toggle_all_records() {
        preferencesStore.update((p) => ({
            ...p,
            display_all_records: !p.display_all_records,
        }));
    }

    function toggleMatch(item: string) {
        if (selectedMatches.has(item)) {
            selectedMatches.delete(item);
        } else {
            selectedMatches.add(item);
        }
        selectedMatches = new Set(selectedMatches); // Ensure reactivity
    }

    function removeMatches(list: string[]) {
        list.forEach((item) => removeMatch(item));
        clearMatches();
    }

    function removeMatch(item: string) {
        preferencesStore.update((p) => ({
            ...p,
            match_criteria: p.match_criteria.filter((i) => i !== item),
        }));
    }

    function clearMatches() {
        selectedMatches.clear();
        selectedMatches = new Set(); // Ensure reactivity
    }

    function addColumn() {
        preferencesStore.update((p) => {
            if (currentColumn && !p.match_criteria.includes(currentColumn)) {
                return {
                    ...p,
                    match_criteria: [...p.match_criteria, currentColumn],
                };
            }
            return p;
        });
        currentColumn = "";
    }

    function handleColumnChange(event: Event) {
        currentColumn = (event.target as HTMLSelectElement).value;
    }

    // Get filtered columns that are not in match_criteria
    $: filteredColumns = $preferencesStore.columns.filter(
        (col) => !$preferencesStore.match_criteria.includes(col),
    );
</script>

<div class="block">
    <div class="header">
        <h2>Duplicate Match Criteria</h2>
        <button
            class="cta-button cancel"
            on:click={() => removeMatches([...selectedMatches])}
        >
            <OctagonX size="18" />
            Remove Selected
        </button>
    </div>

    <div class="header">
        <div class="button-group">
            <button class="cta-button small" on:click={addColumn}>Add</button>
            <select
                class="select-field"
                bind:value={currentColumn}
                on:change={handleColumnChange}
            >
                {#each filteredColumns as option}
                    <option value={option}>{option}</option>
                {/each}
            </select>
        </div>
        <button
            type="button"
            class="grid item"
            style="margin-left: 120px"
            on:click={toggleignore_filetype}
        >
            {#if $preferencesStore.ignore_filetype}
                <CheckSquare size={20} class="checkbox checked" />
            {:else}
                <Square size={20} class="checkbox" />
            {/if}
            <span>Ignore Filetypes (extensions)</span>
        </button>
    </div>
    <div class="block inner">
        <VirtualList
            items={Array.from($preferencesStore.match_criteria)}
            let:item
        >
            <div
                on:click={() => toggleMatch(item)}
                class="list-item"
                class:selected-item={selectedMatches.has(item)}
                class:unselected-item={!selectedMatches.has(item)}
            >
                {item}
            </div>
        </VirtualList>
    </div>
</div>

<style>
    .block {
        height: calc(100vh - 160px);
    }
</style>
