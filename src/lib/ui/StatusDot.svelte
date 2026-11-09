<script lang="ts">
  import { statusName } from '../board'

  type Face = { shape: 'ring' | 'orbit' | 'disc' | 'pending'; colour: string; cut?: string }

  const IDLE: Face = { shape: 'ring', colour: 'var(--faint)' }

  const FACES: Record<string, Face> = {
    idle: IDLE,
    working: { shape: 'orbit', colour: 'var(--working)' },
    shell: { shape: 'orbit', colour: 'var(--working)' },
    waiting: { shape: 'disc', colour: 'var(--waiting)', cut: 'waiting' },
    failed: { shape: 'disc', colour: 'var(--failed)', cut: 'failed' },
    done: { shape: 'disc', colour: 'var(--done)', cut: 'done' },
    pending: { shape: 'pending', colour: 'var(--brand)' },
  }

  let { status, size = 14 }: { status?: string | null; size?: number } = $props()

  const id = $props.id()
  const cut = `cut-${id}`
  const face = $derived((status && FACES[status]) || IDLE)
  const name = $derived(statusName(status))
</script>

<svg
  class="dot"
  role="img"
  aria-label={name}
  width={size}
  height={size}
  viewBox="0 0 24 24"
  style:color={face.colour}
  fill="currentColor"
>
  {#if face.shape === 'ring'}
    <circle cx="12" cy="12" r="8.5" fill="none" stroke="currentColor" stroke-width="1.75" />
  {:else if face.shape === 'orbit'}
    <circle cx="12" cy="12" r="8.5" fill="none" stroke="currentColor" stroke-width="2" stroke-opacity="0.3" />
    <g class="orbit">
      <circle cx="3.5" cy="12" r="1.8" fill-opacity="0.25" />
      <circle cx="4.64" cy="7.75" r="2.3" fill-opacity="0.45" />
      <circle cx="7.75" cy="4.64" r="2.8" fill-opacity="0.7" />
      <circle cx="12" cy="3.5" r="3.4" />
    </g>
  {:else if face.shape === 'pending'}
    <circle cx="12" cy="12" r="8.5" fill="none" stroke="currentColor" stroke-width="1.75" stroke-opacity="0.35" />
    <circle cx="12" cy="12" r="4.5" />
  {:else}
    <mask id={cut}>
      <rect width="24" height="24" fill="white" />
      {#if face.cut === 'waiting'}
        <path d="M12 7v5.6" stroke="black" stroke-width="2.4" stroke-linecap="round" />
        <circle cx="12" cy="16.4" r="1.45" fill="black" />
      {:else if face.cut === 'failed'}
        <path d="M9 9l6 6M15 9l-6 6" stroke="black" stroke-width="2.3" stroke-linecap="round" />
      {:else}
        <path
          d="M8 12.4l2.8 2.8L16.2 9.6"
          fill="none"
          stroke="black"
          stroke-width="2.3"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      {/if}
    </mask>
    <circle cx="12" cy="12" r="9.5" mask="url(#{cut})" />
  {/if}
</svg>

<style>
  .dot {
    flex: none;
  }

  .orbit {
    transform-origin: 12px 12px;
    animation: orbit 1200ms linear infinite;
  }

  @keyframes orbit {
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .orbit {
      animation: none;
    }
  }
</style>
