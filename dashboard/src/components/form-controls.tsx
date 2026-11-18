import { useContext, useId, useState } from 'react'
import type { ComponentProps, ReactNode } from 'react'
import { CircleAlert } from 'lucide-react'
import { FieldContext } from './field-context'
import { Input as InputPrimitive } from './ui/input'
import { Textarea as TextareaPrimitive } from './ui/textarea'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from './ui/select'

// Radix reserves an empty value for placeholders. Our registry also uses it as a real choice.
const emptyChoice = '__yo3_empty_choice__'

export function SelectOption({
  value,
  ...props
}: Omit<ComponentProps<typeof SelectItem>, 'value'> & { value: string | number }) {
  return <SelectItem value={String(value) || emptyChoice} data-value={value} {...props} />
}

export function SelectControl({
  value,
  onValueChange,
  children,
  disabled,
  name,
  required,
  ...props
}: Omit<ComponentProps<typeof SelectTrigger>, 'value' | 'onChange'> & {
  value: string | number
  onValueChange: (value: string) => void
  children: ReactNode
  name?: string
  required?: boolean
}) {
  const field = useContext(FieldContext)
  return (
    <Select
      value={String(value) || emptyChoice}
      onValueChange={(next) => onValueChange(next === emptyChoice ? '' : next)}
      disabled={disabled}
      name={name}
      required={required}
    >
      <SelectTrigger
        {...props}
        disabled={disabled}
        className={`select-control ${props.className ?? ''}`}
        data-value={value}
        aria-describedby={
          props['aria-describedby'] ?? (field.id === props.id ? field.hintId : undefined)
        }
      >
        <SelectValue />
      </SelectTrigger>
      <SelectContent sideOffset={4} collisionPadding={12} className="select-popup">
        {children}
      </SelectContent>
    </Select>
  )
}

function validationMessage(input: HTMLInputElement | HTMLTextAreaElement) {
  const validity = input.validity
  if (validity.valid) return ''
  if (validity.valueMissing) return 'Enter a value.'
  if (validity.badInput) return 'Enter a valid number.'
  if (validity.rangeUnderflow) return `Enter ${input.getAttribute('min')} or more.`
  if (validity.rangeOverflow) return `Enter ${input.getAttribute('max')} or less.`
  if (validity.stepMismatch && input.getAttribute('step') === '1') return 'Enter a whole number.'
  if (validity.tooShort) return `Enter at least ${input.minLength} characters.`
  if (validity.tooLong) return `Enter no more than ${input.maxLength} characters.`
  return input.validationMessage
}

function useValidation(props: ComponentProps<'input'> | ComponentProps<'textarea'>) {
  const field = useContext(FieldContext)
  const errorId = useId()
  const [checked, setChecked] = useState<{ value: string; message: string } | null>(null)
  // A parent can reset a form or replace a news entry without firing an input event.
  const error =
    checked && (props.value === undefined || String(props.value) === checked.value)
      ? checked.message
      : ''
  const [touched, setTouched] = useState(false)
  const descriptions = [
    props['aria-describedby'],
    field.id === props.id ? field.hintId : undefined,
    error ? errorId : undefined,
  ]
    .filter(Boolean)
    .join(' ')
  const describedBy = [...new Set(descriptions.split(/\s+/).filter(Boolean))].join(' ') || undefined
  return {
    error,
    errorId,
    describedBy,
    touched,
    setTouched,
    validate: (input: HTMLInputElement | HTMLTextAreaElement) =>
      setChecked({
        value: input.value,
        message: input.disabled || input.readOnly ? '' : validationMessage(input),
      }),
  }
}

function focusFirstInvalid(input: HTMLInputElement | HTMLTextAreaElement) {
  const first = input.form?.querySelector('input:invalid, textarea:invalid, select:invalid')
  if (!first || first === input) input.focus()
}

function InputError({ id, children }: { id: string; children: ReactNode }) {
  return (
    <p id={id} className="field-error input-error" role="status">
      <CircleAlert size={14} aria-hidden="true" />
      {children}
    </p>
  )
}

export function Input({
  onChange,
  onBlur,
  onInvalid,
  ...props
}: ComponentProps<typeof InputPrimitive>) {
  const validation = useValidation(props)
  return (
    <>
      <InputPrimitive
        {...props}
        aria-invalid={props['aria-invalid'] || Boolean(validation.error)}
        aria-describedby={validation.describedBy}
        onChange={(event) => {
          onChange?.(event)
          if (validation.touched) validation.validate(event.currentTarget)
        }}
        onBlur={(event) => {
          onBlur?.(event)
          validation.setTouched(true)
          validation.validate(event.currentTarget)
        }}
        onInvalid={(event) => {
          onInvalid?.(event)
          event.preventDefault()
          focusFirstInvalid(event.currentTarget)
          validation.setTouched(true)
          validation.validate(event.currentTarget)
        }}
      />
      {validation.error && <InputError id={validation.errorId}>{validation.error}</InputError>}
    </>
  )
}

export function Textarea({
  onChange,
  onBlur,
  onInvalid,
  ...props
}: ComponentProps<typeof TextareaPrimitive>) {
  const validation = useValidation(props)
  return (
    <>
      <TextareaPrimitive
        {...props}
        aria-invalid={props['aria-invalid'] || Boolean(validation.error)}
        aria-describedby={validation.describedBy}
        onChange={(event) => {
          onChange?.(event)
          if (validation.touched) validation.validate(event.currentTarget)
        }}
        onBlur={(event) => {
          onBlur?.(event)
          validation.setTouched(true)
          validation.validate(event.currentTarget)
        }}
        onInvalid={(event) => {
          onInvalid?.(event)
          event.preventDefault()
          focusFirstInvalid(event.currentTarget)
          validation.setTouched(true)
          validation.validate(event.currentTarget)
        }}
      />
      {validation.error && <InputError id={validation.errorId}>{validation.error}</InputError>}
    </>
  )
}
