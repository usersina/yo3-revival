import { SelectControl, SelectOption } from './components/form-controls'
import { Moon, Sun } from 'lucide-react'
import { Button, Field, Panel } from './components/ui'
import { useTheme } from './theme'

export function ThemeToggle() {
  const { theme, setPreference } = useTheme()
  const label = `Switch to ${theme === 'dark' ? 'light' : 'dark'} mode`
  return (
    <Button
      variant="ghost"
      className="theme-toggle"
      aria-label={label}
      title={label}
      onClick={() => setPreference(theme === 'dark' ? 'light' : 'dark')}
    >
      {theme === 'dark' ? <Sun size={18} /> : <Moon size={18} />}
      <span>{theme === 'dark' ? 'Dark' : 'Light'}</span>
    </Button>
  )
}

export function AppearanceSettings() {
  const { preference, setPreference } = useTheme()
  return (
    <Panel
      title="Appearance"
      description="Choose the dashboard’s appearance on this browser."
      className="appearance-panel"
    >
      <Field
        id="dashboard-theme"
        label="Color theme"
        hint={
          preference === 'system'
            ? 'Follows your device’s appearance.'
            : `Always use ${preference} mode on this browser.`
        }
      >
        <SelectControl
          id="dashboard-theme"
          value={preference}
          onValueChange={(value) => {
            if (value === 'light' || value === 'dark' || value === 'system') setPreference(value)
          }}
        >
          <SelectOption value="system">System</SelectOption>
          <SelectOption value="light">Light</SelectOption>
          <SelectOption value="dark">Dark</SelectOption>
        </SelectControl>
      </Field>
    </Panel>
  )
}
