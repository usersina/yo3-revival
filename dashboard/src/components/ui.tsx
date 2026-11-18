import * as Dialog from '@radix-ui/react-dialog'
import {
  Check,
  CheckCircle2,
  Clipboard,
  Info,
  Search,
  SearchX,
  TriangleAlert,
  X,
} from 'lucide-react'
import type { ComponentProps, ReactNode } from 'react'
import { useRef } from 'react'
import { usePreview } from '../preview'

export function Button({
  variant = 'secondary',
  className = '',
  ...props
}: ComponentProps<'button'> & { variant?: 'primary' | 'secondary' | 'ghost' | 'danger' }) {
  return <button type="button" className={`button button-${variant} ${className}`} {...props} />
}

export function Badge({
  children,
  tone = 'neutral',
  dot = false,
}: {
  children: ReactNode
  tone?: 'neutral' | 'green' | 'blue' | 'amber' | 'red' | 'violet'
  dot?: boolean
}) {
  return (
    <span className={`badge badge-${tone}`}>
      {dot && <span className="status-dot" />}
      {children}
    </span>
  )
}

export function PageHeading({
  title,
  description,
  action,
}: {
  title: string
  description: string
  action?: ReactNode
}) {
  return (
    <div className="page-heading">
      <div>
        <h1>{title}</h1>
        <p>{description}</p>
      </div>
      {action && <div className="page-actions">{action}</div>}
    </div>
  )
}

export function Panel({
  title,
  description,
  action,
  children,
  className = '',
}: {
  title?: string
  description?: string
  action?: ReactNode
  children: ReactNode
  className?: string
}) {
  return (
    <section className={`panel ${className}`}>
      {title && (
        <div className="panel-heading">
          <div>
            <h2>{title}</h2>
            {description && <p>{description}</p>}
          </div>
          {action}
        </div>
      )}
      {children}
    </section>
  )
}

export function Modal({
  open,
  onOpenChange,
  title,
  description,
  children,
  wide = false,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  title: string
  description: string
  children: ReactNode
  wide?: boolean
}) {
  const previousFocus = useRef<HTMLElement | null>(null)
  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="modal-overlay" />
        <Dialog.Content
          className={`modal ${wide ? 'modal-wide' : ''}`}
          onOpenAutoFocus={() => {
            previousFocus.current =
              document.activeElement instanceof HTMLElement ? document.activeElement : null
          }}
          onCloseAutoFocus={(event) => {
            event.preventDefault()
            previousFocus.current?.focus()
          }}
        >
          <div className="modal-heading">
            <div>
              <Dialog.Title>{title}</Dialog.Title>
              <Dialog.Description>{description}</Dialog.Description>
            </div>
            <Dialog.Close asChild>
              <Button variant="ghost" className="icon-button" aria-label="Close dialog">
                <X size={19} />
              </Button>
            </Dialog.Close>
          </div>
          {children}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  )
}

export function Field({
  label,
  id,
  hint,
  children,
}: {
  label: string
  id: string
  hint?: string
  children: ReactNode
}) {
  return (
    <div className="field">
      <label htmlFor={id}>{label}</label>
      {children}
      {hint && (
        <p className="field-hint" id={`${id}-hint`}>
          {hint}
        </p>
      )}
    </div>
  )
}

export function SearchField({
  value,
  onChange,
  label,
  placeholder,
  maxLength,
}: {
  value: string
  onChange: (value: string) => void
  label: string
  maxLength?: number
  placeholder: string
}) {
  const input = useRef<HTMLInputElement>(null)
  return (
    <div className="search-field">
      <Search size={17} />
      <input
        ref={input}
        type="search"
        maxLength={maxLength}
        aria-label={label}
        placeholder={placeholder}
        value={value}
        onChange={(e) => onChange(e.target.value)}
      />
      {value && (
        <button
          type="button"
          className="clear-search"
          aria-label="Clear search"
          onClick={() => {
            onChange('')
            input.current?.focus()
          }}
        >
          <X size={15} />
        </button>
      )}
    </div>
  )
}

export function EmptyState({
  title = 'No results found',
  description,
  action,
}: {
  title?: string
  description: string
  action?: ReactNode
}) {
  return (
    <div className="empty-state">
      <span className="empty-icon">
        <SearchX size={25} />
      </span>
      <h3>{title}</h3>
      <p>{description}</p>
      {action}
    </div>
  )
}

export function Avatar({
  initials,
  color = 'blue',
  small = false,
}: {
  initials: string
  color?: string
  small?: boolean
}) {
  return (
    <span aria-hidden="true" className={`avatar avatar-${color} ${small ? 'avatar-small' : ''}`}>
      {initials}
    </span>
  )
}

export function CopyButton({ value, label = 'Copy code' }: { value: string; label?: string }) {
  const { notify } = usePreview()
  async function copy() {
    try {
      await navigator.clipboard.writeText(value)
      notify('Copied to clipboard.')
    } catch {
      notify('Could not copy. Select the code and copy it manually.', 'error')
    }
  }
  return (
    <Button variant="ghost" className="icon-button" onClick={copy} aria-label={label}>
      <Clipboard size={16} />
    </Button>
  )
}

export function Notice() {
  const { notice, dismiss } = usePreview()
  if (!notice) return null
  const Icon =
    notice.tone === 'error' ? TriangleAlert : notice.tone === 'info' ? Info : CheckCircle2
  return (
    <div
      className={`toast toast-${notice.tone}`}
      role={notice.tone === 'error' ? 'alert' : 'status'}
      aria-atomic="true"
    >
      <Icon size={19} />
      <span>{notice.message}</span>
      <Button
        variant="ghost"
        className="icon-button"
        onClick={dismiss}
        aria-label="Dismiss notification"
      >
        <X size={16} />
      </Button>
    </div>
  )
}

export function ReadOnlyNote({ children }: { children: ReactNode }) {
  return (
    <div className="inline-notice">
      <Info size={17} />
      <span>{children}</span>
    </div>
  )
}

export function SavedMark() {
  return (
    <span className="saved-mark">
      <Check size={14} /> Applies without a restart
    </span>
  )
}
