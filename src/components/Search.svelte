<script lang="ts">
    import {
        X,
        Search,
        AlertCircle,
        Loader,
        Square,
        CheckSquare,
        NotebookPenIcon,
    } from "lucide-svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { onMount, onDestroy } from "svelte";
    import { listen } from "@tauri-apps/api/event";

    // Define props
    // export let dbSize: number;
    export let activeTab: string; // This prop is now bindable
    export let isRemove: boolean;
    export let selectedDb: string | null;

    let isFinding = false;

    import { preferencesStore } from "../store";
    import {
        resultsStore,
        metadataStore,
        isSearching,
        searchProgressStore,
        initializeSearchListeners,
        resetSearchProgress,
    } from "../session-store";
    import { get } from "svelte/store";
    import { open } from "@tauri-apps/plugin-dialog";
    import { basename, extname } from "@tauri-apps/api/path";
    import type { Algorithm, Preferences } from "../store";
    import type { FileRecord } from "../session-store";

    async function getFilenameWithoutExtension(fullPath: string) {
        const name = await basename(fullPath); // Extracts filename with extension
        const ext = await extname(fullPath); // Extracts extension
        return name.replace(ext, ""); // Removes extension
    }

    async function openSqliteFile() {
        try {
            let compareDb = await open({
                multiple: false,
                directory: false,
                filters: [{ name: "SQLite Database", extensions: ["sqlite"] }],
            });
            if (Array.isArray(compareDb)) {
                compareDb = compareDb[0];
            }
            if (compareDb) {
                preferencesStore.update((prefs) => ({
                    ...prefs,
                    algorithms: prefs.algorithms.map((algo) => {
                        if (algo.id === "dbcompare") {
                            console.log(
                                "Updating dbcompare:",
                                algo,
                                "New DB:",
                                compareDb,
                            );
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

    $: results = resultsStore;
    $: metadata = metadataStore;
    let waveform_match = true;

    let pref: Preferences = get(preferencesStore);
    // let algorithms = get(algorithmsStore);

    $: isBasicEnabled =
        $preferencesStore?.algorithms?.find((a) => a.id === "basic")?.enabled ||
        false;

    function getAlgoClass(algo: { id: string }, algorithms: any[]) {
        if (
            (algo.id === "audiosuite" || algo.id === "filename") &&
            !algorithms.find((a) => a.id === "basic")?.enabled
        ) {
            return "inactive";
        }
        return "";
    }

    async function replaceMetadata() {
        isRemove = false;
        isFinding = true;
        // Your logic for replacing metadata goes here
        const metaValue = get(metadataStore);
        console.log(
            `Finding: ${metaValue.find}, Replacing: ${metaValue.replace}, Case Sensitive: ${metaValue.case_sensitive}, Column: ${metaValue.column}`,
        );

        await invoke<FileRecord[]>("find", {
            find: metaValue.find,
            column: metaValue.column,
            caseSensitive: metaValue.case_sensitive,
            pref: get(preferencesStore),
        })
            .then((result) => {
                console.log(result);
                resultsStore.set(result); // ✅ Store the results in session storage
            })
            .catch((error) => console.error(error));
        isFinding = false;
        activeTab = "results";
    }
    function toggleCaseSensitivity() {
        metadataStore.update((meta) => ({
            ...meta,
            case_sensitive: !meta.case_sensitive,
        }));
    }

    // Don't create a local snapshot - use the store directly when needed
    // Remove this line: $: prefs = $preferencesStore;

    function toggleAlgorithm(id: string) {
        preferencesStore.update((prefs) => ({
            ...prefs,
            algorithms: prefs.algorithms.map((algo) =>
                algo.id === id ? { ...algo, enabled: !algo.enabled } : algo,
            ),
        }));
    }

    function toggleSearch() {
        console.log("Toggle Search");
        $isSearching = !$isSearching;
        if ($isSearching) {
            search();
        } else {
            cancelSearch();
        }
    }

    // Update the search function
    async function search() {
        if (!$preferencesStore || !$preferencesStore.algorithms) {
            console.error("Preferences store not properly initialized");
            alert(
                "Application settings not loaded properly. Please restart the application.",
            );
            return;
        }
        let $pref = get(preferencesStore);
        let algorithms = $pref.algorithms; // Get algorithms directly from preferences

        console.log("Starting Search");
        isRemove = true;
        resultsStore.set([]);

        let algorithmState = algorithms.reduce(
            (
                acc: Record<string, boolean | number | string>,
                algo: Algorithm,
            ) => {
                acc[algo.id] = algo.enabled;
                if (algo.id === "duration") {
                    acc["min_dur"] = algo.min_dur ?? 0;
                }
                if (algo.id === "dbcompare") {
                    acc["compare_db"] = algo.db ?? "";
                }
                return acc;
            },
            {} as Record<string, boolean | number | string>,
        );

        if (!algorithmState.basic) {
            algorithmState.audiosuite = false;
        }

        await invoke<FileRecord[]>("search", {
            enabled: algorithmState,
            pref: get(preferencesStore),
        })
            .then((result) => {
                console.log("Search Results:", result);
                resultsStore.set(result);
            })
            .catch((error) => {
                $isSearching = false;
                console.error(error);
            });

        if ($isSearching) {
            $isSearching = false;
            activeTab = "results";
        }
    }

    // Setup event listener when component mounts
    onMount(async () => {
        selectedDb = await invoke<string>("get_db_name");

        // Initialize the listeners only once in the application lifecycle
        await initializeSearchListeners();

        console.log("Search component mounted, isSearching:", $isSearching);
    });

    async function cancelSearch() {
        await invoke("cancel_search")
            .then(() => {
                console.log("Search cancellation requested");
                $isSearching = false;

                // Reset progress in store
                resetSearchProgress();
            })
            .catch((error) => console.error("Error cancelling search:", error));
    }

    function getAlgorithmTooltip(id: string): string {
        const tooltips: Record<string, string> = {
            basic: "Finds duplicates by comparing Match Criteria set in Preferences.",
            filename:
                "Will attempt to remove extra letters and numbers (.1.4.21.M.wav) from the filename",
            audiosuite:
                "Searches for Protools Audiosuite tags in the filename and checks for orginal file.",
            duration:
                "Files less than the set duration will be marked for removal.",
            waveform:
                "Compares audio waveform patterns to find similar sounds.  This may take a while.",
            dbcompare: "Compares against another database to find duplicates.",
            invalidpath: "Files with invalid paths will be marked for removal.",
            filetags:
                "Filenames containting tags in this list will be marked for removal.",
        };

        return tooltips[id] || "No description available";
    }
    function checkAnyAlgorithmEnabled() {
        return $preferencesStore.algorithms.some((algo) => algo.enabled);
    }
</script>

<div class="page-columns">
    <!-- Rest of your template remains the same -->
    <div class="block" style="height: 40vh">
        <div class="header">
            <h2>Search Algorithms</h2>
            {#if selectedDb == null || selectedDb == "" || selectedDb == "Select Database" || !checkAnyAlgorithmEnabled()}
                <button class="cta-button inactive">
                    <Search size={18} />
                    <span>Search</span>
                </button>
            {:else}
                <button
                    class="cta-button {$isSearching ? 'cancel' : ''}"
                    on:click={toggleSearch}
                >
                    <div class="flex items-center gap-2">
                        {#if $isSearching}
                            <!-- <X size={18} />
                            <span>Cancel</span> -->
                        {:else}
                            <Search size={18} />
                            <span>Search</span>
                        {/if}
                    </div>
                </button>
            {/if}
        </div>
        {#if $isSearching}
            <div class="block inner">
                <span>
                    <Loader
                        size={24}
                        class="spinner ml-2"
                        style="color: var(--accent-color)"
                    />
                    {$searchProgressStore.searchMessage}
                </span>
                <div class="progress-container">
                    <div
                        class="progress-bar"
                        style="width: {$searchProgressStore.searchProgress}%"
                    ></div>
                </div>
                <span>
                    {$searchProgressStore.subsearchMessage}
                </span>
                <div class="progress-container">
                    <div
                        class="progress-bar"
                        style="width: {$searchProgressStore.subsearchProgress}%"
                    ></div>
                </div>
            </div>
        {:else}
            <div class="grid">
                {#each $preferencesStore.algorithms as algo}
                    <div
                        class="grid item {getAlgoClass(
                            algo,
                            $preferencesStore.algorithms,
                        )}"
                    >
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
                                    class="checkbox {(algo.id ===
                                        'audiosuite' ||
                                        algo.id === 'filename') &&
                                    !isBasicEnabled
                                        ? 'inactive'
                                        : 'checked'}"
                                />
                            {:else}
                                <Square size={20} class="checkbox inactive" />
                            {/if}

                            <span
                                class="tooltip-trigger {(algo.id ===
                                    'audiosuite' ||
                                    algo.id === 'filename') &&
                                !isBasicEnabled
                                    ? 'inactive'
                                    : ''}"
                            >
                                {algo.name}
                                <span class="tooltip-text"
                                    >{getAlgorithmTooltip(algo.id)}</span
                                >
                            </span>
                        </button>

                        {#if algo.id === "dbcompare"}
                            {#if algo.db !== null && algo.db !== undefined}
                                {#await getFilenameWithoutExtension(algo.db) then filename}
                                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                                    <!-- svelte-ignore a11y_no_static_element_interactions -->
                                    <span
                                        class="clickable"
                                        on:click={openSqliteFile}
                                        >{filename}</span
                                    >
                                {/await}
                            {:else}
                                <button
                                    type="button"
                                    class="small-button"
                                    on:click={openSqliteFile}
                                    >Select SQLite File</button
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
                        <!-- {#if algo.id === "waveform"}
                            <select
                                class="select-field"
                                style="width: 150px"
                                bind:value={waveform_match}
                            >
                                {#each [{ text: "Exact Match", val: true }, { text: "Relative Match", val: false }] as { text, val }}
                                    <option value={val}>{text}</option>
                                {/each}
                            </select>
                        {/if} -->
                    </div>
                {/each}
            </div>
            <span style="margin-left: 255px">
                <!-- {#if waveform_match == false}
                    <span>
                        Threshold:
                        <input
                            type="number"
                            class="input-field"
                            style="width: 100px"
                            placeholder="0.0"
                        />
                    </span>
                {/if} -->
            </span>
        {/if}
        <!-- </div> -->
    </div>

    <div class="block" style="height: 100%; margin-top: 20px">
        <div class="header">
            <h2>Metadata Replacement</h2>
            {#if selectedDb == null || selectedDb == "" || selectedDb == "Select Database" || $metadata.find == "" || $metadata.find == null}
                <button class="cta-button inactive" style="width: 125px">
                    <Search size={18} />
                    <span> Find </span>
                </button>
            {:else}
                <button
                    class="cta-button"
                    style="width: 125px"
                    on:click={replaceMetadata}
                >
                    <Search size={18} />
                    <span> Find </span>
                </button>
            {/if}
        </div>
        {#if isFinding}
            <div class="block inner">
                <span>
                    <Loader
                        size={24}
                        class="spinner ml-2"
                        style="color: var(--accent-color)"
                    />
                    {$searchProgressStore.searchMessage}
                </span>
                <div class="progress-container">
                    <div
                        class="progress-bar"
                        style="width: {$searchProgressStore.searchProgress}%"
                    ></div>
                </div>
                <span>
                    {$searchProgressStore.subsearchMessage}
                </span>
                <div class="progress-container">
                    <div
                        class="progress-bar"
                        style="width: {$searchProgressStore.subsearchProgress}%"
                    ></div>
                </div>
            </div>
        {:else}
            <div class="input-group2">
                <label for="case-sensitive">
                    <button
                        type="button"
                        class="grid item"
                        on:click={toggleCaseSensitivity}
                    >
                        {#if $metadata.case_sensitive}
                            <CheckSquare size={20} class="checkbox checked" />
                        {:else}
                            <Square size={20} class="checkbox" />
                        {/if}
                        <span>Case Sensitive</span>
                    </button>
                </label>
            </div>

            <div class="input-group">
                <label for="find-text">Find:</label>
                <input
                    type="text"
                    id="find-text"
                    bind:value={$metadata.find}
                    placeholder="Enter text to find"
                    class="input-field"
                />
            </div>

            <div class="input-group">
                <label for="replace-text">Replace:</label>
                <input
                    type="text"
                    id="replace-text"
                    bind:value={$metadata.replace}
                    placeholder="Enter text to replace"
                    class="input-field"
                />
            </div>

            <div class="input-group">
                <label for="column-select">in Column:</label>
                <select
                    id="column-select"
                    bind:value={$metadata.column}
                    class="select-field"
                >
                    {#each pref.columns as option}
                        <option value={option}>{option}</option>
                    {/each}
                </select>
            </div>
        {/if}
    </div>
</div>

<style>
    .page-columns {
        display: grid;
        grid-template-columns: repeat(1, 1fr); /* 3 equal columns */
        gap: 10px;
    }

    :global(.checkbox.checked) {
        color: var(--accent-color);
    }

    :global(.checkbox.inactive) {
        color: var(--inactive-color);
    }

    /* Tooltip styles */
    .tooltip-trigger {
        position: relative;
        display: inline-flex;
        align-items: center;
    }

    .tooltip-text {
        visibility: hidden;
        width: 220px;
        background-color: var(--tooltip-bg, #333);
        color: var(--tooltip-text, white);
        text-align: center;
        border-radius: 6px;
        padding: 8px;
        position: absolute;
        z-index: 100;
        bottom: 125%;
        left: 50%;
        transform: translateX(-50%);
        opacity: 0;
        transition: opacity 0.3s;
        font-size: 12px;
        pointer-events: none;
        box-shadow: 0 2px 5px rgba(0, 0, 0, 0.2);
    }

    .tooltip-text::after {
        content: "";
        position: absolute;
        top: 100%;
        left: 50%;
        margin-left: -5px;
        border-width: 5px;
        border-style: solid;
        border-color: var(--tooltip-bg, #333) transparent transparent
            transparent;
    }

    .tooltip-trigger:hover .tooltip-text {
        visibility: visible;
        opacity: 1;
    }
</style>
