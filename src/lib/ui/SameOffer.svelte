<script lang="ts">
  import { NEW_MACHINE, UNCHECKED, offerText, type Offer } from '../machineForm'
  import Button from './Button.svelte'

  let {
    offer,
    busy,
    onroute,
    onnew,
    onback,
  }: { offer: Offer; busy: boolean; onroute: () => void; onnew: () => void; onback: () => void } = $props()

  const text = $derived(offerText(offer))
</script>

<div class="offer" role="group" aria-label="Same machine">
  <p class="reason">{text.reason}</p>
  {#if !offer.checked}
    <p class="note">{UNCHECKED}</p>
  {/if}
  <Button {busy} onclick={onroute}>{text.keep}</Button>
  <Button tone="plain" resting={busy} onclick={onnew}>{NEW_MACHINE}</Button>
  <Button tone="plain" resting={busy} onclick={onback}>Back</Button>
</div>

<style>
  .offer {
    display: flex;
    flex-direction: column;
    gap: var(--gap);
    padding-top: var(--gap);
  }

  .reason {
    margin: 0;
    color: var(--text);
    font-size: 15px;
  }

  .note {
    margin: 0;
    color: var(--soft);
    font-size: 13px;
  }
</style>
