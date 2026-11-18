import { useState } from 'react'
import { Link } from 'react-router-dom'
import {
  Activity,
  ArrowDownToLine,
  ArrowRight,
  Check,
  ChevronRight,
  Clock3,
  DatabaseBackup,
  HeartPulse,
  Plus,
  ShieldCheck,
  Swords,
  Users,
} from 'lucide-react'
import type { LucideIcon } from 'lucide-react'
import { usePreview } from '../preview'
import { number, time } from '../data'
import { Avatar, Badge, Button, PageHeading, Panel } from '../components/ui'
import { InviteDialog } from '../components/InviteDialog'

function Metric({
  icon: Icon,
  label,
  value,
  foot,
  tone = 'green',
}: {
  icon: LucideIcon
  label: string
  value: string
  foot: string
  tone?: string
}) {
  return (
    <div className="metric">
      <div className="metric-label">
        <span>{label}</span>
        <Icon size={17} />
      </div>
      <strong>{value}</strong>
      <p>
        <span className={`metric-tick ${tone}`}>
          <Check size={12} />
        </span>
        {foot}
      </p>
    </div>
  )
}

function ActivityChart({ online }: { online: number }) {
  const [range, setRange] = useState('24h')
  const heights =
    range === '24h'
      ? [3, 2, 1, 1, 2, 2, 4, 5, 3, 6, 8, 7, 10, 8, 11, 14, 12, 15, 17, 14, 11, 9, 7, 6]
      : [8, 10, 7, 14, 11, 16, 13, 17, 14, 12, 15, 18, 16, 14, 13, 19, 16, 18, 14, 11, 14, 12, 9, 6]
  heights[heights.length - 1] = online
  const points = heights.map((h, i) => `${i * 30},${158 - h * 7}`).join(' ')
  return (
    <Panel
      title="Players online"
      description="Connected players over time."
      action={
        <div className="segmented" aria-label="Chart period">
          {['24h', '7d'].map((v) => (
            <button key={v} aria-pressed={range === v} onClick={() => setRange(v)}>
              {v === '24h' ? '24 hours' : '7 days'}
            </button>
          ))}
        </div>
      }
    >
      <figure className="activity-chart">
        <div className="chart-axis">
          <span>20</span>
          <span>15</span>
          <span>10</span>
          <span>5</span>
          <span>0</span>
        </div>
        <div className="chart-drawing">
          <svg
            viewBox="0 0 690 175"
            preserveAspectRatio="none"
            role="img"
            aria-label={`Sample player activity over ${range === '24h' ? '24 hours' : '7 days'}, ending at ${online} players`}
          >
            <defs>
              <linearGradient id="chart-fill" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stopColor="var(--color-success)" stopOpacity=".23" />
                <stop offset="100%" stopColor="var(--color-success)" stopOpacity=".015" />
              </linearGradient>
            </defs>
            {[18, 53, 88, 123, 158].map((y) => (
              <line
                key={y}
                x1="0"
                y1={y}
                x2="690"
                y2={y}
                stroke="var(--color-line)"
                strokeDasharray="4 5"
              />
            ))}
            <polygon points={`0,175 ${points} 690,175`} fill="url(#chart-fill)" />
            <polyline
              points={points}
              fill="none"
              stroke="var(--color-success)"
              strokeWidth="2.5"
              vectorEffect="non-scaling-stroke"
              strokeLinejoin="round"
            />
            <circle
              cx="690"
              cy={158 - online * 7}
              r="4"
              fill="var(--color-success)"
              stroke="white"
              strokeWidth="2"
            />
          </svg>
          <div className="chart-times">
            {(range === '24h'
              ? ['00:00', '04:00', '08:00', '12:00', '16:00', '20:00', 'Now']
              : ['Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun', 'Today']
            ).map((t) => (
              <span key={t}>{t}</span>
            ))}
          </div>
        </div>
        <figcaption className="sr-only">
          Illustrative activity. These are sample values, not live server metrics.
        </figcaption>
      </figure>
      <div className="chart-footer">
        <span>
          <i className="legend-dot" />
          Players online
        </span>
        <span>
          Peak <strong>{Math.max(...heights)}</strong> <span className="footer-dot">·</span> Now{' '}
          <strong>{online}</strong>
        </span>
      </div>
    </Panel>
  )
}

export function Overview() {
  const { data } = usePreview()
  const [invite, setInvite] = useState(false)
  const [now] = useState(Date.now)
  const backupMinutes = Math.max(
    0,
    Math.round((now - new Date(data.backups[0].at).getTime()) / 60000),
  )
  const online = data.accounts.filter((a) => a.presence !== 'offline')
  const duels = Math.floor(data.accounts.filter((a) => a.presence === 'dueling').length / 2)
  return (
    <>
      <PageHeading
        title="Overview"
        description="Your community. Your server. All in one place."
        action={
          <Button variant="primary" onClick={() => setInvite(true)}>
            <Plus size={17} />
            Create invite
          </Button>
        }
      />
      <div className="server-strip">
        <div className="server-strip-icon">
          <HeartPulse size={21} />
        </div>
        <div>
          <strong>Ready for the next duel</strong>
          <span>Game server, rules engine and database are healthy.</span>
        </div>
        <Badge tone="green" dot>
          All systems operational
        </Badge>
      </div>
      <div className="metrics-grid">
        <Metric
          icon={Users}
          label="Players online"
          value={String(online.length)}
          foot={`${data.accounts.length} registered accounts`}
        />
        <Metric
          icon={Swords}
          label="Active duels"
          value={String(duels)}
          foot={`${duels} rules engines running`}
        />
        <Metric
          icon={Clock3}
          label="Server uptime"
          value="3d 14h"
          foot="Since the last restart"
          tone="neutral"
        />
        <Metric
          icon={DatabaseBackup}
          label="Latest backup"
          value={backupMinutes === 0 ? 'Just now' : `${backupMinutes} min ago`}
          foot="Database saved successfully"
        />
      </div>
      <div className="overview-charts">
        <ActivityChart online={online.length} />
        <Panel title="Server health" action={<HeartPulse size={17} className="muted" />}>
          <div className="health-list">
            {[
              {
                icon: Swords,
                title: 'Rules engine',
                value: 'Ready',
                text: 'Native engine · historical card rules',
              },
              {
                icon: ShieldCheck,
                title: 'Authentication',
                value: 'Healthy',
                text: 'Encrypted connections on port 7443',
              },
              {
                icon: DatabaseBackup,
                title: 'Database',
                value: 'Saving',
                text: 'SQLite · automatic hourly backups',
              },
            ].map(({ icon: Icon, title, value, text }) => (
              <div className="health-row" key={title}>
                <span className="health-icon">
                  <Icon size={18} />
                </span>
                <div>
                  <strong>{title}</strong>
                  <small>{text}</small>
                </div>
                <span className="health-state">
                  <span className="status-dot" />
                  {value}
                </span>
              </div>
            ))}
          </div>
          <div className="resource-box">
            <div>
              <span>Memory usage</span>
              <strong>
                412 MB <span>/ 4 GB</span>
              </strong>
            </div>
            <div className="progress-track">
              <div style={{ width: '10.3%' }} />
            </div>
            <p>Plenty of room for more duels.</p>
          </div>
          <Link className="panel-bottom-link" to="/logs">
            View server logs
            <ArrowRight size={15} />
          </Link>
        </Panel>
      </div>
      <div className="overview-lower">
        <Panel
          title="At the tables"
          description={`${online.length} players spending some time here.`}
          action={
            <Link className="text-link" to="/accounts">
              View accounts
              <ArrowRight size={14} />
            </Link>
          }
        >
          <div className="online-list">
            {online.slice(0, 5).map((a) => (
              <Link to={`/accounts?account=${a.id}`} className="online-row" key={a.id}>
                <Avatar initials={a.initials} color={a.color} />
                <div>
                  <strong>{a.player}</strong>
                  <span>{a.user}</span>
                </div>
                <Badge tone={a.presence === 'dueling' ? 'blue' : 'green'} dot>
                  {a.presence === 'dueling' ? 'In a duel' : 'Online'}
                </Badge>
                <ChevronRight size={15} className="muted" />
              </Link>
            ))}
          </div>
        </Panel>
        <Panel
          title="Recent activity"
          description="A clear record of what changed."
          action={
            <Link className="text-link" to="/audit">
              View audit log
              <ArrowRight size={14} />
            </Link>
          }
        >
          <div className="activity-list">
            {data.audit.slice(0, 4).map((a, i) => (
              <div key={a.id} className="activity-row">
                <span className={`activity-icon activity-icon-${i % 3}`}>
                  {i === 1 ? (
                    <ArrowDownToLine size={16} />
                  ) : i === 2 ? (
                    <ShieldCheck size={16} />
                  ) : (
                    <Activity size={16} />
                  )}
                </span>
                <div>
                  <strong>{a.action}</strong>
                  <p>
                    {a.actor} <span>· {a.target}</span>
                  </p>
                </div>
                <time dateTime={a.at}>{time(a.at)}</time>
              </div>
            ))}
          </div>
          <div className="activity-summary">
            <ShieldCheck size={15} />
            Every staff action leaves a trace.
          </div>
        </Panel>
      </div>
      <div className="overview-note">
        <span>
          Sample server version <strong>0.0.21</strong>
        </span>
        <Link to="/settings">
          Manage server settings
          <ArrowRight size={14} />
        </Link>
        <span className="sr-only">{number(data.accounts.length)} accounts</span>
      </div>
      <InviteDialog open={invite} onOpenChange={setInvite} />
    </>
  )
}
