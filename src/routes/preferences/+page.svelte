<script lang="ts">
    import {
        ListOrdered,
        ListCheck,
        ListChecks,
        Tags,
        Palette,
        Settings2,
    } from "lucide-svelte";
    import "../../styles.css";
    import { onMount } from "svelte";

    import MainComponent from "../../components/prefs/Main.svelte";
    import MatchComponent from "../../components/prefs/Match.svelte";
    import OrderComponent from "../../components/prefs/Order.svelte";
    import TagsComponent from "../../components/prefs/Tags.svelte";
    import SelectComponent from "../../components/prefs/Select.svelte";
    import ColorsComponent from "../../components/prefs/Colors.svelte";
    import {
        preferencesStore,
        PresetsStore,
        defaultPreferences,
        defaultAlgorithms,
        // algorithmsStore,
    } from "../../store";
    import { get } from "svelte/store";
    import type { Preferences } from "../../store";

    // Use the store directly instead of assigning to `pref`

    let newPreset: string;
    let selectedPreset: string = ""; // Bind this to <select>
    export let activeTab = "matchCriteria"; // Ensure this matches below
    $: console.log("Active Tab:", activeTab);
    $: pref = $preferencesStore;
    $: presets = $PresetsStore;

    // $: if ($preferencesStore && $preferencesStore.algorithms) {
    //     algorithmsStore.set($preferencesStore.algorithms);
    // }

    function savePreset() {
        const trimmedPreset = newPreset?.trim();

        // Make sure the preset name is valid
        if (trimmedPreset) {
            if (trimmedPreset === "Default") {
                console.log("Cannot update or save the Default preset.");
                return;
            }

            // Check if the preset already exists
            const existingPresetIndex = presets.findIndex(
                (p) => p.name === trimmedPreset,
            );

            if (existingPresetIndex !== -1) {
                // If it exists, update its preferences
                PresetsStore.update((presets) => {
                    presets[existingPresetIndex].pref = get(preferencesStore); // Update the preferences
                    return [...presets]; // Return updated presets
                });
                console.log("Preset updated:", trimmedPreset);
            } else {
                // If it doesn't exist, create a new preset
                PresetsStore.update((presets) => [
                    ...presets,
                    { name: trimmedPreset, pref: get(preferencesStore) },
                ]);
                console.log("Preset saved:", trimmedPreset);
            }

            selectedPreset = trimmedPreset; // Update the selected preset
            newPreset = ""; // Clear input after saving
        }
    }

    function loadPreset() {
        if (selectedPreset === "Default") {
            // Get fresh default preferences from store's defaultPreferences
            const defaultPrefs = structuredClone(defaultPreferences); // Create deep copy to avoid reference issues
            preferencesStore.set(defaultPrefs);

            // Update CSS variables for colors
            Object.entries(defaultPrefs.colors).forEach(([key, value]) => {
                const cssVariable = `--${key.replace(/([A-Z])/g, "-$1").toLowerCase()}`;
                document.documentElement.style.setProperty(cssVariable, value);
            });

            console.log("Default preferences restored");
            return;
        }

        // Existing preset loading logic
        const presetObj = presets.find((p) => p.name === selectedPreset);
        if (presetObj) {
            // Create deep copies to avoid reference issues
            const prefCopy = structuredClone(presetObj.pref);
            const defaultPrefs = structuredClone(defaultPreferences);
            const pref = { ...defaultPrefs, ...prefCopy }; // Merge with default preferences

            // Ensure algorithms are set correctly
            if (pref && pref.algorithms) {
                console.log("Loading algorithms:", prefCopy.algorithms);

                // Set the preferences store, which will include algorithms
                preferencesStore.set(pref);

                // Log to verify store was updated
                console.log(
                    "Preferences store updated:",
                    get(preferencesStore),
                );
            } else {
                console.error("Invalid algorithms in preset:", selectedPreset);
                // Fallback to default algorithms if not present
                pref.algorithms = defaultAlgorithms;
                preferencesStore.set(pref);
            }

            // Update CSS variables
            Object.entries(pref.colors || {}).forEach(([key, value]) => {
                const cssVariable = `--${key.replace(/([A-Z])/g, "-$1").toLowerCase()}`;
                document.documentElement.style.setProperty(cssVariable, value);
            });
        }
    }

    function deletePreset() {
        if (!selectedPreset || selectedPreset === "Default") {
            console.log("Cannot delete the Default preset.");
            return;
        }

        // Remove the selected preset
        PresetsStore.update((presets) =>
            presets.filter((p) => p.name !== selectedPreset),
        );

        console.log("Preset deleted:", selectedPreset);

        // Update the selection to another preset or default
        selectedPreset = "";
    }

    onMount(() => {
        // Get current preferences when component mounts
        const currentPrefs = get(preferencesStore);

        // Update CSS variables from current preferences
        if (currentPrefs?.colors) {
            Object.entries(currentPrefs.colors).forEach(([key, value]) => {
                const cssVariable = `--${key.replace(/([A-Z])/g, "-$1").toLowerCase()}`;
                document.documentElement.style.setProperty(cssVariable, value);
            });
        }

        console.log("Preferences window mounted, colors updated");
    });
</script>

<div class="app-container">
    <div class="top-bar">
        <div class="top-bar-left">
            <!-- <button
                class="nav-link {activeTab === 'mainPref' ? 'active' : ''}"
                on:click={() => (activeTab = "mainPref")}
            >
                <div class="flex items-center gap-2">
                    <Settings2 size={18} />
                    <span>Options</span>
                </div>
            </button> -->
            <button
                class="nav-link {activeTab === 'matchCriteria' ? 'active' : ''}"
                on:click={() => (activeTab = "matchCriteria")}
            >
                <div class="flex items-center gap-2">
                    <ListCheck size={18} />
                    <span>Match Criteria</span>
                </div>
            </button>
            <button
                class="nav-link {activeTab === 'preservationOrder'
                    ? 'active'
                    : ''}"
                on:click={() => (activeTab = "preservationOrder")}
            >
                <div class="flex items-center gap-2">
                    <ListOrdered size={18} />
                    <span>Preservation Order</span>
                </div>
            </button>
            <button
                class="nav-link {activeTab === 'Tags Editor' ? 'active' : ''}"
                on:click={() => (activeTab = "audiosuiteTags")}
            >
                <div class="flex items-center gap-2">
                    <Tags size={18} />
                    <span>Tags Manager</span>
                </div>
            </button>
            <!-- <button 
                class="nav-link {activeTab === 'autoSelect' ? 'active' : ''}"
                on:click={() => activeTab = 'autoSelect'}
            >
                <div class="flex items-center gap-2">
                    <ListChecks size={18} />
                    <span>AutoSelect Strings</span>
                </div>
            </button> -->
            <button
                class="nav-link {activeTab === 'colors' ? 'active' : ''}"
                on:click={() => (activeTab = "colors")}
            >
                <div class="flex items-center gap-2">
                    <Palette size={18} />
                    <span>Colors</span>
                </div>
            </button>
        </div>
    </div>

    <main class="content" style="margin-bottom: 0px">
        <div>
            {#if activeTab === "mainPref"}
                <MainComponent />
            {:else if activeTab === "matchCriteria"}
                <MatchComponent />
            {:else if activeTab === "preservationOrder"}
                <OrderComponent />
            {:else if activeTab === "audiosuiteTags"}
                <TagsComponent />
            {:else if activeTab === "autoSelect"}
                <SelectComponent />
            {:else if activeTab === "colors"}
                <ColorsComponent />
            {/if}
        </div>
        <div
            class="bar"
            style="width: calc(100% + 40px); margin-top: 16px; margin-left: -20px; margin-right: 20px;"
        >
            <button
                class="cta-button small"
                style="margin-left: 30px"
                on:click={savePreset}>Save:</button
            >
            <input
                type="text"
                class="input-field"
                placeholder="Enter New Configuration Preset Name"
                style=" margin-right: 10px;"
                bind:value={newPreset}
            />
            <!-- <button class="cta-button small" on:click={loadPreset}>
                Load:
            </button> -->
            <select
                class="select-field"
                style="margin-right: 10px"
                bind:value={selectedPreset}
                on:change={loadPreset}
            >
                {#each presets as p}
                    <option value={p.name}>{p.name}</option>
                {/each}
            </select>
            <button
                class="cta-button small cancel"
                style="margin-right: 25px;"
                on:click={deletePreset}
            >
                Delete
            </button>
        </div>
    </main>
</div>

<style>
    .bar {
        background-color: var(--primary-bg);
    }

    .top-bar {
        display: flex;
        justify-content: space-between;
        align-items: center;
        background-color: var(--topbar-color);
        padding: 10px;
        width: 100%;
    }

    .top-bar-left {
        display: flex;
        justify-content: space-between;
        align-items: center;
        width: 100%;
        padding: 30px;
    }

    .app-container {
        display: flex;
        flex-direction: column;
        height: 100vh;
        /* margin-bottom: 40px; */
    }

    .content {
        flex-grow: 1;
        overflow-y: auto;
    }

    .bar {
        display: flex;
        align-items: center;
        justify-content: center;
        padding: 10px;
        background-color: var(--secondary-bg);
        color: var(--text-color);
        width: 100%;
        position: sticky;
        bottom: 0;
    }
</style>
