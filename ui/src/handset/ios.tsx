import type { ReactNode } from 'react'

/**
 * iOS's grouped lists (owner 2026-11-03: "everything as on the iPhone"), as its Settings has them: a header over
 * a rounded group of rows, a footer under it.
 */
export function List({ header, footer, children }: { header?: string; footer?: string; children: ReactNode }) {
  return (
    <section className="ios-section">
      {header && <h2 className="ios-section-header">{header}</h2>}
      <div className="ios-list">{children}</div>
      {footer && <p className="ios-section-footer">{footer}</p>}
    </section>
  )
}

/** A row of a list: its icon and name on the left, its value (or what it holds) on the right; a chevron where it
 * opens something. */
export function Row({
  label,
  detail,
  value,
  icon,
  late = false,
  onClick,
  children,
}: {
  label: ReactNode
  detail?: ReactNode
  value?: ReactNode
  icon?: ReactNode
  late?: boolean
  onClick?: () => void
  children?: ReactNode
}) {
  const inside = (
    <>
      {icon}
      <span className="ios-row-text">
        <span className="ios-row-label">{label}</span>
        {detail && <span className="ios-row-detail">{detail}</span>}
      </span>
      {value !== undefined && <span className={`ios-row-value${late ? ' late' : ''}`}>{value}</span>}
      {children}
      {onClick && (
        <svg className="ios-chevron" viewBox="0 0 8 14" aria-hidden>
          <path d="m1 1 6 6-6 6" />
        </svg>
      )}
    </>
  )
  return onClick ? (
    <button type="button" className="ios-row ios-row-button" onClick={onClick}>
      {inside}
    </button>
  ) : (
    <div className="ios-row">{inside}</div>
  )
}

/** A choice of a few, as iOS's segmented control. */
export function Segmented<T extends string>({
  options,
  chosen,
  choose,
}: {
  options: [T, string][]
  chosen: T
  choose: (value: T) => void
}) {
  return (
    <div className="ios-segmented" role="radiogroup">
      {options.map(([value, label]) => (
        <button
          key={value}
          type="button"
          role="radio"
          aria-checked={value === chosen}
          className={value === chosen ? 'on' : undefined}
          onClick={() => choose(value)}
        >
          {label}
        </button>
      ))}
    </div>
  )
}
