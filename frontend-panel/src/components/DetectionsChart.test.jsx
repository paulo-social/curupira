import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { DetectionsChart } from './DetectionsChart'

vi.mock('recharts', () => ({
  ResponsiveContainer: ({ children }) => <div data-testid="responsive-container">{children}</div>,
  BarChart: ({ children }) => <div data-testid="bar-chart">{children}</div>,
  CartesianGrid: () => <div data-testid="cartesian-grid" />,
  XAxis: () => <div data-testid="x-axis" />,
  YAxis: () => <div data-testid="y-axis" />,
  Tooltip: () => <div data-testid="tooltip" />,
  Bar: () => <div data-testid="bar" />
}))

describe('DetectionsChart', () => {
  it('renders the chart shell and title', () => {
    render(<DetectionsChart data={[{ hour: '08h', count: 2 }]} />)

    expect(screen.getByText('Ocorrências por hora')).toBeInTheDocument()
    expect(screen.getByTestId('bar-chart')).toBeInTheDocument()
    expect(screen.getByTestId('bar')).toBeInTheDocument()
  })
})
