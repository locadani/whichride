<script lang="ts">
  import { onMount, tick } from 'svelte';
  import CarSheet from './CarSheet.svelte';
  import { accented, format, orList, type Lang, type Messages } from '../i18n/format';
  import type { Engine, Priced, Ranking, Source } from '../lib/engine';
  import { browserStorage, recall, remember } from '../lib/memory';
  import { formatMoney } from '../lib/money';
  import { parseKilometres, parseMinutes } from '../lib/trip';

  let { lang, messages, providerNames }: { lang: Lang; messages: Messages; providerNames: string[] } = $props();

  /** The tariff name of plain per-km/per-minute pricing (the app's STANDARD_CARD). */
  const STANDARD_CARD = 'Standard';
  const t = (key: string, ...args: (string | number)[]) => format(messages[key], lang, ...args);

  let km = $state('');
  let minutes = $state('');
  let picked: string[] = $state([]);
  let engine: Engine | null = $state(null);
  let source: Source = $state('loading');
  let sheetOpen = $state(false);
  let addCars: HTMLButtonElement | undefined = $state();
  let addMore: HTMLButtonElement | undefined = $state();

  const metres = $derived(parseKilometres(km));
  const seconds = $derived(parseMinutes(minutes));
  const kmError = $derived(km.trim() !== '' && metres === null);
  const minutesError = $derived(minutes.trim() !== '' && seconds === null);
  const providers = $derived(engine?.providers() ?? []);
  const offerInfo = $derived(
    new Map(providers.flatMap((p) => p.offers.map((o) => [o.id, { provider: p.name, name: o.name }] as const))),
  );
  const providerName = (id: string) => providers.find((p) => p.id === id)?.name ?? id;
  /** Ticked offers the catalogue still has (memory may hold removed ones). */
  const chosen = $derived(picked.filter((id) => offerInfo.has(id)));
  const prices: Map<string, Priced> = $derived(
    engine && metres !== null && seconds !== null
      ? new Map(engine.offers(metres, seconds).map((o) => [o.offerId, o]))
      : new Map(),
  );
  const ranking: Ranking | null = $derived(
    engine && metres !== null && seconds !== null && chosen.length ? engine.options(metres, seconds, chosen) : null,
  );
  const rows = $derived(
    ranking
      ? ranking.ranked.map((o) => ({
          id: o.offerId,
          provider: providerName(o.providerId),
          offer: o.offerName,
          meta: o.cardLabel === STANDARD_CARD ? null : t('package_meta', o.cardLabel),
          cents: o.totalCents as number | null,
        }))
      : chosen.map((id) => ({ id, provider: offerInfo.get(id)!.provider, offer: offerInfo.get(id)!.name, meta: null, cents: null })),
  );
  const cheaperLine = $derived.by(() => {
    const cheaper = ranking?.cheaperNotAdded ?? [];
    if (cheaper.length === 0) return t('nothing_cheaper');
    const first = cheaper[0];
    return t('cheaper_not_added', cheaper.length, formatMoney(first.totalCents, lang), `${providerName(first.providerId)} ${first.offerName}`);
  });
  const [titleBefore, titleAccent, titleAfter] = $derived(accented(t('intro_title'), t('intro_title_accent')));

  function save() {
    remember(browserStorage(), { km, minutes, offers: [...picked] }, Date.now());
  }

  async function closeSheet(commit: string[] | null) {
    if (commit) {
      picked = commit;
      save();
    }
    sheetOpen = false;
    await tick();
    (chosen.length ? addMore : addCars)?.focus();
  }

  onMount(() => {
    const saved = recall(browserStorage(), Date.now());
    if (saved) {
      km = saved.km;
      minutes = saved.minutes;
      picked = saved.offers;
    }
    let cancelled = false;
    (async () => {
      const { seedEngine, serverEngine } = await import('../lib/engine');
      engine = await seedEngine();
      try {
        const server = await serverEngine();
        if (!cancelled) {
          engine = server;
          source = 'server';
        }
      } catch {
        if (!cancelled) source = 'built-in';
      }
    })();
    return () => {
      cancelled = true;
    };
  });
</script>

<div class="trip">
  <div class="field">
    <label for="trip-km">{t('manual_km')}</label>
    <input
      id="trip-km"
      inputmode="decimal"
      autocomplete="off"
      value={km}
      aria-invalid={kmError}
      aria-describedby={kmError ? 'km-error' : undefined}
      oninput={(e) => { km = e.currentTarget.value; save(); }}
    />
    {#if kmError}<span id="km-error" class="error">{t('manual_km_invalid')}</span>{/if}
  </div>
  <div class="field">
    <label for="trip-minutes">{t('manual_minutes')}</label>
    <input
      id="trip-minutes"
      inputmode="numeric"
      autocomplete="off"
      value={minutes}
      aria-invalid={minutesError}
      aria-describedby={minutesError ? 'minutes-error' : undefined}
      oninput={(e) => { minutes = e.currentTarget.value; save(); }}
    />
    {#if minutesError}<span id="minutes-error" class="error">{t('manual_minutes_invalid')}</span>{/if}
  </div>
</div>

<section class="result" aria-live="polite">
  {#if chosen.length === 0}
    <div class="intro">
      <h2>{titleBefore}<span class="accent">{titleAccent}</span>{titleAfter}</h2>
      <p>{t('intro_body', orList(providerNames, messages, lang))}</p>
      <button type="button" class="primary" bind:this={addCars} onclick={() => (sheetOpen = true)}>{t('add_cars')}</button>
    </div>
  {:else}
    <ol class="options">
      {#each rows as row, i (row.id)}
        <li class={['option', ranking && i === 0 && 'best']}>
          <div class="name">
            {#if ranking && i === 0}<span class="badge">{t('cheapest_of_options')}</span>{/if}
            <span class="provider">{row.provider}</span>
            <span class="meta">{row.offer}{row.meta ? ` · ${row.meta}` : ''}</span>
          </div>
          {#if row.cents !== null}<span class="price">{formatMoney(row.cents, lang)}</span>{/if}
        </li>
      {/each}
    </ol>
    {#if ranking}<p class="cheaper">{cheaperLine}</p>{/if}
    <button type="button" class="secondary" bind:this={addMore} onclick={() => (sheetOpen = true)}>{t('add_more_cars')}</button>
  {/if}
  {#if source !== 'server'}
    <p class="source">{t(source === 'loading' ? 'loading_prices' : 'built_in_prices')}</p>
  {/if}
</section>

<CarSheet
  open={sheetOpen}
  {providers}
  {prices}
  picked={chosen}
  {lang}
  {t}
  onCommit={(ids) => closeSheet(ids)}
  onCancel={() => closeSheet(null)}
/>

<style>
  .trip { display: grid; grid-template-columns: 1fr 1fr; gap: var(--wr-space-sm); margin-block-end: var(--wr-space-lg); }
  .field { display: flex; flex-direction: column; gap: var(--wr-space-xxs); font: var(--wr-type-label-medium); color: var(--wr-color-on-surface-muted); }
  .field input {
    min-inline-size: 0;
    min-block-size: var(--wr-size-control-md);
    padding-inline: var(--wr-space-sm);
    border: var(--wr-size-stroke) solid var(--wr-color-outline);
    border-radius: var(--wr-radius-md);
    background: var(--wr-color-surface);
    color: var(--wr-color-on-surface);
    font: var(--wr-type-number-medium);
  }
  .field input[aria-invalid='true'] { border-color: var(--wr-color-error); }
  .error { font: var(--wr-type-body-small); color: var(--wr-color-error); }
  .result { display: flex; flex-direction: column; gap: var(--wr-space-sm); }
  .intro { display: flex; flex-direction: column; gap: var(--wr-space-sm); padding: var(--wr-space-lg); border-radius: var(--wr-radius-lg); background: var(--wr-color-surface); }
  .intro h2 { margin: 0; font: var(--wr-type-headline-small); letter-spacing: var(--wr-type-headline-small-tracking); }
  .intro p { margin: 0; font: var(--wr-type-body-medium); color: var(--wr-color-on-surface-muted); }
  .options { margin: 0; padding: 0; list-style: none; display: flex; flex-direction: column; gap: var(--wr-space-xs); }
  .option { display: flex; justify-content: space-between; align-items: center; gap: var(--wr-space-sm); padding: var(--wr-space-sm) var(--wr-space-md); border-radius: var(--wr-radius-md); background: var(--wr-color-surface); }
  .option.best { padding-block: var(--wr-space-md); background: var(--wr-color-hero); color: var(--wr-color-on-hero); }
  .name { display: flex; flex-direction: column; gap: var(--wr-space-xxs); }
  .badge { font: var(--wr-type-label-small); letter-spacing: var(--wr-type-label-small-tracking); text-transform: uppercase; }
  .provider { font: var(--wr-type-title-medium); }
  .meta { font: var(--wr-type-body-small); }
  .price { font: var(--wr-type-number-medium); font-variant-numeric: tabular-nums; }
  .best .price { font: var(--wr-type-number-large); letter-spacing: var(--wr-type-number-large-tracking); }
  .cheaper, .source { margin: 0; font: var(--wr-type-body-small); color: var(--wr-color-on-surface-muted); }
  .primary, .secondary { min-block-size: var(--wr-size-control-md); padding-inline: var(--wr-space-lg); border-radius: var(--wr-radius-md); font: var(--wr-type-label-large); cursor: pointer; }
  .primary { border: none; background: var(--wr-color-accent); color: var(--wr-color-on-accent); }
  .secondary { align-self: flex-start; border: var(--wr-size-stroke) solid var(--wr-color-outline); background: none; color: var(--wr-color-on-surface); }
</style>
