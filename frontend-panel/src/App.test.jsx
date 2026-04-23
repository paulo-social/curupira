import { describe, expect, it } from 'vitest'
import { buildChartData, isCriticalAlert } from './App'

describe('App helpers', () => {
  it('marks only alerts above 80 percent as critical', () => {
    expect(isCriticalAlert({ confianca: 81 })).toBe(true)
    expect(isCriticalAlert({ confianca: 80 })).toBe(false)
    expect(isCriticalAlert(null)).toBe(false)
  })

  it('groups detections by hour and sorts the result', () => {
    const data = buildChartData([
      { timestamp: '2026-04-15T11:10:00Z' },
      { timestamp: '2026-04-15T10:05:00Z' },
      { timestamp: '2026-04-15T11:45:00Z' }
    ])

    expect(data).toEqual([
      { hour: '07h', count: 1 },
      { hour: '08h', count: 2 }
    ])
  })
})
