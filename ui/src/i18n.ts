import { invoke, isTauri } from '@tauri-apps/api/core';
import {useSyncExternalStore} from 'react';
import {messages} from './messages';
import {translateDiagnostic} from './diagnostic-text';

export type Language = 'en' | 'zh-CN' | 'ja';
const storageKey = 'island-walkman.language';
export function normalizeLanguage(value: unknown): Language {
    return value === 'zh-CN' || value === 'ja' ? value : 'en';
}
let language: Language = 'en';
try { language = normalizeLanguage(localStorage.getItem(storageKey)); } catch { /* Private storage can be unavailable. */ }
const listeners = new Set<() => void>();
function updateDocument() {
    document.documentElement.lang = language;
    document.title = language === 'zh-CN' ? '唱片小岛 · Island Walkman' : language === 'ja' ? '音楽の島 · Island Walkman' : 'Island Walkman';
}
updateDocument();
export function setLanguage(value: Language) {
    language = normalizeLanguage(value);
    try { localStorage.setItem(storageKey, language); } catch { /* Switching still works without persistence. */ }
    if (isTauri()) void invoke("set_language", { language }).catch(() => {});
    updateDocument();
    listeners.forEach(listener => listener());
}
export function translate(locale: Language, key: string, ...values: unknown[]): string {
    const entry = messages[key];
    const text = locale === 'zh-CN' ? (key === 'Language' ? '界面语言' : entry?.[2] ?? key) : entry?.[locale === 'ja' ? 1 : 0] ?? key;
    return text.replace(/\{(\d+)\}/g, (_, index: string) => String(values[Number(index)] ?? ''));
}
export function tr(key: string, ...values: unknown[]): string { return translate(language, key, ...values); }
export function diagnostic(text: string): string { return translateDiagnostic(language,text); }
export function useLanguage(): readonly [Language, typeof setLanguage] {
    const locale = useSyncExternalStore(listener => { listeners.add(listener); return () => { listeners.delete(listener); }; }, () => language);
    return [locale, setLanguage];
}

export async function restoreDesktopLanguage() {
    if (isTauri()) {
        try { const saved = await invoke<string | null>("get_language"); if (saved) setLanguage(normalizeLanguage(saved)); } catch { /* English remains available. */ }
    }
}
