import { lazy, Suspense } from 'react'
import { BrowserRouter } from 'react-router-dom'
import ConnectedApp from './ConnectedApp'

// Sample pages are an explicit development surface, eliminated from production builds.
const DesignPreview = import.meta.env.DEV ? lazy(() => import('./App')) : null
export default function DashboardRoot() {
  const design =
    DesignPreview &&
    (window.location.pathname === '/design' || window.location.pathname.startsWith('/design/'))
  return (
    <BrowserRouter basename={design ? '/design' : undefined}>
      {design ? (
        <Suspense fallback={<p role="status">Loading design preview…</p>}>
          <DesignPreview />
        </Suspense>
      ) : (
        <ConnectedApp />
      )}
    </BrowserRouter>
  )
}
