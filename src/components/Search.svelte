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
    export let dbSize: number;
    export let activeTab: string; // This prop is now bindable
    export let isRemove: boolean;
    export let selectedDb: string | null;

    let isSearching = false;

    import { algorithmsStore, preferencesStore } from "../store";
    import { resultsStore, metadataStore } from "../session-store";
    import { get } from "svelte/store";
    import { open } from "@tauri-apps/plugin-dialog";
    import { basename, extname } from "@tauri-apps/api/path";
    import type { Algorithm, Preferences, FileRecord } from "../store";

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
                algorithmsStore.update((algorithms) => {
                    return algorithms.map((algo) => {
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
                    });
                });
                let updatedAlgorithms = get(algorithmsStore);
            }
        } catch (error) {
            console.error("Error selecting file:", error);
        }
    }

    $: results = resultsStore;
    $: metadata = metadataStore;

    let pref: Preferences = get(preferencesStore);
    let algorithms = get(algorithmsStore);

    $: isBasicEnabled =
        $algorithmsStore.find((a) => a.id === "basic")?.enabled || false;

    function getAlgoClass(algo: { id: string }, algorithms: any[]) {
        if (
            (algo.id === "audiosuite" || algo.id === "filename") && // Add filename check here
            !algorithms.find((a) => a.id === "basic")?.enabled
        ) {
            return "inactive";
        }
        return "";
    }

    async function replaceMetadata() {
        isRemove = false;
        // Your logic for replacing metadata goes here
        const metaValue = get(metadataStore);
        console.log(
            `Finding: ${metaValue.find}, Replacing: ${metaValue.replace}, Case Sensitive: ${metaValue.case_sensitive}, Column: ${metaValue.column}`,
        );
        await invoke<string>("find", {
            find: metaValue.find,
            column: metaValue.column,
            caseSensitive: metaValue.case_sensitive,
            pref: get(preferencesStore),
        })
            .then((result) => {
                console.log(result);
            })
            .catch((error) => console.error(error));

        activeTab = "results";
    }
    function toggleCaseSensitivity() {
        metadataStore.update((meta) => ({
            ...meta,
            case_sensitive: !meta.case_sensitive,
        }));
    }

    function checkDB(): boolean {
        return selectedDb === "Select Database";
    }

    // Don't create a local snapshot - use the store directly when needed
    // Remove this line: $: prefs = $preferencesStore;

    function toggleAlgorithm(id: string) {
        algorithmsStore.update((algorithms) =>
            algorithms.map((algo) =>
                algo.id === id ? { ...algo, enabled: !algo.enabled } : algo,
            ),
        );
    }

    function toggleSearch() {
        console.log("Toggle Search");
        isSearching = !isSearching;
        if (isSearching) {
            search();
        }
    }

    // Update the search function
    async function search() {
        let $pref = get(preferencesStore);
        let algorithms = get(algorithmsStore); // Ensure fresh values

        // console.log($pref.preservation_order); // Correctly access the current value
        // Always access the current store value by using get() or $ prefix
        // console.log($preferencesStore.match_criteria); // Correctly access the current value

        console.log("Starting Search");
        // console.log(get(preferencesStore)); // Correctly access the current value
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

        // Only affect audiosuite when basic is disabled
        if (!algorithmState.basic) {
            algorithmState.audiosuite = false;
            // Filename is now completely independent
        }

        await invoke<FileRecord[]>("search", {
            enabled: algorithmState,
            pref: get(preferencesStore),
        })
            .then((result) => {
                console.log("Search Results:", result);
                resultsStore.set(result); // ✅ Store the results in session storage
            })
            .catch((error) => console.error(error));

        isSearching = false;
        activeTab = "results"; // Ensure this updates properly
    }

    // Add these variables for search status
    let searchProgress = 0;
    let searchMessage = "Initializing...";
    let searchStage = "";
    let unlistenFn: () => void;
    let subsearchProgress = 0;
    let subsearchMessage = "Requesting Records...";
    let subsearchStage = "";
    let unlistenFn2: () => void;

    // Setup event listener when component mounts
    onMount(async () => {
        unlistenFn = await listen<{
            progress: number;
            message: string;
            stage: string;
        }>("search-status", (event) => {
            const status = event.payload;
            searchProgress = status.progress;
            searchMessage = status.message;
            searchStage = status.stage;
            console.log(
                `Search status: ${status.stage} - ${status.progress}% - ${status.message}`,
            );
        });
        unlistenFn2 = await listen<{
            progress: number;
            message: string;
            stage: string;
        }>("search-sub-status", (event) => {
            const status = event.payload;
            subsearchProgress = status.progress;
            subsearchMessage = status.message;
            subsearchStage = status.stage;
            console.log(
                `Search status: ${status.stage} - ${status.progress}% - ${status.message}`,
            );
        });
    });

    // Cleanup event listener when component unmounts
    onDestroy(() => {
        if (unlistenFn) unlistenFn();
        if (unlistenFn2) unlistenFn2();
    });

    async function cancelSearch() {
        await invoke("cancel_search")
            .then(() => {
                console.log("Search cancellation requested");
                isSearching = false;
                searchProgress = 0;
                searchMessage = "Search cancelled";
                subsearchProgress = 0;
                subsearchMessage = "Search cancelled";
            })
            .catch((error) => console.error("Error cancelling search:", error));
    }
</script>

<div class="page-columns">
    <!-- Rest of your template remains the same -->
    <div class="block" style="height: 40vh">
        <div class="header">
            <h2>Search Algorithms</h2>
            {#if selectedDb == null}
                <button class="cta-button inactive">
                    <Search size={18} />
                    <span>Search</span>
                </button>
            {:else}
                <button
                    class="cta-button {isSearching ? 'cancel' : ''}"
                    on:click={toggleSearch}
                >
                    <div class="flex items-center gap-2">
                        {#if isSearching}
                            <X size={18} />
                            <span>Cancel</span>
                        {:else}
                            <Search size={18} />
                            <span>Search</span>
                        {/if}
                    </div>
                </button>
            {/if}
        </div>
        {#if isSearching}
            <div class="block inner">
                <span>
                    <Loader
                        size={24}
                        class="spinner ml-2"
                        style="color: var(--accent-color)"
                    />
                    {searchMessage}
                </span>
                <div class="progress-container">
                    <div
                        class="progress-bar"
                        style="width: {searchProgress}%"
                    ></div>
                </div>
                <span>
                    {subsearchMessage}
                </span>
                <div class="progress-container">
                    <div
                        class="progress-bar"
                        style="width: {subsearchProgress}%"
                    ></div>
                </div>
                <!-- <AlertCircle size={18} class="ellipsis" /> -->
            </div>
        {:else}
            <div class="grid">
                {#each $algorithmsStore as algo}
                    <div
                        class="grid item {getAlgoClass(algo, $algorithmsStore)}"
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
                                class={(algo.id === "audiosuite" ||
                                    algo.id === "filename") &&
                                !isBasicEnabled
                                    ? "inactive"
                                    : ""}
                            >
                                {algo.name}
                            </span>
                        </button>

                        {#if algo.id === "dbcompare"}
                            {#if algo.db !== null && algo.db !== undefined}
                                {#await getFilenameWithoutExtension(algo.db) then filename}
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
                                style="width: 55px"
                            />
                            s
                        {/if}
                    </div>
                {/each}
            </div>
        {/if}
        <!-- </div> -->
    </div>

    <div class="block" style="height: 100%; margin-top: 20px">
        <div class="header">
            <h2>Metadata Replacement</h2>
            {#if selectedDb == null}
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

    /* Add these styles */
    .progress-container {
        width: 100%;
        height: 6px;
        background-color: var(--secondary-bg);
        border-radius: 3px;
        margin: 8px 0;
        overflow: hidden;
    }

    .progress-bar {
        height: 100%;
        background-color: var(--accent-color);
        transition: width 0.3s ease;
    }
</style>
