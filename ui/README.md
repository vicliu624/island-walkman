# Frontend

React 18 and Animal Island UI 2.2.1 provide a local browser interface to the Rust service. Production assets are served from `ui/dist` or the installed package resources.

```sh
npm ci
npm test
npm run build
```

The build validates translation coverage and interpolation placeholders, checks strict TypeScript and produces Vite assets. Development uses `npm run dev`; API calls still require the local Rust service.

## Languages

English is the initial language. A top-bar picker offers English, Simplified Chinese and Japanese. Its browser storage key is `island-walkman.language`. Invalid or unavailable storage falls back to English. The document language and title follow the selected locale. Music metadata, filesystem paths and raw OS error details are not translated.

`src/messages.ts` contains UI messages. Keys preserve original wording, with English/Japanese translations and Chinese source text. `src/i18n.ts` handles interpolation, storage and reactive changes. `tools/check-ui.mjs` checks coverage and rejects untranslated Chinese JSX except language-picker endonyms.

## Behavior and verification

Song tables have 100 rows per page; collection grids have 48 cards. Header selection applies to the page and persists across pages. Album ownership distinguishes same-named albums. Edits update the application catalog, not original MP3 files.

Transfer controls follow the backend write gate. UI settings cannot bypass it. Local preparation does not require a connected player. The A918 artwork probe succeeded, while full native transfers remain under investigation because of device I/O errors.

Browser layout was previously checked at narrow and desktop widths. Translated labels, dialogs and persistence require browser checks after localization changes; dictionary checks alone are insufficient.
