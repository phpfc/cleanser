export type Language = "en" | "pt-BR";

export const translations = {
  "en": {
    // App
    appTagline: "The crab that cleans your disk",
    readyToClean: "Ready to clean up the mess?",
    clickToScan: "Click \"Scan\" to find files that can be removed",

    // Scan
    scan: "Scan",
    scanning: "Scanning...",
    speed: "Speed:",
    quick: "Quick",
    normal: "Normal",
    thorough: "Thorough",
    clearResults: "Clear results",

    // Results
    found: "Found:",
    items: "items",
    item: "item",
    inCategories: "in {count} categories",
    sortBySize: "Sort by size",
    sortByCount: "Sort by count",
    selectAll: "Select All",
    deselectAll: "Deselect All",
    selected: "Selected:",
    cleanSelected: "Clean Selected",
    cleaning: "Cleaning...",

    // Risk levels
    safe: "Safe",
    moderate: "Moderate",
    risky: "Risky",

    // Clean result
    cleanComplete: "Cleaning Complete!",
    itemsCleaned: "{count} items cleaned, {size} freed",
    itemsFailed: "{count} items failed",

    // Actions
    addToWhitelist: "Add to whitelist",
    openInFileManager: "Open in file manager",
    removeFromWhitelist: "Remove from whitelist",

    // Settings
    settings: "Settings",
    scanSettings: "Scan Settings",
    minFileSize: "Minimum file size (MB)",
    minFileSizeDescription: "Only detect files larger than this size",
    findDuplicates: "Find duplicate files",
    findDuplicatesDescription: "Scan for duplicate files (slower)",
    systemInfo: "System Information",
    platform: "Platform:",
    home: "Home:",
    protectedPaths: "Protected Paths (Whitelist)",
    whitelistDescription: "These paths will be excluded from scans and cleaning.",
    pathPlaceholder: "/path/to/protect",
    add: "Add",
    noWhitelistPaths: "No paths in whitelist",
    language: "Language",
    theme: "Theme",
    themeSystem: "System",
    themeLight: "Light",
    themeDark: "Dark",

    // Map
    filesystemMap: "Filesystem Map",
    intelligentMapping: "Intelligent directory mapping",
    rebuildMap: "Rebuild Map",
    rebuilding: "Rebuilding...",
    createMap: "Create Map",
    mapping: "Mapping...",
    noMapFound: "No map found",
    createMapDescription: "Create a system map for faster scans",
    totalMapped: "Total Mapped",
    directories: "directories",
    cleanable: "Cleanable",
    lastScan: "Last Scan",
    outdated: "Outdated (>7 days)",
    byCategory: "By Category",
    topTypes: "Top 10 Types",

    // Navigation
    scanTab: "Scan",
    mapTab: "Map",

    // Loading
    loading: "Loading...",

    // Version
    updateAvailable: "Update available",
    currentVersion: "Current: v{version}",
    latestVersion: "Latest: v{version}",
    downloadUpdate: "Download",

    // Trash
    trashTab: "Trash",
    trashEmpty: "Trash is empty",
    trashEmptyDescription: "Deleted items will appear here when using --trash mode",
    trashItems: "{count} items",
    trashSize: "{size} total",
    restore: "Restore",
    deleteForever: "Delete Forever",
    emptyTrash: "Empty Trash",
    emptyTrashConfirm: "Permanently delete all {count} items?",
    restored: "Restored successfully",
    deleted: "Deleted permanently",
    trashEmptied: "Trash emptied",

    // Schedule
    scheduleTab: "Schedule",
    noScheduledJobs: "No scheduled jobs",
    noScheduledJobsDescription: "Create automated cleanup schedules",
    createJob: "Create Job",
    jobName: "Job Name",
    frequency: "Frequency",
    riskLevel: "Risk Level",
    useTrash: "Move to trash",
    secureDelete: "Secure delete",
    notifyOnComplete: "Notify when complete",
    enabled: "Enabled",
    disabled: "Disabled",
    lastRun: "Last run",
    never: "Never",
    enable: "Enable",
    disable: "Disable",
    remove: "Remove",
    jobCreated: "Job created",
    jobRemoved: "Job removed",
    jobEnabled: "Job enabled",
    jobDisabled: "Job disabled",
    frequencyDaily: "Daily",
    frequencyWeekly: "Weekly",
    frequencyMonthly: "Monthly",
  },
  "pt-BR": {
    // App
    appTagline: "O caranguejo que limpa seu disco",
    readyToClean: "Pronto pra limpar a bagunça?",
    clickToScan: "Clique em \"Escanear\" para encontrar arquivos que podem ser removidos",

    // Scan
    scan: "Escanear",
    scanning: "Escaneando...",
    speed: "Velocidade:",
    quick: "Rápido",
    normal: "Normal",
    thorough: "Completo",
    clearResults: "Limpar resultados",

    // Results
    found: "Encontrado:",
    items: "itens",
    item: "item",
    inCategories: "em {count} categorias",
    sortBySize: "Ordenar por tamanho",
    sortByCount: "Ordenar por quantidade",
    selectAll: "Selecionar Tudo",
    deselectAll: "Desmarcar Tudo",
    selected: "Selecionado:",
    cleanSelected: "Limpar Selecionados",
    cleaning: "Limpando...",

    // Risk levels
    safe: "Seguro",
    moderate: "Moderado",
    risky: "Arriscado",

    // Clean result
    cleanComplete: "Limpeza Concluída!",
    itemsCleaned: "{count} itens limpos, {size} liberados",
    itemsFailed: "{count} itens falharam",

    // Actions
    addToWhitelist: "Adicionar à whitelist",
    openInFileManager: "Abrir no gerenciador de arquivos",
    removeFromWhitelist: "Remover da whitelist",

    // Settings
    settings: "Configurações",
    scanSettings: "Configurações de Scan",
    minFileSize: "Tamanho mínimo do arquivo (MB)",
    minFileSizeDescription: "Detectar apenas arquivos maiores que este tamanho",
    findDuplicates: "Encontrar arquivos duplicados",
    findDuplicatesDescription: "Procurar arquivos duplicados (mais lento)",
    systemInfo: "Informações do Sistema",
    platform: "Plataforma:",
    home: "Home:",
    protectedPaths: "Caminhos Protegidos (Whitelist)",
    whitelistDescription: "Estes caminhos serão excluídos dos scans e limpeza.",
    pathPlaceholder: "/caminho/para/proteger",
    add: "Adicionar",
    noWhitelistPaths: "Nenhum caminho na whitelist",
    language: "Idioma",
    theme: "Tema",
    themeSystem: "Sistema",
    themeLight: "Claro",
    themeDark: "Escuro",

    // Map
    filesystemMap: "Mapa do Sistema",
    intelligentMapping: "Mapeamento inteligente de diretórios",
    rebuildMap: "Reconstruir Mapa",
    rebuilding: "Reconstruindo...",
    createMap: "Criar Mapa",
    mapping: "Mapeando...",
    noMapFound: "Nenhum mapa encontrado",
    createMapDescription: "Crie um mapa do sistema para scans mais rápidos",
    totalMapped: "Total Mapeado",
    directories: "diretórios",
    cleanable: "Limpáveis",
    lastScan: "Último Scan",
    outdated: "Desatualizado (>7 dias)",
    byCategory: "Por Categoria",
    topTypes: "Top 10 Tipos",

    // Navigation
    scanTab: "Scan",
    mapTab: "Mapa",

    // Loading
    loading: "Carregando...",

    // Version
    updateAvailable: "Atualização disponível",
    currentVersion: "Atual: v{version}",
    latestVersion: "Nova: v{version}",
    downloadUpdate: "Baixar",

    // Trash
    trashTab: "Lixeira",
    trashEmpty: "Lixeira vazia",
    trashEmptyDescription: "Itens deletados aparecerão aqui ao usar o modo --trash",
    trashItems: "{count} itens",
    trashSize: "{size} total",
    restore: "Restaurar",
    deleteForever: "Excluir Permanentemente",
    emptyTrash: "Esvaziar Lixeira",
    emptyTrashConfirm: "Excluir permanentemente todos os {count} itens?",
    restored: "Restaurado com sucesso",
    deleted: "Excluído permanentemente",
    trashEmptied: "Lixeira esvaziada",

    // Schedule
    scheduleTab: "Agendamento",
    noScheduledJobs: "Nenhum agendamento",
    noScheduledJobsDescription: "Crie agendamentos de limpeza automática",
    createJob: "Criar Agendamento",
    jobName: "Nome",
    frequency: "Frequência",
    riskLevel: "Nível de Risco",
    useTrash: "Mover para lixeira",
    secureDelete: "Exclusão segura",
    notifyOnComplete: "Notificar ao concluir",
    enabled: "Ativado",
    disabled: "Desativado",
    lastRun: "Última execução",
    never: "Nunca",
    enable: "Ativar",
    disable: "Desativar",
    remove: "Remover",
    jobCreated: "Agendamento criado",
    jobRemoved: "Agendamento removido",
    jobEnabled: "Agendamento ativado",
    jobDisabled: "Agendamento desativado",
    frequencyDaily: "Diário",
    frequencyWeekly: "Semanal",
    frequencyMonthly: "Mensal",
  },
} as const;

export type TranslationKey = keyof typeof translations["en"];
