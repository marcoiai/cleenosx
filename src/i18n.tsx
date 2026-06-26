import { createContext, useContext, useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";

export type Language = "en" | "pt-BR";

type TranslationValues = Record<string, string | number>;
type I18nContextValue = {
  language: Language;
  setLanguage: (language: Language) => void;
  t: (key: string, values?: TranslationValues) => string;
};

const LANGUAGE_STORAGE_KEY = "cleanerx.language.v1";

const languages: Array<{ value: Language; label: string }> = [
  { value: "en", label: "English" },
  { value: "pt-BR", label: "Português (Brasil)" },
];

const en: Record<string, string> = {
  "app.tagline": "Real cleanup",
  "common.scan": "Scan",
  "common.cancel": "Cancel",
  "common.enabled": "Enabled",
  "common.open": "Open",
  "common.clear": "Clear",
  "common.showScript": "Show Script",
  "common.unknown": "Unknown",
  "common.noVolumeData": "No volume data yet.",
  "nav.dashboard": "Dashboard",
  "nav.volumes": "Volumes",
  "nav.scanner": "Scan & Clear",
  "nav.findings": "Findings",
  "nav.recovery": "Recovery",
  "nav.settings": "Settings",
  "view.dashboard.title": "Dashboard",
  "view.dashboard.subtitle": "Storage summary, APFS roles, warnings, and scan status.",
  "view.volumes.title": "Volumes / Partitions",
  "view.volumes.subtitle": "Mounted and unmounted volume hints from macOS tools.",
  "view.scanner.title": "Scan & Clear",
  "view.scanner.subtitle": "Drill down, select exact files or directories, prepare a plan, then confirm deletion.",
  "view.findings.title": "Findings",
  "view.findings.subtitle": "Assets, snapshots, Rust targets (`target/`) and related build caches, developer tools, containers, plus risk labels.",
  "view.recovery.title": "Recovery",
  "view.recovery.subtitle": "Generated companion script for Recovery workflows.",
  "view.settings.title": "Settings",
  "view.settings.subtitle": "Preferences, language, and cleanup behavior.",
  "dashboard.total": "Total",
  "dashboard.used": "Used",
  "dashboard.free": "Free",
  "dashboard.apfsVolumes": "APFS Volumes",
  "dashboard.primaryStorage": "Primary Storage",
  "dashboard.detectedRoles": "Detected Roles",
  "dashboard.noRoles": "No volume roles parsed yet.",
  "scanStatus.loadingOverview": "Loading lightweight overview",
  "scanStatus.ready": "Ready",
  "scanStatus.deepScanRunning": "Deep scan running",
  "scanStatus.deepScanPartial": "Deep scan partial",
  "scanStatus.deepScanCanceled": "Deep scan canceled",
  "scanStatus.deepScanFailed": "Deep scan failed",
  "scanStatus.cancelScan": "Cancel scan",
  "scanStatus.loadingOverviewDetail": "Reading volumes, partitions, df and APFS metadata...",
  "scanStatus.deepScanDetail": "Scanning selected path with du. Large folders can take a while; the process is cancellable by timeout.",
  "scanStatus.openFullDiskAccess": "Open Full Disk Access",
  "settings.title": "Settings",
  "settings.subtitle": "Cleanup behavior and safety rules.",
  "settings.language.title": "Language",
  "settings.language.description": "Choose the interface language for CleanerX.",
  "settings.theme.title": "Black Mode",
  "settings.theme.description": "Use the dark interface by default for long scanning and cleanup sessions.",
  "settings.projectCleanup.title": "Project Folder Cleanup",
  "settings.projectCleanup.description": "Project roots under paths like `/Users/.../Projects` are blocked by default so source folders are not removed by accident. Build artifacts such as `target/` can still be selected.",
  "settings.projectCleanup.allow": "Allow project roots",
  "settings.projectCleanup.warning": "Whole project folders can now be prepared for deletion. Use the final confirmation flow carefully.",
  "settings.adminMode.title": "Admin Mode",
  "settings.adminMode.unlock": "Unlock Once",
  "settings.adminMode.disable": "Disable",
  "settings.adminMode.authorizing": "Authorizing...",
  "settings.adminMode.off": "Admin Mode is off. Unlock once to reuse administrator cleanup through this app session.",
  "settings.adminMode.on": "Admin Mode is enabled for this app session. CleanerX will prefer administrator cleanup when available.",
  "settings.adminMode.unavailable": "Admin Mode is unavailable in the App Store build.",
  "recovery.oneExecutable": "One Recovery Executable",
  "recovery.description": "Creates an easy Recovery shortcut at `CleanerX/cx.sh` on the Data volume, plus home, Shared, and Desktop copies when possible.",
  "recovery.createShortcut": "Create Recovery Shortcut",
  "recovery.created": "Created",
  "recovery.terminal": "Recovery Terminal",
  "recovery.preview": "Preview",
  "recovery.showScript": "Show Script",
  "recovery.emptyPreview": "Create the executable first, or show the script for review.",
  "recovery.stepsTitle": "Before you paste",
  "recovery.steps1": "1. Boot into macOS Recovery and open Terminal.",
  "recovery.steps2": "2. Paste the command below. If the Data volume is already mounted, CleanerX starts immediately.",
  "recovery.steps3": "3. If the command says the script was not found, open Disk Utility in Recovery, unlock/mount the Data volume, then paste the same command again.",
  "recovery.pasteHelp": "This bootstrap command tries the common CleanerX script locations first, then tells you exactly what to mount if nothing is visible yet.",
  "logs.title": "Logs",
  "logs.empty": "No logs yet.",
  "volumes.title": "Volumes / Partitions",
  "volumes.name": "Name",
  "volumes.identifier": "Identifier",
  "volumes.role": "Role",
  "volumes.mountPoint": "Mount Point",
  "volumes.used": "Used",
  "volumes.available": "Available",
  "volumes.risk": "Risk",
  "volumes.action": "Action",
  "volumes.locked": "Locked / FileVault",
  "volumes.notMounted": "Not mounted",
  "volumes.scanMountPoint": "Scan mount point",
  "findings.title": "Findings",
  "findings.empty": "No findings yet.",
  "findings.scanTitle": "Scan this path",
};

const ptBr: Record<string, string> = {
  "app.tagline": "Limpeza real",
  "common.scan": "Escanear",
  "common.cancel": "Cancelar",
  "common.enabled": "Ativado",
  "common.open": "Abrir",
  "common.clear": "Limpar",
  "common.showScript": "Mostrar script",
  "common.unknown": "Desconhecido",
  "common.noVolumeData": "Ainda sem dados de volumes.",
  "nav.dashboard": "Dashboard",
  "nav.volumes": "Volumes",
  "nav.scanner": "Escanear & Limpar",
  "nav.findings": "Achados",
  "nav.recovery": "Recovery",
  "nav.settings": "Ajustes",
  "view.dashboard.title": "Dashboard",
  "view.dashboard.subtitle": "Resumo de armazenamento, papeis APFS, avisos e status de scan.",
  "view.volumes.title": "Volumes / Partições",
  "view.volumes.subtitle": "Pistas de volumes montados e desmontados pelas ferramentas do macOS.",
  "view.scanner.title": "Escanear & Limpar",
  "view.scanner.subtitle": "Entre nas pastas, selecione arquivos ou diretórios exatos, prepare um plano e confirme a exclusão.",
  "view.findings.title": "Achados",
  "view.findings.subtitle": "Assets, snapshots, targets Rust (`target/`), caches de build, ferramentas de dev, containers e rótulos de risco.",
  "view.recovery.title": "Recovery",
  "view.recovery.subtitle": "Script auxiliar gerado para fluxos no Recovery.",
  "view.settings.title": "Ajustes",
  "view.settings.subtitle": "Preferências, idioma e comportamento de limpeza.",
  "dashboard.total": "Total",
  "dashboard.used": "Usado",
  "dashboard.free": "Livre",
  "dashboard.apfsVolumes": "Volumes APFS",
  "dashboard.primaryStorage": "Armazenamento principal",
  "dashboard.detectedRoles": "Papeis detectados",
  "dashboard.noRoles": "Nenhum papel de volume lido ainda.",
  "scanStatus.loadingOverview": "Carregando resumo leve",
  "scanStatus.ready": "Pronto",
  "scanStatus.deepScanRunning": "Scan profundo rodando",
  "scanStatus.deepScanPartial": "Scan profundo parcial",
  "scanStatus.deepScanCanceled": "Scan profundo cancelado",
  "scanStatus.deepScanFailed": "Scan profundo falhou",
  "scanStatus.cancelScan": "Cancelar scan",
  "scanStatus.loadingOverviewDetail": "Lendo volumes, partições, df e metadados APFS...",
  "scanStatus.deepScanDetail": "Escaneando o caminho selecionado com du. Pastas grandes podem demorar; o processo pode ser cancelado por timeout.",
  "scanStatus.openFullDiskAccess": "Abrir Full Disk Access",
  "settings.title": "Ajustes",
  "settings.subtitle": "Comportamento de limpeza e regras de segurança.",
  "settings.language.title": "Idioma",
  "settings.language.description": "Escolha o idioma da interface do CleanerX.",
  "settings.theme.title": "Modo preto",
  "settings.theme.description": "Use a interface escura por padrão em sessões longas de scan e limpeza.",
  "settings.projectCleanup.title": "Limpeza de pastas de projeto",
  "settings.projectCleanup.description": "Raízes de projeto em caminhos como `/Users/.../Projects` são bloqueadas por padrão para não remover código sem querer. Artefatos de build como `target/` ainda podem ser selecionados.",
  "settings.projectCleanup.allow": "Permitir raízes de projeto",
  "settings.projectCleanup.warning": "Pastas inteiras de projeto agora podem ser preparadas para exclusão. Use o fluxo de confirmação final com cuidado.",
  "settings.adminMode.title": "Modo Admin",
  "settings.adminMode.unlock": "Desbloquear uma vez",
  "settings.adminMode.disable": "Desativar",
  "settings.adminMode.authorizing": "Autorizando...",
  "settings.adminMode.off": "O Modo Admin está desligado. Desbloqueie uma vez para reutilizar a limpeza com privilégios nesta sessão do app.",
  "settings.adminMode.on": "O Modo Admin está ligado nesta sessão do app. O CleanerX vai preferir limpeza com administrador quando disponível.",
  "settings.adminMode.unavailable": "O Modo Admin não está disponível na build da App Store.",
  "recovery.oneExecutable": "Um executável Recovery",
  "recovery.description": "Cria um atalho fácil em `CleanerX/cx.sh` no volume Data, além de cópias em home, Shared e Desktop quando possível.",
  "recovery.createShortcut": "Criar atalho Recovery",
  "recovery.created": "Criado",
  "recovery.terminal": "Terminal Recovery",
  "recovery.preview": "Preview",
  "recovery.showScript": "Mostrar script",
  "recovery.emptyPreview": "Crie o executável primeiro ou mostre o script para revisão.",
  "recovery.stepsTitle": "Antes de colar",
  "recovery.steps1": "1. Entre no macOS Recovery e abra o Terminal.",
  "recovery.steps2": "2. Cole o comando abaixo. Se o volume Data já estiver montado, o CleanerX abre na hora.",
  "recovery.steps3": "3. Se o comando disser que não encontrou o script, abra o Utilitário de Disco no Recovery, desbloqueie/monte o volume Data e cole o mesmo comando de novo.",
  "recovery.pasteHelp": "Esse comando bootstrap tenta primeiro os locais mais comuns do script do CleanerX e, se não achar nada, diz exatamente o que falta montar.",
  "logs.title": "Logs",
  "logs.empty": "Ainda sem logs.",
  "volumes.title": "Volumes / Partições",
  "volumes.name": "Nome",
  "volumes.identifier": "Identificador",
  "volumes.role": "Papel",
  "volumes.mountPoint": "Ponto de montagem",
  "volumes.used": "Usado",
  "volumes.available": "Disponível",
  "volumes.risk": "Risco",
  "volumes.action": "Ação",
  "volumes.locked": "Bloqueado / FileVault",
  "volumes.notMounted": "Não montado",
  "volumes.scanMountPoint": "Escanear ponto de montagem",
  "findings.title": "Achados",
  "findings.empty": "Ainda sem achados.",
  "findings.scanTitle": "Escanear este caminho",
};

const dictionaries: Record<Language, Record<string, string>> = {
  en,
  "pt-BR": ptBr,
};

const I18nContext = createContext<I18nContextValue | null>(null);

export function I18nProvider({ children }: { children: ReactNode }) {
  const [language, setLanguageState] = useState<Language>(() => readLanguage());

  useEffect(() => {
    try {
      localStorage.setItem(LANGUAGE_STORAGE_KEY, language);
    } catch {
      // Ignore storage failures for the current session.
    }
  }, [language]);

  const value = useMemo<I18nContextValue>(() => {
    const dictionary = dictionaries[language];
    return {
      language,
      setLanguage: setLanguageState,
      t: (key, values) => interpolate(dictionary[key] ?? en[key] ?? key, values),
    };
  }, [language]);

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n() {
  const context = useContext(I18nContext);
  if (!context) {
    throw new Error("useI18n must be used inside I18nProvider");
  }
  return context;
}

export function languageOptions() {
  return languages;
}

function readLanguage(): Language {
  try {
    const stored = localStorage.getItem(LANGUAGE_STORAGE_KEY);
    if (stored === "en" || stored === "pt-BR") return stored;
  } catch {
    // Fall back to browser language.
  }

  const browserLanguage = navigator.language.toLowerCase();
  return browserLanguage.startsWith("pt") ? "pt-BR" : "en";
}

function interpolate(message: string, values?: TranslationValues) {
  if (!values) return message;
  return message.replace(/\{(\w+)\}/g, (match, key) => String(values[key] ?? match));
}
