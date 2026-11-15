<script lang="ts">
  let {
    label,
    checked = $bindable(false),
    disabled = false,
  }: {
    label: string
    checked?: boolean
    disabled?: boolean
  } = $props()
</script>

<label class="switch">
  <span class="words">{label}</span>
  <input class="toggle" type="checkbox" role="switch" bind:checked {disabled} />
</label>

<style>
  .switch {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--gap);
    min-height: 44px;
  }

  .words {
    color: var(--text);
    font-size: 16px;
  }

  .toggle {
    position: relative;
    flex: none;
    width: 51px;
    height: 31px;
    margin: 0;
    border-radius: 16px;
    background: var(--edge);
    -webkit-appearance: none;
    appearance: none;
  }

  .toggle::after {
    content: '';
    position: absolute;
    top: 2px;
    left: 2px;
    width: 27px;
    height: 27px;
    border-radius: 50%;
    background: var(--text);
  }

  .toggle:checked {
    background: var(--brand);
  }

  .toggle:checked::after {
    transform: translateX(20px);
  }

  .toggle:disabled {
    opacity: 0.6;
  }

  @media (prefers-reduced-motion: no-preference) {
    .toggle {
      transition: background-color var(--enter) ease;
    }

    .toggle::after {
      transition: transform var(--enter) cubic-bezier(0.23, 1, 0.32, 1);
    }
  }
</style>
