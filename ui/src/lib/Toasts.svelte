<script lang="ts">
  import { toasts, dismissToast } from "./toastStore";
</script>

{#if $toasts.length > 0}
<div class="toast-container">
  {#each $toasts as toast (toast.id)}
    <div class="toast" class:error={toast.type === "error"} class:success={toast.type === "success"}>
      <span>{toast.message}</span>
      <button class="toast-close" on:click={() => dismissToast(toast.id)}>×</button>
    </div>
  {/each}
</div>
{/if}

<style>
  .toast-container {
    position: fixed;
    bottom: 16px;
    right: 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 9999;
    pointer-events: none;
  }

  .toast {
    pointer-events: auto;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    border-radius: var(--radius-md);
    background: var(--bg-panel-solid);
    color: var(--text-primary);
    font-size: 12px;
    box-shadow: var(--shadow-md);
    border-left: 3px solid transparent;
    animation: slideIn 200ms ease forwards;
    max-width: 360px;
  }

  .toast.error {
    border-left-color: #ef4444;
  }

  .toast.success {
    border-left-color: #22c55e;
  }

  .toast-close {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 16px;
    line-height: 1;
    padding: 0 2px;
    flex-shrink: 0;
  }

  .toast-close:hover {
    color: var(--text-primary);
  }

  @keyframes slideIn {
    from {
      opacity: 0;
      transform: translateX(20px);
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
  }
</style>
