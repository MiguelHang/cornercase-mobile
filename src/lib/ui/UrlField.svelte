<script lang="ts">
  import * as barcode from '@tauri-apps/plugin-barcode-scanner'
  import { pageClasses, Scanner } from '../scanner.svelte'
  import Button from './Button.svelte'
  import Field from './Field.svelte'
  import Icon from './Icon.svelte'
  import ScanOverlay from './ScanOverlay.svelte'

  let {
    value = $bindable(''),
    error = null,
    disabled = false,
    onscanned,
  }: {
    value?: string
    error?: string | null
    disabled?: boolean
    onscanned: (url: string) => void
  } = $props()

  const scanner = new Scanner(barcode, pageClasses)

  $effect(() => () => void scanner.stop())

  const scan = async () => {
    const url = await scanner.scan()
    if (url !== null) {
      onscanned(url)
    }
  }
</script>

<div class="url">
  <Button tone="plain" resting={disabled} onclick={() => void scan()}>
    <Icon name="scan-qr-code" size={18} />
    Scan QR code
  </Button>
  <ScanOverlay {scanner} />
  <Field label="Bridge URL" url bind:value {error} {disabled} />
</div>

<style>
  .url {
    display: flex;
    flex-direction: column;
    gap: var(--gap);
  }
</style>
