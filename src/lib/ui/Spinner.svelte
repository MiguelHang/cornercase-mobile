<script lang="ts">
  import Icon from './Icon.svelte'
</script>

<span class="spinner" aria-hidden="true"><Icon name="loader-circle" size={12} /></span>

<style>
  .spinner {
    display: inline-flex;
    flex: none;
    animation: turn 900ms linear infinite;
  }

  @keyframes turn {
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .spinner {
      animation: none;
    }
  }
</style>
