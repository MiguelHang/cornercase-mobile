<script lang="ts">
  import { icons, type IconName, type Shape } from './icons/shapes'

  let { name, size = 24 }: { name: IconName; size?: number } = $props()

  const shapes: Shape[] = $derived(icons[name])
</script>

<svg
  xmlns="http://www.w3.org/2000/svg"
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.75"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#each shapes as shape, index (index)}
    {#if shape.kind === 'rect'}
      <rect x={shape.x} y={shape.y} width={shape.width} height={shape.height} rx={shape.rx} />
    {:else if shape.kind === 'line'}
      <line x1={shape.x1} y1={shape.y1} x2={shape.x2} y2={shape.y2} />
    {:else if shape.kind === 'circle'}
      <circle cx={shape.cx} cy={shape.cy} r={shape.r} />
    {:else}
      <path d={shape.d} />
    {/if}
  {/each}
</svg>
