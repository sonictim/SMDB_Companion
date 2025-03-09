<script lang="ts">
    import { writable } from 'svelte/store';
//     import { onMount } from 'svelte';
//   onMount(() => {
//     window.addEventListener("dragover", () => console.log("Global dragover detected"));
//     window.addEventListener("drop", () => console.log("Global drop detected"));
// });

    let items = writable(['Item 1', 'Item 2', 'Item 3', 'Item 4']);
    let dragSourceIndex: number | null = null;

    function onDragStart(event: DragEvent, index: number) {
        console.log('drag start', index);
        dragSourceIndex = index;
        if (event.dataTransfer) {
            event.dataTransfer.effectAllowed = 'move';
            event.dataTransfer.setData('text/plain', index.toString());
        }
    }

    function onDragOver(event: DragEvent) {
        event.preventDefault(); // Necessary for drop to work
        event.dataTransfer!.dropEffect = "move"; // Show correct cursor
        console.log('drag over');
    }

    function onDrop(event: DragEvent) {
        event.preventDefault();
        event.stopPropagation(); // Prevent event bubbling issues
        console.log('drop triggered');

        const sourceIndex = dragSourceIndex;
        if (sourceIndex !== null) {
            const targetIndex = parseInt(event.dataTransfer?.getData('text/plain') || '-1', 10);
            if (targetIndex >= 0 && sourceIndex !== targetIndex) {
                items.update(list => {
                    const updatedList = [...list];
                    const [movedItem] = updatedList.splice(sourceIndex, 1);
                    updatedList.splice(targetIndex, 0, movedItem);
                    return updatedList;
                });
            }
        }
        dragSourceIndex = null;
    }
</script>

<style>
    .list {
        padding: 10px;
        width: 200px;
        border: 1px solid #ccc;
        min-height: 150px;
    }
    .list-item {
        padding: 10px;
        margin: 5px 0;
        background-color: #393535;
        cursor: grab;
        border: 1px solid #aaa;
    }
    .list-item:active {
        cursor: grabbing;
    }
</style>

<!-- Wrap the list in a container that handles drop events -->
<div class="list" on:dragover={onDragOver} on:drop={onDrop}>
    {#each $items as item, index}
        <div
            class="list-item"
            draggable="true"
            on:dragstart={(e) => onDragStart(e, index)}
        >
            {item}
        </div>
    {/each}
</div>
