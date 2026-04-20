export const t = {
  topbar: {
    selectFolder: '未选择文件夹',
    claudeStatus: 'Claude',
    network: '网络',
    commands: '命令',
    skills: 'Skills',
    settings: '设置',
    updateAvailable: '有新版本可用',
  },
  history: {
    newSession: '新会话',
    continueLast: '继续上次',
    searchPlaceholder: '搜索历史...',
    emptyTitle: '选个文件夹开始对话',
    emptySubtitle: '历史会话会出现在这里',
  },
  terminal: {
    placeholderTitle: '$ claude',
    placeholderSubtitle: '终端区将在 M2 接入 xterm.js + tauri-plugin-pty',
    exitedTitle: 'Claude 已退出',
    restart: '点击重启',
  },
} as const;
