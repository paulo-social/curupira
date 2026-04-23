import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { AlertsList } from './AlertsList'

describe('AlertsList', () => {
  it('shows an empty-state message when there are no alerts', () => {
    render(<AlertsList alerts={[]} />)

    expect(screen.getByText('Nenhum alerta crítico registrado ainda.')).toBeInTheDocument()
  })

  it('renders alert rows with type and rounded confidence', () => {
    render(
      <AlertsList
        alerts={[
          {
            id: 1,
            tipo_evento: 'motosserra',
            timestamp: '2026-04-15T10:00:00Z',
            confianca: 93.7
          }
        ]}
      />
    )

    expect(screen.getByText('motosserra')).toBeInTheDocument()
    expect(screen.getByText('94%')).toBeInTheDocument()
  })
})
