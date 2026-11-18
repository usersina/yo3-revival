import { useState } from 'react'
import { ImageOff, Layers } from 'lucide-react'
import type { CardChoice } from './api/generated/http/models'
import { Modal } from './components/ui'

export function CardThumbnail({ url }: { url: string | null | undefined }) {
  const [failed, setFailed] = useState(false)
  return (
    <span className="gift-symbol" aria-hidden="true">
      {url && !failed ? (
        <img
          src={url}
          alt=""
          loading="lazy"
          decoding="async"
          width={50}
          height={72}
          onError={() => setFailed(true)}
        />
      ) : (
        <Layers size={21} />
      )}
    </span>
  )
}

export function CardPreview({ card, onClose }: { card: CardChoice; onClose: () => void }) {
  const [failed, setFailed] = useState(false)
  const [loaded, setLoaded] = useState(false)
  return (
    <Modal
      open
      onOpenChange={(open) => {
        if (!open) onClose()
      }}
      title={card.name}
      description={`Artwork ${card.art} · Konami ${card.konami_id}`}
      wide
    >
      <div className="modal-body card-preview">
        <div className="card-preview-art" aria-busy={!!card.image_url && !loaded && !failed}>
          {card.image_url && !failed ? (
            <>
              {!loaded && <span role="status">Loading card…</span>}
              <img
                src={card.image_url}
                alt={`${card.name}, artwork ${card.art}`}
                width={200}
                height={290}
                decoding="async"
                onLoad={() => setLoaded(true)}
                onError={() => setFailed(true)}
              />
            </>
          ) : (
            <div className="card-preview-placeholder">
              <ImageOff size={32} aria-hidden="true" />
              <p>Image unavailable</p>
              <small>
                The host may need to add the original game’s image archives. You can still give this
                card.
              </small>
            </div>
          )}
        </div>
        <div className="card-preview-text">
          <h3>Original game text</h3>
          <p>{card.description || 'No description is available in the host’s game files.'}</p>
        </div>
      </div>
    </Modal>
  )
}
