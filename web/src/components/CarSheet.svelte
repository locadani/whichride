<script lang="ts">
  import type { Lang } from '../i18n/format';
  import type { Priced, ProviderInfo } from '../lib/engine';
  import { formatMoney } from '../lib/money';

  let {
    open, providers, prices, picked, lang, t, onCommit, onCancel,
  }: {
    open: boolean;
    providers: ProviderInfo[];
    prices: Map<string, Priced>;
    picked: string[];
    lang: Lang;
    t: (key: string, ...args: (string | number)[]) => string;
    onCommit: (ids: string[]) => void;
    onCancel: () => void;
  } = $props();

  let dialog: HTMLDialogElement | undefined = $state();
  // The ticks are a draft until the button applies them (as the app's sheet).
  let draft: string[] = $state([]);

  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) {
      draft = [...picked];
      dialog.showModal();
    } else if (!open && dialog.open) {
      dialog.close();
    }
  });

  function toggle(id: string, on: boolean) {
    draft = on ? [...draft, id] : draft.filter((d) => d !== id);
  }
</script>

<dialog
  bind:this={dialog}
  aria-labelledby="sheet-title"
  aria-describedby="sheet-body"
  oncancel={(e) => { e.preventDefault(); onCancel(); }}
>
  <div class="head">
    <h2 id="sheet-title">{t('intro_title')}</h2>
    <button type="button" class="close" aria-label={t('close')} onclick={onCancel}>✕</button>
  </div>
  <p id="sheet-body" class="body">{t('which_cars_body')}</p>
  {#each providers as provider (provider.id)}
    <fieldset>
      <legend>{provider.name}</legend>
      {#each provider.offers as offer (offer.id)}
        {@const price = prices.get(offer.id)}
        <label class="car">
          <input
            type="checkbox"
            value={offer.id}
            checked={draft.includes(offer.id)}
            onchange={(e) => toggle(offer.id, e.currentTarget.checked)}
          />
          <span class="name">{offer.name}</span>
          {#if price}<span class="price">{formatMoney(price.totalCents, lang)}</span>{/if}
        </label>
      {/each}
    </fieldset>
  {/each}
  <button type="button" class="primary" onclick={() => onCommit(draft)}>
    {draft.length ? t('add_n_cars', draft.length) : t('done')}
  </button>
</dialog>

<style>
  dialog {
    inline-size: 100%;
    max-inline-size: var(--wr-size-content-max);
    max-block-size: 85dvh;
    margin-block-end: 0;
    padding: var(--wr-space-md);
    border: none;
    border-start-start-radius: var(--wr-radius-xxl);
    border-start-end-radius: var(--wr-radius-xxl);
    background: var(--wr-color-surface);
    color: var(--wr-color-on-surface);
  }
  dialog::backdrop { background: var(--wr-color-scrim); opacity: 0.7; }
  .head { display: flex; justify-content: space-between; align-items: center; gap: var(--wr-space-sm); }
  h2 { margin: 0; font: var(--wr-type-headline-small); letter-spacing: var(--wr-type-headline-small-tracking); }
  .body { margin-block: var(--wr-space-xs) var(--wr-space-md); font: var(--wr-type-body-medium); color: var(--wr-color-on-surface-muted); }
  .close {
    inline-size: var(--wr-size-touch-target);
    block-size: var(--wr-size-touch-target);
    border: none;
    border-radius: var(--wr-radius-full);
    background: var(--wr-color-surface-high);
    color: var(--wr-color-on-surface);
    font: var(--wr-type-title-medium);
    cursor: pointer;
  }
  fieldset { margin: 0 0 var(--wr-space-md); padding: 0; border: none; display: flex; flex-direction: column; gap: var(--wr-space-xxs); }
  legend { margin-block-end: var(--wr-space-xs); font: var(--wr-type-label-small); letter-spacing: var(--wr-type-label-small-tracking); text-transform: uppercase; color: var(--wr-color-on-surface-muted); }
  .car {
    display: flex; align-items: center; gap: var(--wr-space-sm);
    min-block-size: var(--wr-size-touch-target);
    padding-inline: var(--wr-space-sm);
    border-radius: var(--wr-radius-md);
    background: var(--wr-color-surface-high);
    cursor: pointer;
  }
  .car input { inline-size: var(--wr-space-lg); block-size: var(--wr-space-lg); margin: 0; accent-color: var(--wr-color-accent); }
  .name { flex: 1; font: var(--wr-type-title-small); }
  .price { font: var(--wr-type-number-medium); font-variant-numeric: tabular-nums; }
  .primary {
    position: sticky; inset-block-end: 0;
    inline-size: 100%; min-block-size: var(--wr-size-control-lg);
    border: none; border-radius: var(--wr-radius-md);
    background: var(--wr-color-accent); color: var(--wr-color-on-accent);
    font: var(--wr-type-label-large); cursor: pointer;
  }
</style>
