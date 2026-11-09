<script lang="ts">
  const GRID = 12

  let { unit = 4, pulsing = false }: { unit?: number; pulsing?: boolean } = $props()

  const size = $derived(`${GRID * unit}px`)
</script>

<div class="mark" style:--unit="{unit}px" style:width={size} style:height={size} aria-hidden="true">
  <span class="cell corner top"></span>
  <span class="cell corner side"></span>
  <span class="cell cursor" class:pulsing></span>
</div>

<style>
  .mark {
    position: relative;
    flex: none;
  }

  .cell {
    position: absolute;
  }

  .corner {
    background: var(--brand);
    box-shadow: 0 0 calc(3 * var(--unit)) rgb(0 95 255 / 0.55);
  }

  .top {
    left: calc(3 * var(--unit));
    top: 0;
    width: calc(9 * var(--unit));
    height: calc(3 * var(--unit));
  }

  .side {
    left: 0;
    top: calc(3 * var(--unit));
    width: calc(3 * var(--unit));
    height: calc(9 * var(--unit));
  }

  .cursor {
    left: calc(6 * var(--unit));
    top: calc(6 * var(--unit));
    width: calc(2 * var(--unit));
    height: calc(5 * var(--unit));
    background: var(--text);
  }

  .pulsing {
    animation: pulse var(--pulse) ease-in-out infinite;
  }

  @keyframes pulse {
    50% {
      opacity: 0.3;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .pulsing {
      animation: none;
    }
  }
</style>
