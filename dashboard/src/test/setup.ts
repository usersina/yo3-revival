import '@testing-library/jest-dom/vitest'
import { cleanup } from '@testing-library/react'
import { afterEach, vi } from 'vitest'

// jsdom has no scrolling implementation; the browser suite verifies real navigation scroll.
window.scrollTo = vi.fn()

afterEach(cleanup)
