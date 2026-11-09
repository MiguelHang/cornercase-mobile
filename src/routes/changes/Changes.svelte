<script lang="ts">
  import Icon from '../../lib/ui/Icon.svelte'

  let {
    title,
    onback,
  }: {
    title: string
    onback: () => void
  } = $props()
</script>

<section class="screen">
  <header class="top">
    <button class="back" type="button" aria-label="Back" onclick={onback} ontouchstart={() => {}}>
      <Icon name="chevron-left" size={24} />
    </button>
    <h1 class="title">{title}</h1>
  </header>
</section>

<style>
  .screen {
    display: flex;
    flex-direction: column;
    gap: var(--inset-wide);
    padding: calc(env(safe-area-inset-top) + var(--gap)) var(--inset-wide) 40px;
  }

  .top {
    display: flex;
    align-items: center;
    gap: var(--gap);
    min-width: 0;
    margin-left: calc(-1 * var(--inset));
  }

  .back {
    display: flex;
    flex: none;
    align-items: center;
    justify-content: center;
    width: 44px;
    height: 44px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-control);
    background: none;
    color: var(--accent);
  }

  .back:active {
    background: var(--hover);
  }

  .title {
    margin: 0;
    overflow: hidden;
    font-size: 22px;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
