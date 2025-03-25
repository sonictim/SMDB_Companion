<script lang="ts">
    import { createVirtualizer } from "@tanstack/svelte-virtual";
    import { onMount, onDestroy, createEventDispatcher } from "svelte";

    // Props
    export let items: any[] = [];
    export let selectedItems: Set<any> = new Set();
    export let getItemId = (item: any) => item.id;
    export let estimatedItemSize = 40;
    export let overscan = 5;
    export let enableSelections = true;
    export let filterKey: string | null = null;
    export let filterValue: any = null;
    export let metadataKeys: string[] = [];
    export let getAlgorithmIcon: (algoName: string) => any = () => null;

    // Event dispatcher
    const dispatch = createEventDispatcher<{
        itemClick: { item: any; event: MouseEvent };
        toggleChecked: { item: any };
        playAudio: { item: any };
        resize: { index: number; width: number };
        selectionChange: { selectedItems: Set<any> };
    }>();

    // Derived data
    $: filteredItems =
        filterKey && filterValue !== null
            ? items.filter((item) => item[filterKey] === filterValue)
            : items;

    // References
    let parentRef: Element;
    let containerElement: HTMLElement;
    let parentWidth = 0;
    let parentHeight = 0;
    let containerWidth = 0;
    let lastSelectedIndex = -1;

    // Column configuration
    type ColumnConfig = {
        minWidth: number;
        width: number; // Current width in pixels
        percentage: number; // Width as a percentage of total width
        name: string;
        isMetadata: boolean;
    };

    // Initial column configurations
    let columnConfigs: ColumnConfig[] = [
        {
            minWidth: 8,
            width: 15,
            percentage: 1.5,
            name: "audio",
            isMetadata: false,
        },
        {
            minWidth: 10,
            width: 20,
            percentage: 2,
            name: "checkbox",
            isMetadata: false,
        },
        {
            minWidth: 100,
            width: 200,
            percentage: 28,
            name: "filename",
            isMetadata: false,
        },
        {
            minWidth: 150,
            width: 300,
            percentage: 58,
            name: "path",
            isMetadata: false,
        },
        {
            minWidth: 20,
            width: 30,
            percentage: 10.5,
            name: "algorithm",
            isMetadata: false,
        },
    ];

    // Update columns when metadata keys change
    $: {
        if (metadataKeys && metadataKeys.length > 0) {
            // Keep the fixed columns
            const fixedColumnConfigs = columnConfigs.filter(
                (col) => !col.isMetadata,
            );

            // Create dynamic metadata columns
            const metadataColumnConfigs = metadataKeys.map((key) => ({
                minWidth: 20,
                width: 150,
                percentage: 30 / metadataKeys.length,
                name: key,
                isMetadata: true,
            }));

            // Combine fixed and metadata columns
            columnConfigs = [...fixedColumnConfigs, ...metadataColumnConfigs];

            // Update column widths
            updateColumnWidthsFromContainer();
        }
    }

    // Column width calculations
    $: columnWidths = columnConfigs.map((config) => config.width);
    $: totalWidth = columnWidths.reduce((sum, width) => sum + width, 0);
    $: gridTemplateColumns = columnWidths
        .map((width) => `${width}px`)
        .join(" ");

    // Virtualizer
    $: rowVirtualizer = createVirtualizer({
        count: filteredItems.length,
        estimateSize: () => estimatedItemSize,
        overscan,
        getScrollElement: () => parentRef,
    });

    // Update virtualizer while preserving scroll position
    function updateVirtualizer() {
        if ($rowVirtualizer) {
            const scrollElement = parentRef;
            const scrollTop = scrollElement?.scrollTop;

            $rowVirtualizer.measure();

            if (scrollTop !== undefined) {
                queueMicrotask(() => {
                    if (scrollElement) scrollElement.scrollTop = scrollTop;
                });
            }
        }
    }

    // Resize handling
    function handleResize() {
        if (containerElement) {
            const newContainerWidth = containerElement.clientWidth;
            if (newContainerWidth !== containerWidth) {
                containerWidth = newContainerWidth;
                updateColumnWidthsFromContainer();
            }
        }
    }

    // Update column widths based on container size
    function updateColumnWidthsFromContainer() {
        if (!containerWidth) return;

        const availableWidth = containerWidth - 20;

        columnConfigs = columnConfigs.map((config) => {
            const calculatedWidth = Math.max(
                config.minWidth,
                Math.floor(availableWidth * (config.percentage / 100)),
            );
            return { ...config, width: calculatedWidth };
        });
    }

    // Column resizing
    function startResize(index: number, event: MouseEvent) {
        event.preventDefault();

        const startX = event.clientX;
        const startWidth = columnConfigs[index].width;
        const totalWidthBefore = columnConfigs.reduce(
            (sum, col) => sum + col.width,
            0,
        );

        function onMouseMove(e: MouseEvent) {
            const diff = e.clientX - startX;
            const newWidth = Math.max(
                columnConfigs[index].minWidth,
                startWidth + diff,
            );

            // Update width in pixels
            columnConfigs[index].width = newWidth;

            // Recalculate percentages for all columns
            const totalWidthAfter = columnConfigs.reduce(
                (sum, col) => sum + col.width,
                0,
            );
            columnConfigs = columnConfigs.map((config) => {
                return {
                    ...config,
                    percentage: (config.width / totalWidthAfter) * 100,
                };
            });

            dispatch("resize", { index, width: newWidth });
        }

        function onMouseUp() {
            window.removeEventListener("mousemove", onMouseMove);
            window.removeEventListener("mouseup", onMouseUp);
        }

        window.addEventListener("mousemove", onMouseMove);
        window.addEventListener("mouseup", onMouseUp);
    }

    // Selection handling
    function toggleSelect(item: any, event: MouseEvent) {
        event.preventDefault();

        const scrollElement = parentRef;
        const scrollTop = scrollElement?.scrollTop;

        const currentIndex = filteredItems.findIndex(
            (record) => getItemId(record) === getItemId(item),
        );

        // Handle Option/Alt click (toggle all)
        if (event.altKey) {
            if (selectedItems.size > 0) {
                selectedItems.clear();
            } else {
                filteredItems.forEach((record) =>
                    selectedItems.add(getItemId(record)),
                );
            }
            selectedItems = new Set(selectedItems);
            dispatch("selectionChange", { selectedItems });
            queueMicrotask(() => {
                updateVirtualizer();
                if (scrollTop !== undefined && scrollElement) {
                    scrollElement.scrollTop = scrollTop;
                }
            });
            return;
        }

        // Handle Shift click (range selection)
        if (event.shiftKey && lastSelectedIndex !== -1) {
            const start = Math.min(lastSelectedIndex, currentIndex);
            const end = Math.max(lastSelectedIndex, currentIndex);

            for (let i = start; i <= end; i++) {
                selectedItems.add(getItemId(filteredItems[i]));
            }
        } else {
            // Normal click (toggle individual)
            if (selectedItems.has(getItemId(item))) {
                selectedItems.delete(getItemId(item));
            } else {
                selectedItems.add(getItemId(item));
                lastSelectedIndex = currentIndex;
            }
        }

        selectedItems = new Set(selectedItems);
        dispatch("selectionChange", { selectedItems });

        queueMicrotask(() => {
            updateVirtualizer();
            if (scrollTop !== undefined && scrollElement) {
                scrollElement.scrollTop = scrollTop;
            }
        });
    }

    // Event handlers
    function handleItemClick(item: any, event: MouseEvent) {
        if (enableSelections) {
            toggleSelect(item, event);
        } else {
            dispatch("itemClick", { item, event });
        }
    }

    function handleToggleChecked(item: any) {
        dispatch("toggleChecked", { item });
    }

    function handlePlayAudio(item: any) {
        dispatch("playAudio", { item });
    }

    // Update virtualizer when selections change
    $: {
        if (selectedItems) {
            updateVirtualizer();
        }
    }

    // Initialize resize observer
    onMount(() => {
        if (containerElement) {
            containerWidth = containerElement.clientWidth;
            updateColumnWidthsFromContainer();
        }

        const resizeObserver = new ResizeObserver((entries) => {
            if (entries[0]) {
                parentWidth = entries[0].contentRect.width;
                parentHeight = entries[0].contentRect.height;
                handleResize();
                if ($rowVirtualizer) {
                    $rowVirtualizer.measure();
                }
            }
        });

        if (parentRef) {
            resizeObserver.observe(parentRef);
        }

        window.addEventListener("resize", handleResize);

        return () => {
            window.removeEventListener("resize", handleResize);
            resizeObserver.disconnect();
        };
    });
</script>

<div class="virtual-table-container" bind:this={containerElement}>
    <div bind:this={parentRef} class="virtual-table-viewport">
        <!-- Table header (fixed, not virtualized) -->
        <div class="virtual-table-header" style="width: {totalWidth}px;">
            <div
                class="grid-container rheader"
                style="grid-template-columns: {gridTemplateColumns};"
            >
                <div class="grid-item header"></div>
                <div class="grid-item header">✔</div>
                <div class="grid-item header bold">Filename</div>
                <div class="grid-item header">Path</div>
                <div class="grid-item header">
                    <span>Match</span>
                </div>

                <!-- Metadata column headers -->
                {#each metadataKeys as key}
                    <div class="grid-item header">{key}</div>
                {/each}
            </div>

            <!-- Resizers -->
            <div
                class="resizer-container"
                style="grid-template-columns: {gridTemplateColumns};"
            >
                <!-- Audio column resizer -->
                <div class="resizer-cell">
                    <div></div>
                </div>

                <!-- Checkbox column resizer -->
                <div class="resizer-cell">
                    <div
                        class="resizer"
                        on:mousedown={(event) => startResize(1, event)}
                    ></div>
                </div>

                <!-- Filename column resizer -->
                <div class="resizer-cell">
                    <div
                        class="resizer"
                        on:mousedown={(event) => startResize(2, event)}
                    ></div>
                </div>

                <!-- Path column resizer -->
                <div class="resizer-cell">
                    <div
                        class="resizer"
                        on:mousedown={(event) => startResize(3, event)}
                    ></div>
                </div>

                <!-- Algorithm column resizer -->
                <div class="resizer-cell">
                    <div
                        class="resizer"
                        on:mousedown={(event) => startResize(4, event)}
                    ></div>
                </div>

                <!-- Metadata column resizers -->
                {#each metadataKeys as key, i}
                    <div class="resizer-cell">
                        <div
                            class="resizer"
                            on:mousedown={(event) => startResize(5 + i, event)}
                        ></div>
                    </div>
                {/each}
            </div>
        </div>

        <!-- Virtualized rows -->
        <div
            class="virtual-table-body"
            style="height: {$rowVirtualizer.getTotalSize()}px; width: {totalWidth}px;"
        >
            {#each $rowVirtualizer.getVirtualItems() as virtualRow (virtualRow.index)}
                {#if filteredItems[virtualRow.index]}
                    <div
                        class="virtual-row"
                        style="transform: translateY({virtualRow.start}px); height: {virtualRow.size}px; width: {totalWidth}px;"
                    >
                        <div
                            class="list-item {filteredItems[
                                virtualRow.index
                            ].algorithm?.includes('Keep')
                                ? 'unselected-item'
                                : 'checked-item'}"
                        >
                            <div
                                class="grid-container"
                                style="{selectedItems.has(
                                    getItemId(filteredItems[virtualRow.index]),
                                ) && enableSelections
                                    ? 'background-color: var(--accent-color)'
                                    : ''};
                grid-template-columns: {gridTemplateColumns};"
                            >
                                <!-- Audio Column -->
                                <div
                                    class="grid-item"
                                    on:click={() =>
                                        handlePlayAudio(
                                            filteredItems[virtualRow.index],
                                        )}
                                >
                                    <slot
                                        name="audio-icon"
                                        item={filteredItems[virtualRow.index]}
                                    >
                                        <span>▶</span>
                                    </slot>
                                </div>

                                <!-- Checkbox Column -->
                                <div
                                    class="grid-item"
                                    on:click={() =>
                                        handleToggleChecked(
                                            filteredItems[virtualRow.index],
                                        )}
                                >
                                    <slot
                                        name="checkbox"
                                        item={filteredItems[virtualRow.index]}
                                    >
                                        <span
                                            >{filteredItems[
                                                virtualRow.index
                                            ].algorithm?.includes("Keep")
                                                ? "☐"
                                                : "☑"}</span
                                        >
                                    </slot>
                                </div>

                                <!-- Filename Column -->
                                <div
                                    class="grid-item bold"
                                    on:click={(event) =>
                                        handleItemClick(
                                            filteredItems[virtualRow.index],
                                            event,
                                        )}
                                >
                                    <slot
                                        name="filename"
                                        item={filteredItems[virtualRow.index]}
                                    >
                                        {filteredItems[virtualRow.index].root ||
                                            ""}
                                    </slot>
                                </div>

                                <!-- Path Column -->
                                <div
                                    class="grid-item"
                                    on:click={(event) =>
                                        handleItemClick(
                                            filteredItems[virtualRow.index],
                                            event,
                                        )}
                                >
                                    <slot
                                        name="path"
                                        item={filteredItems[virtualRow.index]}
                                    >
                                        {filteredItems[virtualRow.index].path ||
                                            ""}
                                    </slot>
                                </div>

                                <!-- Algorithm Column -->
                                <div
                                    class="grid-item"
                                    on:click={(event) =>
                                        handleItemClick(
                                            filteredItems[virtualRow.index],
                                            event,
                                        )}
                                >
                                    <slot
                                        name="algorithm"
                                        item={filteredItems[virtualRow.index]}
                                    >
                                        <div class="algorithm-icons">
                                            {#if filteredItems[virtualRow.index].algorithm}
                                                {#each filteredItems[virtualRow.index].algorithm.filter((algo: string) => algo !== "Keep" || filteredItems[virtualRow.index].algorithm.length === 1) as algo}
                                                    {#if getAlgorithmIcon}
                                                        {@const iconData =
                                                            getAlgorithmIcon(
                                                                algo,
                                                            )}
                                                        <span
                                                            class="icon-wrapper"
                                                            title={iconData.tooltip}
                                                        >
                                                            <svelte:component
                                                                this={iconData.component}
                                                                size={20}
                                                                style={iconData.color
                                                                    ? `color: ${iconData.color};`
                                                                    : ""}
                                                            />
                                                        </span>
                                                    {:else}
                                                        <span
                                                            class="icon-wrapper"
                                                            title={algo}
                                                            >{algo}</span
                                                        >
                                                    {/if}
                                                {/each}
                                            {/if}
                                        </div>
                                    </slot>
                                </div>

                                <!-- Metadata columns -->
                                {#each metadataKeys as key}
                                    <div
                                        class="grid-item"
                                        on:click={(event) =>
                                            handleItemClick(
                                                filteredItems[virtualRow.index],
                                                event,
                                            )}
                                    >
                                        <slot
                                            name="metadata-item"
                                            item={filteredItems[
                                                virtualRow.index
                                            ]}
                                            metadataKey={key}
                                        >
                                            {filteredItems[
                                                virtualRow.index
                                            ]?.data?.[key]?.toString() || ""}
                                        </slot>
                                    </div>
                                {/each}
                            </div>
                        </div>
                    </div>
                {/if}
            {/each}
        </div>
    </div>
</div>

<style>
    .virtual-table-container {
        position: relative;
        overflow: hidden;
        width: 100%;
        height: 100%;
    }

    .virtual-table-viewport {
        overflow: auto;
        height: 100%;
        width: 100%;
        will-change: transform;
        position: relative;
    }

    .virtual-table-header {
        position: sticky;
        top: 0;
        z-index: 10;
        background-color: var(--primary-bg);
        box-shadow: 0 1px 0 rgba(0, 0, 0, 0.1);
    }

    .virtual-table-body {
        position: relative;
    }

    .virtual-row {
        position: absolute;
        top: 0;
        left: 0;
        width: 100%;
    }

    .resizer-container {
        display: grid;
        height: 5px;
        position: relative;
        cursor: row-resize;
    }

    .resizer-cell {
        position: relative;
        overflow: visible;
        height: 100%;
    }

    .resizer {
        width: 4px;
        height: 60px;
        background-color: var(--inactive-color);
        position: absolute;
        right: -20px;
        top: -60px;
        cursor: col-resize;
        z-index: 20;
        opacity: 0.7;
    }

    .resizer:hover {
        background-color: var(--hover-color);
        opacity: 1;
    }

    .grid-container {
        display: grid;
        width: 100%;
    }

    .grid-item {
        position: relative;
        padding: 3px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        font-size: 14px;
        user-select: none;
        -webkit-user-select: none;
        -moz-user-select: none;
        -ms-user-select: none;
    }

    .grid-item.header {
        font-size: 16px;
        background-color: var(--primary-bg);
    }

    .grid-item.bold {
        font-weight: bold;
    }

    .rheader {
        font-weight: bold;
        font-size: 16px;
        color: var(--accent-color);
        background-color: var(--primary-bg);
        border-bottom: 1px solid var(--inactive-color);
    }

    .list-item {
        user-select: none;
        -webkit-user-select: none;
        -moz-user-select: none;
        -ms-user-select: none;
    }

    .algorithm-icons {
        display: flex;
        flex-wrap: wrap;
        gap: 8px;
    }

    .icon-wrapper {
        display: inline-flex;
        align-items: center;
        justify-content: center;
    }

    .icon-wrapper:hover::after {
        content: attr(title);
        position: absolute;
        background: var(--primary-bg);
        border: 1px solid var(--border-color);
        padding: 2px 6px;
        border-radius: 4px;
        font-size: 10px;
        z-index: 100;
        white-space: nowrap;
        top: 100%;
        left: 50%;
        transform: translateX(-50%);
    }
</style>
