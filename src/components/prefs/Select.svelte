<script lang="ts">
    import { invoke } from '@tauri-apps/api/core';
    import { onMount } from 'svelte';
    import VirtualList from 'svelte-virtual-list'; // Ensure this package is installed
    import { Square, CheckSquare, OctagonX, ArrowBigLeft, ArrowBigRight } from 'lucide-svelte';

    import { preferencesStore } from '../../store';
    import { get } from 'svelte/store';
    import type { Preferences } from '../../store';

    let newSelect: string;
    let newTag: string;

    $: pref = $preferencesStore;
    let selectedItems = new Set<string>();
    let selectedTags = new Set<string>();


    function toggleSelected(item: string) {
        if (selectedItems.has(item)) {
            selectedItems.delete(item);
        } else {
            selectedItems.add(item);
        }
        selectedItems = new Set(selectedItems); // Ensure reactivity
    }
    function toggleTags(item: string) {
        if (selectedTags.has(item)) {
            selectedTags.delete(item);
        } else {
            selectedTags.add(item);
        }
        selectedTags = new Set(selectedTags); // Ensure reactivity
    }

    function removeSelected(list: string[]) {
        list.forEach(item => removeSelect(item));
        clearSelected();
    }

    function removeSelect(item: string) {
        preferencesStore.update(pref => {
            // Filter out the item to remove it
            const updatedArray = pref.autoselects.filter(i => i !== item);
            return { ...pref, autoselects: updatedArray };
        });
    }

    function clearSelected() {
        selectedItems.clear();
        selectedItems = new Set(); // Ensure reactivity
    }

    function addSelected(item: string) {
        if (!pref.autoselects.includes(item)) {
            pref.autoselects = [...pref.autoselects, item];
            pref.autoselects.sort();
            preferencesStore.set(pref);
            newSelect = '';
        }
    }

function moveToTags() {
    selectedItems.forEach(item => addTag(item));
    selectedItems.forEach(item => removeSelect(item));
    clearSelected();
}
function moveToSelects() {
    selectedTags.forEach(item => addSelected(item));
    selectedTags.forEach(item => removeTag(item));
    clearTags();
}


    function addTag(item: string) {
        if (!pref.tags.includes(item)) {
            pref.tags = [...pref.tags, item];
            pref.tags.sort();
            preferencesStore.set(pref);
            newTag = '';
        }
    }




    function removeTags(list: string[]) {
        list.forEach(item => removeTag(item));
        clearTags();
    }

    function clearTags() {
        selectedTags.clear();
        selectedTags = new Set(); // Ensure reactivity
    }

    function removeTag(item: string) {
        preferencesStore.update(pref => {
            // Filter out the item to remove it
            const updatedArray = pref.tags.filter(i => i !== item);
            return { ...pref, tags: updatedArray };
        });
    }

</script>


<div class="page-columns">
    <div>
        <div class="block">
            <div class="header">
                <h2>
                   Audiosuite Tags
                </h2>
                <button class="cta-button cancel" on:click={() => removeTags([...selectedTags])}>
                    <OctagonX size=18/>
                    Remove
                  </button>
            </div>
            <!-- Files with these tags will only be marked for removal if they cannot find a root file with the same name. -->
        
            <div class="bar">
                    <button class="cta-button small" on:click={() => addTag(newTag)}>
                        Add
                    </button>
                    <input
                    type="text"
                    id="find-text"
                    bind:value={newTag}
                    placeholder="New Tag"
                    class="input-field"
                    />
        

  
        
        
            </div>
        
            <div class="block inner">

            
                <VirtualList items={Array.from(pref.tags)} let:item>
                    <div 
                        on:click={() => toggleTags(item)}
                        class="list-item"
                        class:selected-item={selectedTags.has(item)}
                        class:unselected-item={!selectedTags.has(item)}
                    >
                        {item}
                    </div>
                </VirtualList>
            </div>
        </div>
        
    </div>



    <div class="arrow-column">
        <div class="move-button-container">
            <button class="arrow-button" on:click={() => moveToSelects()}>
               <ArrowBigRight size=100/>
                
            </button>
            <!-- <span class="{selectedItems.size == 0 && selectedTags.size == 0 ? 'inactive' : ''}">Move Selected</span> -->
            <button class="arrow-button" on:click={() => moveToTags()}>
                <!-- → -->
                <ArrowBigLeft size=100/>
            </button>
        </div>
    </div>



    <div>
        <div class="block">
            <div class="header">
                <h2>
                   Filename Tags
                </h2>
                <button class="cta-button cancel" on:click={() => removeSelected([...selectedItems])}>
                    <OctagonX size=18/>
                    Remove
                  </button>
            </div>
            <div class="bar">
                <button class="cta-button small" on:click={() => addSelected(newSelect)}>
                    Add
                </button>
                    <input
                    type="text"
                    id="find-text"
                    bind:value={newSelect}
                    placeholder="Add New String"
                    class="input-field"
                    />
        
             
            
                
        
            </div>
        
            <div class="block inner">
                <VirtualList items={Array.from(pref.autoselects)} let:item>
                    <div 
                        on:click={() => toggleSelected(item)}
                        class="list-item"
                        class:selected-item={selectedItems.has(item)}
                        class:unselected-item={!selectedItems.has(item)}
                    >
                        {item}
                    </div>
                </VirtualList>
            </div>
        </div>
    </div>
    <!-- <div class="column">Column 3</div> -->
  </div>




  <style>
    .page-columns {
      display: grid;
      grid-template-columns: 1fr auto 1fr; /* Left, center, right */
      /* gap: 20px; */
    }
  

  
    .arrow-column {
      display: flex;
      justify-content: center;
      align-items: center;
    }
  
    .move-button-container {
      display: flex;
      flex-direction: column;
      align-items: center;
    }
  
    .arrow-button {
        color: var(--secondary-bg);
      font-size: 2rem;
      background: transparent;
      border: none;
      /* padding: 10px; */
      cursor: pointer;
      /* margin: 5px; */
    }
  
    .arrow-button:hover {
      /* background-color: #f0a500; */
      color: var(--accent-color);
    }
</style>