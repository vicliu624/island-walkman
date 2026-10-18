# Frontend

React 18 and Animal Island UI 2.2.1 provide the interface inside a Tauri 2 desktop window. Production assets are served from `ui/dist` or the installed package resources.

```sh
npm ci
npm test
npm run build
```

The build validates translation coverage and interpolation placeholders, checks strict TypeScript and produces Vite assets. Development uses `npm run dev`; API calls still require the local Rust service.

## Languages

English is the initial language. A top-bar picker offers English, Simplified Chinese and Japanese. Desktop preferences persist in `desktop-preferences.json` beside the catalog, independently of the internal service port. The optional headless interface uses browser storage key `island-walkman.language`. Invalid or unavailable storage falls back to English. Music metadata and raw OS error details are not translated.

Native dialogs select songs, folders and artwork. `src/updates.tsx` uses Tauri's updater and process plugins for explicit update checking and installation from signed GitHub Release assets. Native permissions are restricted to the active window and its exact loopback origin.

`src/messages.ts` contains UI messages. Keys preserve original wording, with English/Japanese translations and Chinese source text. An optional third value overrides Chinese wording when help text changes without changing a source key. `src/i18n.ts` handles interpolation, storage and reactive changes. `tools/check-ui.mjs` checks coverage and rejects untranslated Chinese JSX except language-picker endonyms.

## Behavior and verification

Song tables have 100 rows per page; collection grids have 48 cards. Header selection applies to the page and persists across pages. Album ownership distinguishes same-named albums. Edits update the application catalog, not original MP3 files.

Transfer controls follow the backend write gate. UI settings cannot bypass it. Local preparation does not require a connected player. The A918 artwork probe succeeded, while full native transfers remain under investigation because of device I/O errors.

Browser layout was previously checked at narrow and desktop widths. Translated labels, dialogs and persistence require browser checks after localization changes; dictionary checks alone are insufficient.
