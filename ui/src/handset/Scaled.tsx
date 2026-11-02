import { type ReactNode, useLayoutEffect, useRef, useState } from 'react'

/**
 * The cab's own screens (its map, its music apps) in the phone: laid out `1 / scale` times the room the phone
 * gives them, and drawn `scale` times that, so their buttons and words, made for the cab's wide screen, come out
 * the phone's size (owner 2026-11-02: "the map's icons are too big").
 */
export function Scaled({ scale, className, children }: { scale: number; className?: string; children: ReactNode }) {
  const room = useRef<HTMLDivElement>(null)
  const [size, setSize] = useState<{ width: number; height: number } | null>(null)
  useLayoutEffect(() => {
    const element = room.current
    if (!element) return
    const measure = () => setSize({ width: element.clientWidth, height: element.clientHeight })
    measure()
    const observer = new ResizeObserver(measure)
    observer.observe(element)
    return () => observer.disconnect()
  }, [])
  return (
    <div ref={room} className={`handset-scaled${className ? ` ${className}` : ''}`}>
      {size && (
        <div
          className="handset-scaled-page"
          style={{ width: size.width / scale, height: size.height / scale, transform: `scale(${scale})` }}
        >
          {children}
        </div>
      )}
    </div>
  )
}
