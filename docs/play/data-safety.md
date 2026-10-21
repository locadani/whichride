# Play Console: Data safety answers

What to enter in Play Console → *App content* → *Data safety*. Every answer comes from the code
and spec §9; if the code changes what leaves the phone, update this file in the same PR.

## What leaves the phone

| Data | Sent to | Kept | Code |
|---|---|---|---|
| Start and destination coordinates, typed address text | whichride server → OpenRouteService (HeiGIT, Germany) | Not stored or logged; in the request body, never the URL | `crates/server/src/routing.rs`, `ors.rs` |
| Install ID (random UUID) and its token | whichride server | Database, until deleted by the owner | `installs` table |
| Price reports (including names of proposed cars) and yes/no confirmations | whichride server | Database, with the install ID | `reports`, `votes` tables |
| Play Integrity token | whichride server → Google | Not stored | `crates/server/src/integrity.rs` |
| IP address, user agent, URL path | Google Cloud Run request logs | Cloud Logging's default retention (30 days) | Not app code: Cloud Run |

Personal discounts and passes, the app chosen per provider and cached prices stay on the phone.
Database backups (`infra/README.md`, *Backups*) hold the same tables for 30 days.

## Answers

**Does your app collect or share any of the required user data types?** Yes.

**Is all of the user data collected by your app encrypted in transit?** Yes (HTTPS only; the
release build refuses cleartext).

**Do you provide a way for users to request that their data is deleted?** No in-app way yet.
The install ID is random and not linked to a person; uninstalling deletes everything on the
phone. Owner decision before production: offer deletion by e-mail, or answer "No".

| Play data type | Collected | Shared | Ephemeral | Required | Purposes |
|---|---|---|---|---|---|
| Location → Precise location | Yes | No | Yes | No (the user can type an address or km/min) | App functionality |
| Device or other IDs | Yes | No | No | Yes | App functionality; Fraud prevention, security and compliance |
| User-generated content → Other | Yes (price reports, confirmations) | No | No | No | App functionality |

Not collected: personal info (name, e-mail…), financial info, health, messages, photos, audio,
files, calendar, contacts, web history, app activity, app info and performance (no crash
reporting or analytics).

"Shared" is No for all: OpenRouteService and Google process data on the developer's behalf
(service providers), which Play does not count as sharing.

## Other App content pages

- **Ads:** no ads.
- **Target audience:** 18+ (driving).
- **Content rating questionnaire:** category *Utility, productivity, communication or other*;
  answer No to every content question except "users can interact or exchange content": a
  proposed car name ("Car not listed? Add it") is free text other users see, so answer Yes.
- **Government app / financial features / health:** No.
- **Privacy policy URL:** required before the closed test; not written yet.
