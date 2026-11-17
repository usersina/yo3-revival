export type Role = 'admin' | 'moderator'
export type Presence = 'dueling' | 'online' | 'offline'
export type Standing = 'active' | 'muted' | 'banned' | 'restricted'

export interface Account {
  id: number
  user: string
  player: string
  initials: string
  color: string
  country: string
  role: Role | 'player'
  presence: Presence
  standing: Standing
  bp: number
  wins: number
  losses: number
  joined: string
}
export interface Invite {
  code: string
  uses: number
  limit: number
  expires: string
  by: string
}
export interface Reward {
  code: string
  bp: number
  claims: number
  limit: number
}
export interface AuditEntry {
  id: string
  at: string
  actor: string
  action: string
  target: string
  source: string
}
export interface LogEntry {
  id: string
  at: string
  level: 'info' | 'warning' | 'error'
  module: string
  message: string
}
export interface Backup {
  id: string
  at: string
  size: string
  kind: 'Automatic' | 'Manual'
  preview?: boolean
}
export interface News {
  id: string
  title: string
  body: string
  at: string
}
export interface Settings {
  name: string
  headline: string
  registration: 'invite' | 'open' | 'closed'
  lp: number
  turn: number
  total: number
  regulation: string
}
export interface DemoState {
  accounts: Account[]
  invites: Invite[]
  rewards: Reward[]
  audit: AuditEntry[]
  logs: LogEntry[]
  backups: Backup[]
  news: News[]
  settings: Settings
  maintenance: { at: string; message: string } | null
}

export const number = new Intl.NumberFormat('en-US').format
export const time = (at: string) =>
  new Intl.DateTimeFormat('en-GB', { hour: '2-digit', minute: '2-digit' }).format(new Date(at))
export const date = (at: string) =>
  new Intl.DateTimeFormat('en-GB', { day: 'numeric', month: 'short', year: 'numeric' }).format(
    new Date(at),
  )
export const timestamp = (at: string) => `${date(at)}, ${time(at)}`
export const id = () => crypto.randomUUID()

export function createDemoState(): DemoState {
  const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString()
  const days = (n: number) => new Date(Date.now() + n * 86_400_000).toISOString()
  return {
    accounts: [
      {
        id: 1,
        user: 'YuseiFudo',
        player: 'Yusei',
        initials: 'YF',
        color: 'blue',
        country: 'Japan',
        role: 'player',
        presence: 'dueling',
        standing: 'active',
        bp: 4200,
        wins: 24,
        losses: 8,
        joined: days(-14),
      },
      {
        id: 2,
        user: 'SetoKaiba',
        player: 'Kaiba',
        initials: 'SK',
        color: 'violet',
        country: 'Germany',
        role: 'player',
        presence: 'dueling',
        standing: 'active',
        bp: 8700,
        wins: 38,
        losses: 12,
        joined: days(-12),
      },
      {
        id: 3,
        user: 'AkizaIzinski',
        player: 'Akiza',
        initials: 'AI',
        color: 'rose',
        country: 'France',
        role: 'player',
        presence: 'online',
        standing: 'active',
        bp: 2650,
        wins: 17,
        losses: 9,
        joined: days(-10),
      },
      {
        id: 4,
        user: 'JadenYuki',
        player: 'Jaden',
        initials: 'JY',
        color: 'amber',
        country: 'Tunisia',
        role: 'player',
        presence: 'dueling',
        standing: 'active',
        bp: 3100,
        wins: 21,
        losses: 14,
        joined: days(-9),
      },
      {
        id: 5,
        user: 'ZaneTruesdale',
        player: 'Zane',
        initials: 'ZT',
        color: 'cyan',
        country: 'Belgium',
        role: 'player',
        presence: 'dueling',
        standing: 'active',
        bp: 6500,
        wins: 29,
        losses: 11,
        joined: days(-8),
      },
      {
        id: 6,
        user: 'TestAdmin',
        player: 'Admin',
        initials: 'TA',
        color: 'green',
        country: 'Tunisia',
        role: 'admin',
        presence: 'online',
        standing: 'active',
        bp: 1200,
        wins: 6,
        losses: 3,
        joined: days(-18),
      },
      {
        id: 7,
        user: 'CrowHogan',
        player: 'Crow',
        initials: 'CH',
        color: 'amber',
        country: 'Italy',
        role: 'player',
        presence: 'offline',
        standing: 'active',
        bp: 1800,
        wins: 14,
        losses: 8,
        joined: days(-5),
      },
      {
        id: 8,
        user: 'JackAtlas',
        player: 'Jack',
        initials: 'JA',
        color: 'violet',
        country: 'United Kingdom',
        role: 'player',
        presence: 'offline',
        standing: 'muted',
        bp: 5400,
        wins: 19,
        losses: 15,
        joined: days(-6),
      },
      {
        id: 9,
        user: 'TestModerator',
        player: 'Moderator',
        initials: 'TM',
        color: 'blue',
        country: 'France',
        role: 'moderator',
        presence: 'offline',
        standing: 'active',
        bp: 900,
        wins: 4,
        losses: 2,
        joined: days(-15),
      },
      {
        id: 10,
        user: 'Duelist010',
        player: 'Duelist',
        initials: 'D0',
        color: 'slate',
        country: 'United States',
        role: 'player',
        presence: 'offline',
        standing: 'banned',
        bp: 500,
        wins: 2,
        losses: 6,
        joined: days(-3),
      },
    ],
    invites: [
      { code: 'YO3-FRIENDS-7K2P', uses: 2, limit: 5, expires: days(6), by: 'TestAdmin' },
      { code: 'YO3-WEEKEND-9M4R', uses: 10, limit: 10, expires: days(2), by: 'TestModerator' },
      { code: 'YO3-EARLY-3H8N', uses: 1, limit: 1, expires: days(-2), by: 'TestAdmin' },
    ],
    rewards: [
      { code: 'WELCOME-BACK', bp: 500, claims: 7, limit: 100 },
      { code: 'WEEKEND-DUELS', bp: 1000, claims: 4, limit: 20 },
    ],
    audit: [
      {
        id: 'a1',
        at: ago(3),
        actor: 'TestAdmin',
        action: 'Created an invite',
        target: 'YO3-FRIENDS-7K2P',
        source: 'Dashboard',
      },
      {
        id: 'a2',
        at: ago(12),
        actor: 'System',
        action: 'Created an automatic backup',
        target: 'server.db',
        source: 'Server',
      },
      {
        id: 'a3',
        at: ago(28),
        actor: 'TestModerator',
        action: 'Muted JackAtlas',
        target: 'JackAtlas',
        source: 'Dashboard',
      },
      {
        id: 'a4',
        at: ago(45),
        actor: 'TestAdmin',
        action: 'Updated the server headline',
        target: 'Settings',
        source: 'CLI',
      },
      {
        id: 'a5',
        at: ago(71),
        actor: 'TestAdmin',
        action: 'Added 500 BP to CrowHogan',
        target: 'CrowHogan',
        source: 'Dashboard',
      },
      {
        id: 'a6',
        at: ago(92),
        actor: 'TestAdmin',
        action: 'Banned Duelist010',
        target: 'Duelist010',
        source: 'Dashboard',
      },
    ],
    logs: [
      {
        id: 'l1',
        at: ago(0),
        level: 'info',
        module: 'duel',
        message: 'Duel started: Yusei vs Kaiba. Rules engine ready.',
      },
      {
        id: 'l2',
        at: ago(1),
        level: 'info',
        module: 'accounts',
        message: 'AkizaIzinski signed in. Player: Akiza.',
      },
      {
        id: 'l3',
        at: ago(5),
        level: 'warning',
        module: 'auth',
        message: 'Sign-in throttled after repeated failed attempts. Retry in 30 seconds.',
      },
      {
        id: 'l4',
        at: ago(12),
        level: 'info',
        module: 'store',
        message: 'Database backup completed. 24 snapshots retained.',
      },
      {
        id: 'l5',
        at: ago(18),
        level: 'info',
        module: 'duel',
        message: 'Duel ended: Jaden vs Zane. Result saved.',
      },
      {
        id: 'l6',
        at: ago(24),
        level: 'info',
        module: 'settings',
        message: 'Live settings applied without a restart.',
      },
      {
        id: 'l7',
        at: ago(36),
        level: 'warning',
        module: 'transport',
        message: 'Connection closed before authentication completed.',
      },
    ],
    backups: [12, 72, 132, 192, 252].map((n, i) => ({
      id: `backup-${i}`,
      at: ago(n),
      size: `${(8.4 - i * 0.1).toFixed(1)} MB`,
      kind: 'Automatic',
    })),
    news: [
      {
        id: 'n1',
        title: 'Welcome back to Duel Accelerator',
        body: 'The duel disk is ready. Build a deck, meet the others, and make yourself at home.',
        at: days(-2),
      },
      {
        id: 'n2',
        title: 'A small reminder',
        body: 'Found a card that behaves strangely? Export your logs and open an issue on GitHub so we can take a look.',
        at: days(-5),
      },
    ],
    settings: {
      name: 'YO3 Revival',
      headline: 'Good cards. Old friends. One more duel.',
      registration: 'invite',
      lp: 8000,
      turn: 180,
      total: 1800,
      regulation: 'Season 16',
    },
    maintenance: null,
  }
}
