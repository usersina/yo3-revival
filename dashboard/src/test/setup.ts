import '@testing-library/jest-dom/vitest'
import { cleanup } from '@testing-library/react'
import { afterEach, vi } from 'vitest'

// jsdom has no scrolling implementation; the browser suite verifies real navigation scroll.
window.scrollTo = vi.fn()
// Radix selects use these browser APIs; popup geometry is covered in Playwright.
HTMLElement.prototype.scrollIntoView = vi.fn()
HTMLElement.prototype.hasPointerCapture = () => false
HTMLElement.prototype.setPointerCapture = vi.fn()
HTMLElement.prototype.releasePointerCapture = vi.fn()

afterEach(cleanup)
