<script lang="ts">
  import type { Tone } from '../connection'

  let { tone, pulsing = false }: { tone: Tone; pulsing?: boolean } = $props()
</script>

<span class="dot {tone}" class:pulsing></span>

<style>
  .dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 999px;
  }

  .working {
    background: var(--working);
  }

  .done {
    background: var(--done);
  }

  .failed {
    background: var(--failed);
  }
</style>
