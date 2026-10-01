<script lang="ts">
  import { getContext, onMount } from 'svelte';
  import { X } from '@lucide/svelte';
  import type { Snippet } from 'svelte';
  let {
    title,
    close,
    children,
  }: { title: string; close: () => void; children: Snippet } = $props();
  let dialog: HTMLDialogElement;
  const feedback = getContext<{ error: () => string }>('feedback');
  onMount(() => dialog.showModal());
</script>

<dialog
  bind:this={dialog}
  oncancel={(event) => {
    event.preventDefault();
    close();
  }}
>
  <header class="dialog-header">
    <h2>{title}</h2>
    <button class="icon-button" aria-label="Close dialog" onclick={close}
      ><X size={18} /></button
    >
  </header>
  {#if feedback?.error()}<p class="banner error" role="alert">
      {feedback.error()}
    </p>{/if}
  {@render children()}
</dialog>
