<script lang="ts">
    import { createVirtualizer } from "@tanstack/svelte-virtual";
    import { onMount } from "svelte";

    export let items: string | any[] = [];
    export let columnWidths: any[] = [];
    export let height = "60vh";
    export let width = "100%";
    export let estimatedItemSize = 40;
    export let overscan = 5;

    let parentRef: Element;
    let parentWidth = 0;
    let parentHeight = 0;

    // Calculate total content width from column widths
    $: totalWidth = columnWidths.reduce((sum, width) => sum + width, 0);

    // Create vertical virtualizer for rows
    $: rowVirtualizer = createVirtualizer({
        count: items.length,
        estimateSize: () => estimatedItemSize,
        overscan,
        getScrollElement: () => parentRef,
    });

    onMount(() => {
        // Force an update to handle initial viewport sizes
        const resizeObserver = new ResizeObserver((entries) => {
            if (entries[0]) {
                parentWidth = entries[0].contentRect.width;
                parentHeight = entries[0].contentRect.height;
                $rowVirtualizer.measure();
            }
        });

        if (parentRef) {
            resizeObserver.observe(parentRef);
        }

        return () => {
            resizeObserver.disconnect();
        };
    });
</script>

<div class="virtual-table-container" style="height: {height}; width: {width};">
    <div bind:this={parentRef} class="virtual-table-viewport">
        <!-- Table header (fixed, not virtualized) -->
        <div class="virtual-table-header" style="width: {totalWidth}px;">
            <slot name="header"></slot>
        </div>

        <!-- Virtualized rows -->
        <div
            class="virtual-table-body"
            style="height: {$rowVirtualizer.getTotalSize()}px; width: {totalWidth}px;"
        >
            {#each $rowVirtualizer.getVirtualItems() as virtualRow (virtualRow.index)}
                <div
                    class="virtual-row"
                    style="transform: translateY({virtualRow.start}px); height: {virtualRow.size}px; width: {totalWidth}px;"
                >
                    <slot
                        name="row"
                        item={items[virtualRow.index]}
                        index={virtualRow.index}
                    ></slot>
                </div>
            {/each}
        </div>
    </div>
</div>

<style>
    .virtual-table-container {
        position: relative;
        overflow: hidden;
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
</style>
