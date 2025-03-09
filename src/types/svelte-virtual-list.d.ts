declare module 'svelte-virtual-list' {
    import { SvelteComponentTyped } from 'svelte';
    
    export interface VirtualListProps<T> {
        items: T[];
        height?: string;
        itemHeight?: number;
        start?: number;
        end?: number;
    }

    export default class VirtualList<T = any> extends SvelteComponentTyped<VirtualListProps<T>> {}
}