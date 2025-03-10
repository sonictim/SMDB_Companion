<script lang="ts">
    import VirtualList from "svelte-virtual-list"; // Ensure this package is installed
    import { Square, CheckSquare, OctagonX } from "lucide-svelte";

    import { preferencesStore } from "../../store";

    // Use the store directly instead of assigning to `pref`
    let currentColumn = "";
    $: pref = $preferencesStore;
    let selectedMatches = new Set<string>();
    let waveform_match = false;

    function toggleignore_filetype() {
        preferencesStore.update((p) => ({
            ...p,
            ignore_filetype: !p.ignore_filetype,
        }));
    }
    function toggleStoreWaveforms() {
        preferencesStore.update((p) => ({
            ...p,
            store_waveforms: !p.store_waveforms,
        }));
    }
    function toggleFetchWaveforms() {
        preferencesStore.update((p) => ({
            ...p,
            fetch_waveforms: !p.fetch_waveforms,
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

<div class="grid-container">
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
                <button class="cta-button small" on:click={addColumn}
                    >Add</button
                >
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
    <div class="block" style=" height: 22vh;">
        <div class="header">
            <h2>Audio Content Search Options</h2>
        </div>
        <div class="grid">
            <span>
                <button
                    type="button"
                    class="grid item"
                    on:click={toggleStoreWaveforms}
                >
                    {#if $preferencesStore.store_waveforms}
                        <CheckSquare size={20} class="checkbox checked" />
                    {:else}
                        <Square size={20} class="checkbox" />
                    {/if}
                    <span>Store audio fingerprints in database</span>
                </button>
            </span>
            <span>
                Compare Algorithm:
                <select class="select-field" bind:value={pref.exact_waveform}>
                    {#each [{ text: "Exact Match", val: true }, { text: "Relative Match", val: false }] as { text, val }}
                        <option value={val}>{text}</option>
                    {/each}
                </select>
            </span>
            <span>
                <button
                    type="button"
                    class="grid item"
                    on:click={toggleFetchWaveforms}
                >
                    {#if $preferencesStore.fetch_waveforms}
                        <CheckSquare size={20} class="checkbox checked" />
                    {:else}
                        <Square size={20} class="checkbox" />
                    {/if}
                    <span>Fetch stored audio fingerprints from database</span>
                </button>
            </span>
            {#if pref.exact_waveform == false}
                <span style="margin-left: 70px">
                    Threshold:
                    <input
                        type="number"
                        class="input-field"
                        style="width: 100px"
                        placeholder="0.0"
                        step="0.1"
                        min="0"
                        bind:value={pref.similarity_threshold}
                    />
                </span>
            {/if}
        </div>
    </div>
</div>

<style>
    .block {
        height: calc(80vh - 200px);
    }

    .grid-container {
        height: calc(100vh - 160px);
        display: grid;
        grid-template-columns: 1fr;
        grid-template-rows: 2fr 1fr;
        gap: 20px;
    }
</style>
