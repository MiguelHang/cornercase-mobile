<script lang="ts">
  import Icon from './Icon.svelte'
  import type { IconName } from './icons/shapes'

  let { icon, title, lines }: { icon: IconName; title: string; lines: string[] } = $props()
</script>

<div class="page">
  <div class="disc">
    <Icon name={icon} size={24} />
  </div>
  <h2 class="title">{title}</h2>
  {#each lines as line, index (index)}
    <p class="line">{line}</p>
  {/each}
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 40px 28px 28px;
    text-align: center;
  }

  .disc {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 52px;
    height: 52px;
    margin-bottom: 4px;
    border: 1px solid rgb(43 123 255 / 0.35);
    border-radius: 50%;
    background: rgb(0 95 255 / 0.16);
    color: var(--accent);
  }

  .title {
    margin: 0;
    font-size: 20px;
    font-weight: 600;
  }

  .line {
    max-width: 470px;
    margin: 0;
    color: var(--muted);
    font-size: 16px;
    line-height: 1.4;
  }
</style>
