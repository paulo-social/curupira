import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { StatusCard } from './StatusCard'

describe('StatusCard', () => {
  it('renders online state and last sync', () => {
    render(<StatusCard online lastSync="14:32:10" />)

    expect(screen.getByText('Backend acessível')).toBeInTheDocument()
    expect(screen.getByText('Última sincronização às 14:32:10')).toBeInTheDocument()
  })

  it('renders waiting state when no sync happened yet', () => {
    render(<StatusCard online={false} lastSync="" />)

    expect(screen.getByText('Sem resposta do backend')).toBeInTheDocument()
    expect(screen.getByText('Aguardando primeira atualização')).toBeInTheDocument()
  })
})
